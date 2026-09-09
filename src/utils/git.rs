//! Centralized Git and GitHub CLI system operations.

use semver::Version;
use std::path::Path;
use std::process::Command;

/// Errors originating from Git and GitHub CLI operations.
#[derive(thiserror::Error, Debug)]
pub enum GitError {
  /// Git CLI execution error.
  #[error("Git error: {0}")]
  Command(String),
  /// Working tree has uncommitted or untracked changes.
  #[error(
    "Working tree has uncommitted changes. Commit or stash before proceeding."
  )]
  UncommittedChanges,
  /// No HEAD commit found.
  #[error("Repository has no commits on the current branch.")]
  NoHeadCommit,
  /// Current branch has no upstream tracking branch.
  #[error("Current branch has no upstream tracking branch configured.")]
  NoUpstreamBranch,
  /// Unpushed commits detected.
  #[error(
    "Branch has {count} unpushed commit(s). Push to upstream before releasing."
  )]
  UnpushedCommits {
    /// Number of unpushed commits.
    count: u64,
  },
  /// No URL configured for git remote origin.
  #[error(
    "git remote get-url origin failed: no URL found for remote 'origin'"
  )]
  NoRemoteUrl,
  /// Error querying git remote origin URL.
  #[error("git remote get-url origin failed: {0}")]
  RemoteError(String),
  /// Failed to spawn git process.
  #[error("Failed to spawn git process: {0}")]
  Spawn(#[source] std::io::Error),
  /// Git execution failed with non-zero exit code.
  #[error("git command failed: {stderr}")]
  Failed {
    /// Stderr output.
    stderr: String,
  },
}

/// Checks if the target directory is inside an existing git work tree.
pub(crate) fn is_inside_work_tree(dir: &Path) -> bool {
  Command::new("git")
    .args(["rev-parse", "--is-inside-work-tree"])
    .current_dir(dir)
    .output()
    .map(|output| {
      output.status.success()
        && String::from_utf8_lossy(&output.stdout).trim() == "true"
    })
    .unwrap_or(false)
}

/// Initializes a new git repository in the target directory.
///
/// # Errors
/// Returns a [`GitError`] if `git init` cannot be spawned or fails.
pub(crate) fn init_repo(dir: &Path) -> Result<(), GitError> {
  let output = Command::new("git")
    .arg("init")
    .current_dir(dir)
    .output()
    .map_err(GitError::Spawn)?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    return Err(GitError::Failed { stderr });
  }
  Ok(())
}

/// Checks that the git working tree has no uncommitted or untracked changes.
///
/// # Errors
/// Returns a [`GitError`] if working tree is dirty or git inspection fails.
pub(crate) fn check_working_tree_clean(
  repo_dir: &Path,
) -> Result<(), GitError> {
  let output = Command::new("git")
    .args(["status", "--porcelain"])
    .current_dir(repo_dir)
    .output()
    .map_err(|e| GitError::Command(format!("Failed to run git status: {e}")))?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    return Err(GitError::Command(format!("git status failed: {stderr}")));
  }

  let stdout = String::from_utf8_lossy(&output.stdout);
  if !stdout.trim().is_empty() {
    return Err(GitError::UncommittedChanges);
  }

  Ok(())
}

/// Verifies that the current branch tracks an upstream remote branch and has 0 unpushed commits.
///
/// # Errors
/// Returns a [`GitError`] if upstream branch is missing, commits are unpushed, or git fails.
pub(crate) fn check_upstream_synced(repo_dir: &Path) -> Result<(), GitError> {
  let repo_check = Command::new("git")
    .args(["rev-parse", "--git-dir"])
    .current_dir(repo_dir)
    .output()
    .map_err(|e| GitError::Command(format!("Failed to run git: {e}")))?;

  if !repo_check.status.success() {
    return Err(GitError::Command(format!(
      "Failed to open git repository: {}",
      String::from_utf8_lossy(&repo_check.stderr).trim()
    )));
  }

  let head_check = Command::new("git")
    .args(["rev-parse", "--verify", "HEAD"])
    .current_dir(repo_dir)
    .output()
    .map_err(|e| GitError::Command(format!("Failed to run git: {e}")))?;

  if !head_check.status.success() {
    return Err(GitError::NoHeadCommit);
  }

  let upstream_check = Command::new("git")
    .args(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
    .current_dir(repo_dir)
    .output()
    .map_err(|e| GitError::Command(format!("Failed to run git: {e}")))?;

  if !upstream_check.status.success() {
    return Err(GitError::NoUpstreamBranch);
  }

  let upstream_branch = String::from_utf8_lossy(&upstream_check.stdout)
    .trim()
    .to_string();
  if upstream_branch.is_empty() {
    return Err(GitError::NoUpstreamBranch);
  }

  let rev_list = Command::new("git")
    .args(["rev-list", "--left-right", "--count", "HEAD...@{u}"])
    .current_dir(repo_dir)
    .output()
    .map_err(|e| {
      GitError::Command(format!("Failed to run git rev-list: {e}"))
    })?;

  if !rev_list.status.success() {
    let stderr = String::from_utf8_lossy(&rev_list.stderr).to_string();
    return Err(GitError::Command(format!(
      "Failed to count unpushed commits: {stderr}"
    )));
  }

  let output_str = String::from_utf8_lossy(&rev_list.stdout);
  let counts: Vec<&str> = output_str.split_whitespace().collect();
  if let Some(ahead_str) = counts.first() {
    if let Ok(ahead) = ahead_str.parse::<u64>() {
      if ahead > 0 {
        return Err(GitError::UnpushedCommits { count: ahead });
      }
    }
  }

  Ok(())
}

/// Retrieves all existing semver git tags in the repository and returns the highest version, if any.
///
/// # Errors
/// Returns a [`GitError`] if git tag inspection fails.
pub(crate) fn get_latest_semver_tag(
  repo_dir: &Path,
) -> Result<Option<Version>, GitError> {
  let output = Command::new("git")
    .args(["tag", "-l"])
    .current_dir(repo_dir)
    .output()
    .map_err(|e| GitError::Command(format!("Failed to run git tag: {e}")))?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    return Err(GitError::Command(format!(
      "Failed to list git tags: {stderr}"
    )));
  }

  let stdout = String::from_utf8_lossy(&output.stdout);
  let mut highest: Option<Version> = None;

  for line in stdout.lines() {
    let tag_name = line.trim();
    if tag_name.is_empty() {
      continue;
    }
    let without_v = tag_name
      .strip_prefix('v')
      .or_else(|| tag_name.strip_prefix('V'))
      .unwrap_or(tag_name);
    if let Ok(ver) = Version::parse(without_v) {
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

/// Gets the remote origin URL from git config.
///
/// # Errors
/// Returns a [`GitError`] if git repository cannot be opened or origin URL is not set.
pub(crate) fn get_remote_origin_url(
  repo_dir: &Path,
) -> Result<String, GitError> {
  let output = Command::new("git")
    .args(["remote", "get-url", "origin"])
    .current_dir(repo_dir)
    .output()
    .map_err(|e| {
      GitError::Command(format!("Failed to run git remote get-url: {e}"))
    })?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if stderr.contains("No such remote")
      || stderr.contains("not found")
      || stderr.contains("fatal:")
    {
      return Err(GitError::NoRemoteUrl);
    }
    return Err(GitError::RemoteError(stderr.trim().to_string()));
  }

  let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
  if url.is_empty() {
    return Err(GitError::NoRemoteUrl);
  }

  Ok(url)
}

/// Creates an annotated git tag with the specified tag name and message.
///
/// # Errors
/// Returns a [`GitError`] if the git process fails to spawn or exits with a non-zero status.
pub(crate) fn create_annotated_tag(
  repo_dir: &Path,
  tag: &str,
  message: &str,
) -> Result<(), GitError> {
  let output = Command::new("git")
    .arg("tag")
    .arg("-a")
    .arg(tag)
    .arg("-m")
    .arg(message)
    .current_dir(repo_dir)
    .output()
    .map_err(GitError::Spawn)?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    return Err(GitError::Failed { stderr });
  }

  Ok(())
}

/// Pushes a git tag to a remote repository.
///
/// # Errors
/// Returns a [`GitError`] if the git process fails to spawn or exits with a non-zero status.
pub(crate) fn push_tag(
  repo_dir: &Path,
  remote: &str,
  tag: &str,
) -> Result<(), GitError> {
  let output = Command::new("git")
    .arg("push")
    .arg(remote)
    .arg(tag)
    .current_dir(repo_dir)
    .output()
    .map_err(GitError::Spawn)?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    return Err(GitError::Failed { stderr });
  }

  Ok(())
}

/// Deletes a local git tag.
///
/// # Errors
/// Returns a [`GitError`] if the git process fails to spawn or exits with a non-zero status.
pub(crate) fn delete_tag(repo_dir: &Path, tag: &str) -> Result<(), GitError> {
  let output = Command::new("git")
    .arg("tag")
    .arg("-d")
    .arg(tag)
    .current_dir(repo_dir)
    .output()
    .map_err(GitError::Spawn)?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    return Err(GitError::Failed { stderr });
  }

  Ok(())
}

/// Checks if GitHub CLI `gh` is installed and authenticated.
pub(crate) fn is_gh_authenticated(dir: &Path) -> bool {
  Command::new("gh")
    .args(["auth", "status"])
    .current_dir(dir)
    .output()
    .map(|output| output.status.success())
    .unwrap_or(false)
}

/// Creates a new GitHub repository via `gh` CLI and pushes the current branch.
///
/// # Errors
/// Returns a [`GitError`] if repository creation fails.
pub(crate) fn create_repo_and_push(dir: &Path) -> Result<(), GitError> {
  let status = Command::new("gh")
    .args(["repo", "create", "--source=.", "--push"])
    .current_dir(dir)
    .status()
    .map_err(GitError::Spawn)?;

  if !status.success() {
    return Err(GitError::Command("gh repo create failed".to_string()));
  }

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::TempDir;

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
  fn test_is_inside_work_tree_and_init() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    assert!(!is_inside_work_tree(dir));

    init_repo(dir).unwrap();
    assert!(is_inside_work_tree(dir));

    let nested = dir.join("sub").join("nested");
    std::fs::create_dir_all(&nested).unwrap();
    assert!(is_inside_work_tree(&nested));
  }

  #[test]
  fn test_create_and_delete_annotated_tag() {
    let temp = TempDir::new().unwrap();
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

    assert!(create_annotated_tag(dir, "v1.0.0", "Release 1.0.0").is_ok());
    let latest = get_latest_semver_tag(dir).unwrap();
    assert_eq!(latest, Some(Version::parse("1.0.0").unwrap()));

    assert!(delete_tag(dir, "v1.0.0").is_ok());
    let latest_after_del = get_latest_semver_tag(dir).unwrap();
    assert_eq!(latest_after_del, None);
  }
}
