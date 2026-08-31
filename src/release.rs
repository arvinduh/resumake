//! Release orchestration, pre-flight repository and semver verification, and tag management.

use crate::engine::{verify_content, DEFAULT_TEMPLATE};
use crate::init::check_workflow_version_skew;
use crate::schema::load_content_version;
use colored::Colorize;
use gix::bstr::ByteSlice;
use semver::Version;
use std::path::Path;
use std::process::Command;

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
  /// Git working tree contains uncommitted changes.
  #[error("Working tree contains uncommitted changes. Please commit or stash them before releasing.")]
  UncommittedChanges,
  /// Repository HEAD has no commit.
  #[error("HEAD has no commit.")]
  NoHeadCommit,
  /// Current branch has no upstream tracking branch configured.
  #[error("Branch has no upstream tracking branch configured. Set an upstream remote branch before releasing.")]
  NoUpstreamBranch,
  /// Current branch has unpushed commits.
  #[error("Branch has {count} unpushed commit(s). Push your commits to upstream before releasing.")]
  UnpushedCommits {
    /// Number of unpushed commits.
    count: u64,
  },
  /// Proposed version is not strictly newer than existing release tags.
  #[error("Version v{target} is not strictly newer than existing tag v{latest} (semver monotonicity check failed).")]
  NonMonotonicSemver {
    /// Proposed version.
    target: Version,
    /// Highest existing tag version.
    latest: Version,
  },
  /// Error querying git remote origin URL.
  #[error("git remote get-url origin failed: {0}")]
  RemoteError(String),
  /// No URL configured for git remote origin.
  #[error(
    "git remote get-url origin failed: no URL found for remote 'origin'"
  )]
  NoRemoteUrl,
  /// Error interacting with git repository.
  #[error("Git error: {0}")]
  Git(String),
  /// Failed to spawn `git tag`.
  #[error("Failed to run git tag: {0}")]
  GitTagSpawn(#[source] std::io::Error),
  /// `git tag` command failed with non-zero exit status.
  #[error("Failed to create git tag '{tag}': {stderr}")]
  GitTagFailed {
    /// Tag name attempted.
    tag: String,
    /// Stderr output.
    stderr: String,
  },
  /// Failed to spawn `git push`.
  #[error("Failed to run git push: {0}")]
  GitPushSpawn(#[source] std::io::Error),
  /// `git push` command failed with non-zero exit status.
  #[error("Failed to push tag '{tag}' to origin: {stderr}")]
  GitPushFailed {
    /// Tag name attempted.
    tag: String,
    /// Stderr output.
    stderr: String,
  },
  /// Schema inspection error.
  #[error(transparent)]
  Schema(#[from] crate::schema::SchemaError),
  /// Engine compilation or verification error.
  #[error(transparent)]
  Engine(#[from] crate::engine::EngineError),
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
pub fn parse_version(s: &str) -> Result<Version, ReleaseError> {
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

/// Checks that the git working tree has no uncommitted or untracked changes.
///
/// # Errors
///
/// Returns a [`ReleaseError`] if working tree is dirty or git inspection fails.
pub fn check_working_tree_clean(repo_dir: &Path) -> Result<(), ReleaseError> {
  let repo = gix::discover(repo_dir).map_err(|e| {
    ReleaseError::Git(format!("Failed to open git repository: {e}"))
  })?;

  let status = repo.status(gix::progress::Discard).map_err(|e| {
    ReleaseError::Git(format!("Failed to get repository status: {e}"))
  })?;

  let mut iter = status.into_iter(Vec::new()).map_err(|e| {
    ReleaseError::Git(format!("Failed to inspect repository status: {e}"))
  })?;

  if let Some(item) = iter.next() {
    let _ = item.map_err(|e| {
      ReleaseError::Git(format!("Error during git status: {e}"))
    })?;
    return Err(ReleaseError::UncommittedChanges);
  }

  Ok(())
}

/// Verifies that the current branch tracks an upstream remote branch and has 0 unpushed commits.
///
/// # Errors
///
/// Returns a [`ReleaseError`] if upstream branch is missing, commits are unpushed, or git fails.
pub fn check_upstream_synced(repo_dir: &Path) -> Result<(), ReleaseError> {
  let repo = gix::discover(repo_dir).map_err(|e| {
    ReleaseError::Git(format!("Failed to open git repository: {e}"))
  })?;

  let head = repo
    .head()
    .map_err(|e| ReleaseError::Git(format!("Failed to resolve HEAD: {e}")))?;

  let head_id = head.id().ok_or(ReleaseError::NoHeadCommit)?.detach();

  let branch = head
    .try_into_referent()
    .ok_or(ReleaseError::NoUpstreamBranch)?;

  let tracking_ref_name = branch
    .remote_tracking_ref_name(gix::remote::Direction::Fetch)
    .transpose()
    .map_err(|e| {
      ReleaseError::Git(format!(
        "Failed to resolve upstream tracking branch: {e}"
      ))
    })?
    .ok_or(ReleaseError::NoUpstreamBranch)?;

  let upstream_ref = repo
    .try_find_reference(tracking_ref_name.as_ref())
    .map_err(|e| {
      ReleaseError::Git(format!("Failed to find upstream reference: {e}"))
    })?
    .ok_or(ReleaseError::NoUpstreamBranch)?;

  let upstream_id = upstream_ref
    .into_fully_peeled_id()
    .map_err(|e| {
      ReleaseError::Git(format!("Failed to peel upstream reference: {e}"))
    })?
    .detach();

  let walk = repo
    .rev_walk([head_id])
    .with_boundary([upstream_id])
    .all()
    .map_err(|e| {
      ReleaseError::Git(format!("Failed to traverse commits: {e}"))
    })?;

  let mut count: u64 = 0;
  for item in walk {
    let _ = item
      .map_err(|e| ReleaseError::Git(format!("Failed to walk commit: {e}")))?;
    count += 1;
  }

  if count > 0 {
    return Err(ReleaseError::UnpushedCommits { count });
  }

  Ok(())
}

/// Retrieves all existing semver git tags in the repository and returns the highest version, if any.
///
/// # Errors
///
/// Returns a [`ReleaseError`] if git tag inspection fails.
pub fn get_latest_semver_tag(
  repo_dir: &Path,
) -> Result<Option<Version>, ReleaseError> {
  let repo = gix::discover(repo_dir).map_err(|e| {
    ReleaseError::Git(format!("Failed to open git repository: {e}"))
  })?;

  let references = repo
    .references()
    .map_err(|e| ReleaseError::Git(format!("Failed to get references: {e}")))?;

  let tags = references
    .tags()
    .map_err(|e| ReleaseError::Git(format!("Failed to get tags: {e}")))?;

  let mut highest: Option<Version> = None;

  for tag in tags {
    let tag = tag.map_err(|e| {
      ReleaseError::Git(format!("Failed to read tag reference: {e}"))
    })?;
    let tag_name = tag.name().shorten().to_str().unwrap_or("");
    if let Ok(ver) = parse_version(tag_name) {
      match &highest {
        Some(cur) if ver > *cur => {
          highest = Some(ver);
        }
        None => {
          highest = Some(ver);
        }
        _ => {}
      }
    }
  }

  Ok(highest)
}

/// Validates that `target_ver` is strictly newer than any existing git semver tag.
///
/// # Errors
///
/// Returns a [`ReleaseError`] if `target_ver` is not strictly monotonic over existing tags.
pub fn check_semver_monotonicity(
  target_ver: &Version,
  repo_dir: &Path,
) -> Result<Option<Version>, ReleaseError> {
  let latest_tag = get_latest_semver_tag(repo_dir)?;
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

/// Gets the remote origin URL from git config.
///
/// # Errors
///
/// Returns a [`ReleaseError`] if git repository cannot be opened or origin URL is not set.
pub fn get_remote_origin_url(repo_dir: &Path) -> Result<String, ReleaseError> {
  let repo = gix::discover(repo_dir).map_err(|e| {
    ReleaseError::Git(format!("Failed to open git repository: {e}"))
  })?;

  let remote = repo
    .find_remote("origin")
    .map_err(|e| ReleaseError::RemoteError(e.to_string()))?;

  let url = remote
    .url(gix::remote::Direction::Fetch)
    .or_else(|| remote.url(gix::remote::Direction::Push))
    .ok_or(ReleaseError::NoRemoteUrl)?;

  Ok(url.to_bstring().to_string())
}

/// Runs the complete release pipeline: pre-flight checks, tag creation, and push.
///
/// # Errors
///
/// Returns a [`ReleaseError`] if any pre-flight verification, tagging, or pushing fails.
pub fn run_release(
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
  let raw_version = load_content_version(content_path)?;
  let target_ver = parse_version(&raw_version)?;

  check_workflow_version_skew(repo_dir, env!("CARGO_PKG_VERSION"));

  if !quiet {
    println!("Résumé Release v{target_ver}\n");
  }

  // Pre-flight check 1: Clean working tree
  check_working_tree_clean(repo_dir)?;
  if !quiet {
    println!("  {} working tree clean", "✓".green());
  }

  // Pre-flight check 2: Upstream sync
  check_upstream_synced(repo_dir)?;
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
    verify_content(content_path, DEFAULT_TEMPLATE, None, None, None)?;
    if !quiet {
      println!(
        "  {} pre-flight check passed (rsmk build --check)",
        "✓".green()
      );
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

  let tag_output = Command::new("git")
    .arg("tag")
    .arg("-a")
    .arg(&tag_name)
    .arg("-m")
    .arg(tag_msg)
    .current_dir(repo_dir)
    .output()
    .map_err(ReleaseError::GitTagSpawn)?;

  if !tag_output.status.success() {
    let stderr = String::from_utf8_lossy(&tag_output.stderr).to_string();
    return Err(ReleaseError::GitTagFailed {
      tag: tag_name,
      stderr: stderr.trim().to_string(),
    });
  }

  if !quiet {
    println!("\n  {} created tag v{target_ver}", "✓".green());
  }

  let push_output = Command::new("git")
    .arg("push")
    .arg("origin")
    .arg(&tag_name)
    .current_dir(repo_dir)
    .output()
    .map_err(ReleaseError::GitPushSpawn)?;

  if !push_output.status.success() {
    let stderr = String::from_utf8_lossy(&push_output.stderr).to_string();
    let _ = Command::new("git")
      .arg("tag")
      .arg("-d")
      .arg(&tag_name)
      .current_dir(repo_dir)
      .output();
    return Err(ReleaseError::GitPushFailed {
      tag: tag_name,
      stderr: stderr.trim().to_string(),
    });
  }

  if !quiet {
    println!("  {} pushed tag to origin", "✓".green());
    let remote_url = get_remote_origin_url(repo_dir)
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

  fn setup_test_repo(dir: &Path) {
    Command::new("git")
      .arg("init")
      .current_dir(dir)
      .output()
      .unwrap();
    Command::new("git")
      .args(["config", "user.name", "Test User"])
      .current_dir(dir)
      .output()
      .unwrap();
    Command::new("git")
      .args(["config", "user.email", "test@example.com"])
      .current_dir(dir)
      .output()
      .unwrap();
    Command::new("git")
      .args(["config", "commit.gpgsign", "false"])
      .current_dir(dir)
      .output()
      .unwrap();
  }

  #[test]
  fn test_check_working_tree_clean_unit() {
    let temp = tempfile::TempDir::new().unwrap();
    let dir = temp.path();
    setup_test_repo(dir);

    // Initial clean commit
    let file = dir.join("file.txt");
    std::fs::write(&file, "initial").unwrap();
    Command::new("git")
      .args(["add", "."])
      .current_dir(dir)
      .output()
      .unwrap();
    Command::new("git")
      .args(["commit", "-m", "init"])
      .current_dir(dir)
      .output()
      .unwrap();

    assert!(check_working_tree_clean(dir).is_ok());

    // Dirty with untracked file
    let untracked = dir.join("untracked.txt");
    std::fs::write(&untracked, "dirty").unwrap();
    assert!(check_working_tree_clean(dir).is_err());
    std::fs::remove_file(&untracked).unwrap();
    assert!(check_working_tree_clean(dir).is_ok());

    // Dirty with modified file
    std::fs::write(&file, "modified").unwrap();
    assert!(check_working_tree_clean(dir).is_err());

    // Staged change is also not clean
    Command::new("git")
      .args(["add", "."])
      .current_dir(dir)
      .output()
      .unwrap();
    assert!(check_working_tree_clean(dir).is_err());
  }

  #[test]
  fn test_get_latest_semver_tag_and_monotonicity_unit() {
    let temp = tempfile::TempDir::new().unwrap();
    let dir = temp.path();
    setup_test_repo(dir);

    let file = dir.join("file.txt");
    std::fs::write(&file, "initial").unwrap();
    Command::new("git")
      .args(["add", "."])
      .current_dir(dir)
      .output()
      .unwrap();
    Command::new("git")
      .args(["commit", "-m", "init"])
      .current_dir(dir)
      .output()
      .unwrap();

    // No tags yet
    assert_eq!(get_latest_semver_tag(dir).unwrap(), None);

    // Add non-semver tag and semver tags
    Command::new("git")
      .args(["tag", "non-semver"])
      .current_dir(dir)
      .output()
      .unwrap();
    Command::new("git")
      .args(["tag", "v0.1.0"])
      .current_dir(dir)
      .output()
      .unwrap();
    Command::new("git")
      .args(["tag", "v1.0.0"])
      .current_dir(dir)
      .output()
      .unwrap();
    Command::new("git")
      .args(["tag", "v0.9.5"])
      .current_dir(dir)
      .output()
      .unwrap();

    let latest = get_latest_semver_tag(dir).unwrap();
    assert_eq!(latest, Some(Version::parse("1.0.0").unwrap()));

    // Monotonicity check
    let v2 = Version::parse("2.0.0").unwrap();
    assert!(check_semver_monotonicity(&v2, dir).is_ok());

    let v1 = Version::parse("1.0.0").unwrap();
    assert!(check_semver_monotonicity(&v1, dir).is_err());

    let v0 = Version::parse("0.9.0").unwrap();
    assert!(check_semver_monotonicity(&v0, dir).is_err());
  }

  #[test]
  fn test_get_remote_origin_url_unit() {
    let temp = tempfile::TempDir::new().unwrap();
    let dir = temp.path();
    setup_test_repo(dir);

    // No remote
    assert!(get_remote_origin_url(dir).is_err());

    // Add origin
    Command::new("git")
      .args([
        "remote",
        "add",
        "origin",
        "https://github.com/arvinduh/resumake.git",
      ])
      .current_dir(dir)
      .output()
      .unwrap();

    let url = get_remote_origin_url(dir).unwrap();
    assert_eq!(url, "https://github.com/arvinduh/resumake.git");
  }

  #[test]
  fn test_check_upstream_synced_unit() {
    let temp = tempfile::TempDir::new().unwrap();
    let origin_dir = temp.path().join("remote.git");
    let work_dir = temp.path().join("repo");
    std::fs::create_dir_all(&origin_dir).unwrap();
    std::fs::create_dir_all(&work_dir).unwrap();

    // Bare remote
    Command::new("git")
      .args(["init", "--bare"])
      .current_dir(&origin_dir)
      .output()
      .unwrap();

    // Work repo
    setup_test_repo(&work_dir);

    // Initial commit
    let file = work_dir.join("file.txt");
    std::fs::write(&file, "initial").unwrap();
    Command::new("git")
      .args(["add", "."])
      .current_dir(&work_dir)
      .output()
      .unwrap();
    Command::new("git")
      .args(["commit", "-m", "init"])
      .current_dir(&work_dir)
      .output()
      .unwrap();

    // No upstream configured yet
    assert!(check_upstream_synced(&work_dir).is_err());

    // Configure remote and push
    Command::new("git")
      .args(["remote", "add", "origin", origin_dir.to_str().unwrap()])
      .current_dir(&work_dir)
      .output()
      .unwrap();
    Command::new("git")
      .args(["branch", "-M", "main"])
      .current_dir(&work_dir)
      .output()
      .unwrap();
    Command::new("git")
      .args(["push", "-u", "origin", "main"])
      .current_dir(&work_dir)
      .output()
      .unwrap();

    // Synced upstream
    assert!(check_upstream_synced(&work_dir).is_ok());

    // Add unpushed commit
    std::fs::write(&file, "modified").unwrap();
    Command::new("git")
      .args(["add", "."])
      .current_dir(&work_dir)
      .output()
      .unwrap();
    Command::new("git")
      .args(["commit", "-m", "second"])
      .current_dir(&work_dir)
      .output()
      .unwrap();

    let res = check_upstream_synced(&work_dir);
    assert!(res.is_err());
    assert!(res.unwrap_err().to_string().contains("1 unpushed commit"));
  }
}
