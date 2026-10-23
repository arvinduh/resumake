//! Release orchestration, pre-flight repository and semver verification, and
//! tag management.

use std::path::{Path, PathBuf};

use colored::Colorize;
use resumake::engine::templates::DEFAULT_TEMPLATE;
use resumake::utils::git;
use resumake::{engine, schema};
use semver::Version;

use crate::cli::init;

/// Available CLI arguments for `rsmk release`.
#[derive(Debug, Clone, PartialEq, Eq, clap::Args)]
pub struct ReleaseArgs {
  /// Path to content YAML file
  #[arg(value_name = "CONTENT", default_value = "content.yaml")]
  pub content: PathBuf,

  /// Optional release message for the annotated git tag
  #[arg(short, long)]
  pub message: Option<String>,

  /// Dry-run mode: run all pre-flight checks without creating or pushing a git
  /// tag
  #[arg(long)]
  pub dry_run: bool,

  /// Skip compilation and telemetry pre-flight check (rsmk check)
  #[arg(long)]
  pub skip_build: bool,
}

impl ReleaseArgs {
  /// Runs the release command.
  ///
  /// # Errors
  ///
  /// Returns an [`Error`] if pre-flight checks, tag creation, or tag push
  /// fails.
  pub fn run(self, quiet: bool) -> Result<(), Error> {
    run_release(
      &self.content,
      self.message.as_deref(),
      self.dry_run,
      self.skip_build,
      quiet,
    )
  }
}

/// Errors originating from release pipeline and semver verification.
#[derive(thiserror::Error, Debug)]
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
  /// Git or `GitHub` CLI operation error.
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
fn parse_version(s: &str) -> Result<Version, Error> {
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

/// Validates that `target_ver` is strictly newer than any existing git semver
/// tag.
fn check_semver_monotonicity(
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

/// Runs the complete release pipeline: pre-flight checks, tag creation, and
/// push.
pub(crate) fn run_release(
  content_path: &Path,
  message: Option<&str>,
  dry_run: bool,
  skip_build: bool,
  quiet: bool,
) -> Result<(), Error> {
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
