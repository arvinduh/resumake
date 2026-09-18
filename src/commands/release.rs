//! Release orchestration, pre-flight repository and semver verification, and tag management.

use crate::commands::init;
use crate::engine;
use crate::engine::error::EngineError;
use crate::engine::templates::DEFAULT_TEMPLATE;
use crate::schema::{self, SchemaError};
use crate::utils::git::{self, GitError};
use colored::Colorize;
use semver::Version;
use std::path::Path;

/// Errors originating from release pipeline and semver verification.
#[derive(thiserror::Error, Debug)]
pub enum ReleaseError {
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
  #[error("Version v{target} is not strictly newer than existing tag v{latest} (semver monotonicity check failed).")]
  NonMonotonicSemver {
    /// Proposed version.
    target: Version,
    /// Highest existing tag version.
    latest: Version,
  },
  /// Git or GitHub CLI operation error.
  #[error(transparent)]
  Git(#[from] GitError),
  /// Schema inspection error.
  #[error(transparent)]
  Schema(#[from] SchemaError),
  /// Engine compilation or verification error.
  #[error(transparent)]
  Engine(#[from] EngineError),
  /// Underlying I/O error.
  #[error("I/O error: {0}")]
  Io(#[from] std::io::Error),
}

/// Parses a version string into a [`Version`].
///
/// Supports optional leading `'v'`/`'V'`.
///
/// # Errors
///
/// Returns a [`ReleaseError`] if version string is empty or invalid semver.
fn parse_version(s: &str) -> Result<Version, ReleaseError> {
  let s_trimmed = s.trim();
  let without_v = s_trimmed
    .strip_prefix('v')
    .or_else(|| s_trimmed.strip_prefix('V'))
    .unwrap_or(s_trimmed);
  if without_v.is_empty() {
    return Err(ReleaseError::EmptyVersion);
  }

  Version::parse(without_v).map_err(|source| ReleaseError::InvalidSemver {
    version: s.to_string(),
    source,
  })
}

/// Derives the GitHub Actions URL from a git remote URL.
fn derive_actions_url(remote_url: &str) -> String {
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

/// Validates that `target_ver` is strictly newer than any existing git semver tag.
///
/// # Errors
///
/// Returns a [`ReleaseError`] if `target_ver` is not strictly monotonic over existing tags.
fn check_semver_monotonicity(
  target_ver: &Version,
  repo_dir: &Path,
) -> Result<Option<Version>, ReleaseError> {
  let latest_tag = git::get_latest_semver_tag(repo_dir)?;
  if let Some(ref latest) = latest_tag {
    if target_ver <= latest {
      return Err(ReleaseError::NonMonotonicSemver {
        target: target_ver.clone(),
        latest: latest.clone(),
      });
    }
  }
  Ok(latest_tag)
}

/// Runs the complete release pipeline: pre-flight checks, tag creation, and push.
///
/// # Errors
///
/// Returns a [`ReleaseError`] if any pre-flight verification, tagging, or pushing fails.
pub(crate) fn run_release(
  content_path: &Path,
  message: Option<&str>,
  dry_run: bool,
  skip_build: bool,
  quiet: bool,
) -> Result<(), ReleaseError> {
  let repo_dir = if content_path.is_file() {
    content_path.parent().unwrap_or(Path::new("."))
  } else {
    Path::new(".")
  };
  let repo_dir = if repo_dir.as_os_str().is_empty() {
    Path::new(".")
  } else {
    repo_dir
  };

  // 1. Read and validate version from content.yaml
  let raw_version = schema::load_content_version(content_path)?;
  let target_ver = parse_version(&raw_version)?;

  init::check_workflow_version_skew(repo_dir, env!("CARGO_PKG_VERSION"));

  if !quiet {
    println!("Résumé Release v{target_ver}\n");
  }

  // Pre-flight check 1: Clean working tree
  git::check_working_tree_clean(repo_dir)?;
  if !quiet {
    println!("  {} working tree clean", "✓".green());
  }

  // Pre-flight check 2: Upstream sync
  git::check_upstream_synced(repo_dir)?;
  if !quiet {
    println!(
      "  {} upstream branch synced (nothing unpushed)",
      "✓".green()
    );
  }

  // Pre-flight check 3: Semver monotonicity
  let latest_tag = check_semver_monotonicity(&target_ver, repo_dir)?;
  if !quiet {
    if let Some(prev) = latest_tag {
      println!("  {} v{target_ver} is new, ahead of v{prev}", "✓".green());
    } else {
      println!("  {} v{target_ver} is new", "✓".green());
    }
  }

  // Pre-flight check 4: Build / layout check
  if !skip_build {
    engine::verify_content(content_path, DEFAULT_TEMPLATE, None, None)?;
    if !quiet {
      println!("  {} pre-flight check passed (rsmk check)", "✓".green());
    }
  } else if !quiet {
    println!("  {} pre-flight check skipped (--skip-build)", "✓".green());
  }

  if dry_run {
    return Ok(());
  }

  // 5. Atomic Tag & Push
  let tag_name = format!("v{target_ver}");
  let default_msg = format!("v{target_ver}");
  let tag_msg = message.unwrap_or(&default_msg);

  git::create_annotated_tag(repo_dir, &tag_name, tag_msg)?;

  if !quiet {
    println!("\n  {} created tag v{target_ver}", "✓".green());
  }

  if let Err(e) = git::push_tag(repo_dir, "origin", &tag_name) {
    let _ = git::delete_tag(repo_dir, &tag_name);
    return Err(e.into());
  }

  if !quiet {
    println!("  {} pushed tag to origin", "✓".green());
    let remote_url = git::get_remote_origin_url(repo_dir)
      .unwrap_or_else(|_| "https://github.com/arvinduh/resumake".to_string());
    let actions_url = derive_actions_url(&remote_url);
    println!("    Release workflow triggered: {actions_url}");
  }

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_semver_parse_and_display() {
    let v1 = parse_version("1.2.0").unwrap();
    assert_eq!(v1.major, 1);
    assert_eq!(v1.minor, 2);
    assert_eq!(v1.patch, 0);
    assert!(v1.pre.is_empty());
    assert_eq!(v1.to_string(), "1.2.0");

    let v2 = parse_version("v0.1.1").unwrap();
    assert_eq!(v2.to_string(), "0.1.1");

    let v3 = parse_version("V2.0.0-rc.1+build.42").unwrap();
    assert_eq!(v3.pre.as_str(), "rc.1");
    assert_eq!(v3.to_string(), "2.0.0-rc.1+build.42");

    assert!(parse_version("invalid").is_err());
    assert!(parse_version("1.2").is_err());
    assert!(parse_version("").is_err());
  }

  #[test]
  fn test_semver_comparison() {
    let v1 = parse_version("1.0.0").unwrap();
    let v2 = parse_version("1.1.0").unwrap();
    let v3 = parse_version("1.2.0").unwrap();
    let v3_rc = parse_version("1.2.0-rc.1").unwrap();

    assert!(v2 > v1);
    assert!(v3 > v2);
    assert!(v3 > v3_rc);
    assert!(v3_rc > v2);
  }

  #[test]
  fn test_derive_actions_url() {
    assert_eq!(
      derive_actions_url("https://github.com/arvinduh/resumake"),
      "https://github.com/arvinduh/resumake/actions"
    );
    assert_eq!(
      derive_actions_url("https://github.com/arvinduh/resumake.git"),
      "https://github.com/arvinduh/resumake/actions"
    );
    assert_eq!(
      derive_actions_url("git@github.com:arvinduh/resumake.git"),
      "https://github.com/arvinduh/resumake/actions"
    );
    assert_eq!(
      derive_actions_url("ssh://git@github.com/user/custom-repo.git"),
      "https://github.com/user/custom-repo/actions"
    );
  }
}
