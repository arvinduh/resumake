//! Release orchestration, pre-flight repository and semver verification, and
//! tag management.

use std::path::Path;

use semver::Version;

use crate::engine::templates::DEFAULT_TEMPLATE;
use crate::engine::{self};
use crate::schema;
use crate::utils::git;

/// Errors originating from release pipeline and semver verification.
#[derive(Debug, thiserror::Error)]
pub enum Error {
  /// Version string was empty.
  #[error("Version string cannot be empty")]
  EmptyVersion,
  /// Invalid semantic version string.
  #[error("Invalid semver '{version}': {source}")]
  InvalidSemver {
    /// Provided version string.
    version: String,
    /// Underlying semver error.
    #[source]
    source: semver::Error,
  },
  /// Proposed version is not strictly newer than existing release tags.
  #[error(
    "Version v{target} is not strictly newer than existing tag v{latest} \
     (semver monotonicity check failed)."
  )]
  NonMonotonicSemver {
    /// Proposed version.
    target: Version,
    /// Highest existing tag version.
    latest: Version,
  },
  /// Git operation error.
  #[error(transparent)]
  Git(#[from] git::Error),
  /// Schema inspection error.
  #[error(transparent)]
  Schema(#[from] schema::Error),
  /// Engine compilation or verification error.
  #[error(transparent)]
  Engine(#[from] engine::Error),
  /// Underlying I/O error.
  #[error("I/O error: {0}")]
  Io(#[from] std::io::Error),
}

/// Parses a version string into a [`Version`].
///
/// Strips leading `v` or `V` before parsing.
///
/// # Errors
/// Returns [`Error::EmptyVersion`] if the string is empty or
/// [`Error::InvalidSemver`] if not valid semver.
pub fn parse_version(s: &str) -> Result<Version, Error> {
  let s_trimmed = s.trim();
  let without_v = s_trimmed
    .strip_prefix('v')
    .or_else(|| s_trimmed.strip_prefix('V'))
    .unwrap_or(s_trimmed);
  if without_v.is_empty() {
    return Err(Error::EmptyVersion);
  }

  Version::parse(without_v).map_err(|source| Error::InvalidSemver {
    version: s.to_string(),
    source,
  })
}

/// Derives the `GitHub` Actions URL from a git remote URL.
#[must_use]
pub fn derive_actions_url(remote_url: &str) -> String {
  let trimmed = remote_url.trim();
  let stripped = trimmed.strip_suffix(".git").unwrap_or(trimmed);

  if let Some(rest) = stripped.strip_prefix("git@github.com:") {
    format!("https://github.com/{rest}/actions")
  } else if let Some(rest) = stripped.strip_prefix("ssh://git@github.com/") {
    format!("https://github.com/{rest}/actions")
  } else if stripped.starts_with("http://") || stripped.starts_with("https://")
  {
    format!("{stripped}/actions")
  } else {
    format!("https://github.com/{stripped}/actions")
  }
}

/// Validates that `target_ver` is strictly newer than any existing git semver
/// tag.
///
/// # Errors
/// Returns [`Error::NonMonotonicSemver`] if `target_ver` <= highest tag, or
/// [`Error::Git`] if tag inspection fails.
pub fn check_semver_monotonicity(
  target_ver: &Version,
  repo_dir: &Path,
) -> Result<Option<Version>, Error> {
  let latest_tag = git::get_latest_semver_tag(repo_dir)?;
  if let Some(ref latest) = latest_tag {
    if target_ver <= latest {
      return Err(Error::NonMonotonicSemver {
        target: target_ver.clone(),
        latest: latest.clone(),
      });
    }
  }
  Ok(latest_tag)
}

/// Pre-flight release verification outcome.
#[derive(Debug, Clone)]
pub struct PreflightOutcome {
  /// Target version read from content file.
  pub target_version: Version,
  /// Previously released tag version, if any.
  pub latest_tag: Option<Version>,
}

/// Executes all pre-flight release checks: clean working tree, upstream sync,
/// semver monotonicity, and layout verification.
///
/// # Errors
/// Returns [`Error`] if any pre-flight verification check fails.
pub fn run_preflight(
  content_path: &Path,
  repo_dir: &Path,
  skip_build: bool,
) -> Result<PreflightOutcome, Error> {
  let raw_version = schema::load_content_version(content_path)?;
  let target_version = parse_version(&raw_version)?;

  git::check_working_tree_clean(repo_dir)?;
  git::check_upstream_synced(repo_dir)?;
  let latest_tag = check_semver_monotonicity(&target_version, repo_dir)?;

  if !skip_build {
    engine::verify_content(content_path, DEFAULT_TEMPLATE, None, None)?;
  }

  Ok(PreflightOutcome {
    target_version,
    latest_tag,
  })
}

/// Creates and pushes an annotated release tag to `origin`.
///
/// On push failure, automatically deletes the local tag to avoid a dangling
/// tag state.
///
/// # Errors
/// Returns [`Error::Git`] if tag creation or pushing fails.
pub fn execute_release(
  repo_dir: &Path,
  version: &Version,
  message: Option<&str>,
) -> Result<String, Error> {
  let tag_name = format!("v{version}");
  let default_msg = format!("v{version}");
  let tag_msg = message.unwrap_or(&default_msg);

  git::create_annotated_tag(repo_dir, &tag_name, tag_msg)?;

  if let Err(e) = git::push_tag(repo_dir, "origin", &tag_name) {
    let _ = git::delete_tag(repo_dir, &tag_name);
    return Err(e.into());
  }

  Ok(tag_name)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parse_version_variants() {
    assert_eq!(parse_version("1.2.3").unwrap(), Version::new(1, 2, 3));
    assert_eq!(parse_version("v1.2.3").unwrap(), Version::new(1, 2, 3));
    assert_eq!(parse_version("V2.0.0").unwrap(), Version::new(2, 0, 0));
    assert!(matches!(parse_version(""), Err(Error::EmptyVersion)));
    assert!(matches!(
      parse_version("invalid"),
      Err(Error::InvalidSemver { .. })
    ));
  }

  #[test]
  fn test_derive_actions_url_formats() {
    assert_eq!(
      derive_actions_url("git@github.com:octocat/resume.git"),
      "https://github.com/octocat/resume/actions"
    );
    assert_eq!(
      derive_actions_url("https://github.com/octocat/resume.git"),
      "https://github.com/octocat/resume/actions"
    );
    assert_eq!(
      derive_actions_url("https://github.com/octocat/resume"),
      "https://github.com/octocat/resume/actions"
    );
  }
}
