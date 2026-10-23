//! Centralized Git and `GitHub` CLI system operations.

use std::path::Path;
use std::process::Command;

use semver::Version;

/// Errors originating from Git and `GitHub` CLI operations.
#[derive(thiserror::Error, Debug)]
pub enum Error {
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
#[must_use]
pub fn is_inside_work_tree(dir: &Path) -> bool {
  Command::new("git")
    .args(["rev-parse", "--is-inside-work-tree"])
    .current_dir(dir)
    .output()
    .is_ok_and(|output| {
      output.status.success()
        && String::from_utf8_lossy(&output.stdout).trim() == "true"
    })
}

/// Initializes a new git repository in the target directory.
///
/// # Errors
/// Returns a [`Error`] if `git init` cannot be spawned or fails.
pub fn init_repo(dir: &Path) -> Result<(), Error> {
  let output = Command::new("git")
    .arg("init")
    .current_dir(dir)
    .output()
    .map_err(Error::Spawn)?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    return Err(Error::Failed { stderr });
  }
  Ok(())
}

/// Checks that the git working tree has no uncommitted or untracked changes.
///
/// # Errors
/// Returns a [`Error`] if working tree is dirty or git inspection fails.
pub fn check_working_tree_clean(repo_dir: &Path) -> Result<(), Error> {
  let output = Command::new("git")
    .args(["status", "--porcelain"])
    .current_dir(repo_dir)
    .output()
    .map_err(|e| Error::Command(format!("Failed to run git status: {e}")))?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    return Err(Error::Command(format!("git status failed: {stderr}")));
  }

  let stdout = String::from_utf8_lossy(&output.stdout);
  if !stdout.trim().is_empty() {
    return Err(Error::UncommittedChanges);
  }

  Ok(())
}

/// Verifies that the current branch tracks an upstream remote branch and has 0
/// unpushed commits.
///
/// # Errors
/// Returns a [`Error`] if upstream branch is missing, commits are unpushed, or
/// git fails.
pub fn check_upstream_synced(repo_dir: &Path) -> Result<(), Error> {
  let repo_check = Command::new("git")
    .args(["rev-parse", "--git-dir"])
    .current_dir(repo_dir)
    .output()
    .map_err(|e| Error::Command(format!("Failed to run git: {e}")))?;

  if !repo_check.status.success() {
    return Err(Error::Command(format!(
      "Failed to open git repository: {}",
      String::from_utf8_lossy(&repo_check.stderr).trim()
    )));
  }

  let head_check = Command::new("git")
    .args(["rev-parse", "--verify", "HEAD"])
    .current_dir(repo_dir)
    .output()
    .map_err(|e| Error::Command(format!("Failed to run git: {e}")))?;

  if !head_check.status.success() {
    return Err(Error::NoHeadCommit);
  }

  let upstream_check = Command::new("git")
    .args(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
    .current_dir(repo_dir)
    .output()
    .map_err(|e| Error::Command(format!("Failed to run git: {e}")))?;

  if !upstream_check.status.success() {
    return Err(Error::NoUpstreamBranch);
  }

  let upstream_branch = String::from_utf8_lossy(&upstream_check.stdout)
    .trim()
    .to_string();
  if upstream_branch.is_empty() {
    return Err(Error::NoUpstreamBranch);
  }

  let rev_list = Command::new("git")
    .args(["rev-list", "--left-right", "--count", "HEAD...@{u}"])
    .current_dir(repo_dir)
    .output()
    .map_err(|e| Error::Command(format!("Failed to run git rev-list: {e}")))?;

  if !rev_list.status.success() {
    let stderr = String::from_utf8_lossy(&rev_list.stderr).to_string();
    return Err(Error::Command(format!(
      "Failed to count unpushed commits: {stderr}"
    )));
  }

  let output_str = String::from_utf8_lossy(&rev_list.stdout);
  let counts: Vec<&str> = output_str.split_whitespace().collect();
  if let Some(ahead_str) = counts.first() {
    if let Ok(ahead) = ahead_str.parse::<u64>() {
      if ahead > 0 {
        return Err(Error::UnpushedCommits { count: ahead });
      }
    }
  }

  Ok(())
}

/// Extracts the highest semantic version from an iterator of tag strings.
#[must_use]
pub fn find_highest_semver<'a, I>(tags: I) -> Option<Version>
where
  I: IntoIterator<Item = &'a str>,
{
  let mut highest: Option<Version> = None;

  for line in tags {
    let tag_name = line.trim();
    if tag_name.is_empty() {
      continue;
    }
    let without_v = tag_name
      .strip_prefix('v')
      .or_else(|| tag_name.strip_prefix('V'))
      .unwrap_or(tag_name);
    if let Ok(ver) = Version::parse(without_v) {
      if highest.as_ref().is_none_or(|cur| ver > *cur) {
        highest = Some(ver);
      }
    }
  }

  highest
}

/// Retrieves all existing semver git tags in the repository and returns the
/// highest version, if any.
///
/// # Errors
/// Returns a [`Error`] if git tag inspection fails.
pub fn get_latest_semver_tag(
  repo_dir: &Path,
) -> Result<Option<Version>, Error> {
  let output = Command::new("git")
    .args(["tag", "-l"])
    .current_dir(repo_dir)
    .output()
    .map_err(|e| Error::Command(format!("Failed to run git tag: {e}")))?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    return Err(Error::Command(format!("Failed to list git tags: {stderr}")));
  }

  let stdout = String::from_utf8_lossy(&output.stdout);
  Ok(find_highest_semver(stdout.lines()))
}

/// Gets the remote origin URL from git config.
///
/// # Errors
/// Returns a [`Error`] if git repository cannot be opened or origin URL is not
/// set.
pub fn get_remote_origin_url(repo_dir: &Path) -> Result<String, Error> {
  let output = Command::new("git")
    .args(["remote", "get-url", "origin"])
    .current_dir(repo_dir)
    .output()
    .map_err(|e| {
      Error::Command(format!("Failed to run git remote get-url: {e}"))
    })?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if stderr.contains("No such remote")
      || stderr.contains("not found")
      || stderr.contains("fatal:")
    {
      return Err(Error::NoRemoteUrl);
    }
    return Err(Error::RemoteError(stderr.trim().to_string()));
  }

  let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
  if url.is_empty() {
    return Err(Error::NoRemoteUrl);
  }

  Ok(url)
}

/// Creates an annotated git tag with the specified tag name and message.
///
/// # Errors
/// Returns a [`Error`] if the git process fails to spawn or exits with a
/// non-zero status.
pub fn create_annotated_tag(
  repo_dir: &Path,
  tag: &str,
  message: &str,
) -> Result<(), Error> {
  let output = Command::new("git")
    .arg("tag")
    .arg("-a")
    .arg(tag)
    .arg("-m")
    .arg(message)
    .current_dir(repo_dir)
    .output()
    .map_err(Error::Spawn)?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    return Err(Error::Failed { stderr });
  }

  Ok(())
}

/// Pushes a git tag to a remote repository.
///
/// # Errors
/// Returns a [`Error`] if the git process fails to spawn or exits with a
/// non-zero status.
pub fn push_tag(repo_dir: &Path, remote: &str, tag: &str) -> Result<(), Error> {
  let output = Command::new("git")
    .arg("push")
    .arg(remote)
    .arg(tag)
    .current_dir(repo_dir)
    .output()
    .map_err(Error::Spawn)?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    return Err(Error::Failed { stderr });
  }

  Ok(())
}

/// Deletes a local git tag.
///
/// # Errors
/// Returns a [`Error`] if the git process fails to spawn or exits with a
/// non-zero status.
pub fn delete_tag(repo_dir: &Path, tag: &str) -> Result<(), Error> {
  let output = Command::new("git")
    .arg("tag")
    .arg("-d")
    .arg(tag)
    .current_dir(repo_dir)
    .output()
    .map_err(Error::Spawn)?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    return Err(Error::Failed { stderr });
  }

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_find_highest_semver() {
    let tags = [
      "v0.1.0",
      "s1.0",
      "v1.2.0-rc.1",
      "v1.2.0",
      "1.1.5",
      "invalid-tag",
      "v0.9.9",
    ];
    let highest = find_highest_semver(tags).unwrap();
    assert_eq!(highest, Version::parse("1.2.0").unwrap());
  }

  #[test]
  fn test_find_highest_semver_empty() {
    assert_eq!(find_highest_semver(["not-semver", ""]), None);
  }
}
