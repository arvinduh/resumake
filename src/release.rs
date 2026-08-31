//! Release orchestration, pre-flight repository and semver verification, and tag management.

use crate::engine::{verify_content, DEFAULT_TEMPLATE};
use crate::init::check_workflow_version_skew;
use crate::schema::load_content_version;
use colored::Colorize;
use std::cmp::Ordering;
use std::fmt;
use std::path::Path;
use std::process::Command;

/// Semantic version representation with comparison and validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemVer {
  /// Major version number.
  pub major: u64,
  /// Minor version number.
  pub minor: u64,
  /// Patch version number.
  pub patch: u64,
  /// Optional prerelease identifier (e.g. `rc.1`, `beta.2`).
  pub pre: Option<String>,
}

impl SemVer {
  /// Parses a string into a [`SemVer`].
  ///
  /// Supports optional leading `'v'`/`'V'` and build metadata (ignored for comparison).
  pub fn parse(s: &str) -> Result<Self, String> {
    let s = s.trim();
    let without_v = s
      .strip_prefix('v')
      .or_else(|| s.strip_prefix('V'))
      .unwrap_or(s);
    if without_v.is_empty() {
      return Err("Version string cannot be empty".to_string());
    }

    // Discard build metadata (after '+')
    let without_build = without_v.split('+').next().unwrap_or(without_v);

    // Split on '-' for prerelease
    let mut parts = without_build.splitn(2, '-');
    let main_part = parts.next().unwrap_or("");
    let pre_part = parts.next().map(|p| p.to_string());

    let num_parts: Vec<&str> = main_part.split('.').collect();
    if num_parts.len() != 3 {
      return Err(format!(
        "Invalid semver '{s}': expected format 'MAJOR.MINOR.PATCH'"
      ));
    }

    let major = num_parts[0].parse::<u64>().map_err(|_| {
      format!(
        "Invalid semver '{s}': invalid major version '{}'",
        num_parts[0]
      )
    })?;
    let minor = num_parts[1].parse::<u64>().map_err(|_| {
      format!(
        "Invalid semver '{s}': invalid minor version '{}'",
        num_parts[1]
      )
    })?;
    let patch = num_parts[2].parse::<u64>().map_err(|_| {
      format!(
        "Invalid semver '{s}': invalid patch version '{}'",
        num_parts[2]
      )
    })?;

    Ok(Self {
      major,
      minor,
      patch,
      pre: pre_part,
    })
  }
}

impl PartialOrd for SemVer {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for SemVer {
  fn cmp(&self, other: &Self) -> Ordering {
    match self.major.cmp(&other.major) {
      Ordering::Equal => {}
      ord => return ord,
    }
    match self.minor.cmp(&other.minor) {
      Ordering::Equal => {}
      ord => return ord,
    }
    match self.patch.cmp(&other.patch) {
      Ordering::Equal => {}
      ord => return ord,
    }

    // Prerelease comparison per SemVer 2.0.0
    match (&self.pre, &other.pre) {
      (None, None) => Ordering::Equal,
      (Some(_), None) => Ordering::Less,
      (None, Some(_)) => Ordering::Greater,
      (Some(a), Some(b)) => compare_prerelease(a, b),
    }
  }
}

fn compare_prerelease(a: &str, b: &str) -> Ordering {
  let a_parts: Vec<&str> = a.split('.').collect();
  let b_parts: Vec<&str> = b.split('.').collect();

  for (part_a, part_b) in a_parts.iter().zip(b_parts.iter()) {
    let num_a = part_a.parse::<u64>();
    let num_b = part_b.parse::<u64>();

    match (num_a, num_b) {
      (Ok(na), Ok(nb)) => match na.cmp(&nb) {
        Ordering::Equal => continue,
        ord => return ord,
      },
      (Ok(_), Err(_)) => return Ordering::Less,
      (Err(_), Ok(_)) => return Ordering::Greater,
      (Err(_), Err(_)) => match part_a.cmp(part_b) {
        Ordering::Equal => continue,
        ord => return ord,
      },
    }
  }

  a_parts.len().cmp(&b_parts.len())
}

impl fmt::Display for SemVer {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match &self.pre {
      Some(pre) => {
        write!(f, "{}.{}.{}-{}", self.major, self.minor, self.patch, pre)
      }
      None => write!(f, "{}.{}.{}", self.major, self.minor, self.patch),
    }
  }
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
pub fn check_working_tree_clean(repo_dir: &Path) -> Result<(), String> {
  let output = Command::new("git")
    .arg("status")
    .arg("--porcelain")
    .current_dir(repo_dir)
    .output()
    .map_err(|e| format!("Failed to run git status: {e}"))?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr);
    return Err(format!("git status failed: {}", stderr.trim()));
  }

  let stdout = String::from_utf8_lossy(&output.stdout);
  if !stdout.trim().is_empty() {
    return Err(
      "Working tree contains uncommitted changes. Please commit or stash them before releasing."
        .to_string(),
    );
  }

  Ok(())
}

/// Verifies that the current branch tracks an upstream remote branch and has 0 unpushed commits.
pub fn check_upstream_synced(repo_dir: &Path) -> Result<(), String> {
  let rev_parse = Command::new("git")
    .arg("rev-parse")
    .arg("--abbrev-ref")
    .arg("@{u}")
    .current_dir(repo_dir)
    .output()
    .map_err(|e| format!("Failed to run git rev-parse: {e}"))?;

  if !rev_parse.status.success() {
    return Err(
      "Branch has no upstream tracking branch configured. Set an upstream remote branch before releasing."
        .to_string(),
    );
  }

  let rev_list = Command::new("git")
    .arg("rev-list")
    .arg("@{u}..HEAD")
    .arg("--count")
    .current_dir(repo_dir)
    .output()
    .map_err(|e| format!("Failed to run git rev-list: {e}"))?;

  if !rev_list.status.success() {
    let stderr = String::from_utf8_lossy(&rev_list.stderr);
    return Err(format!("git rev-list failed: {}", stderr.trim()));
  }

  let count_str = String::from_utf8_lossy(&rev_list.stdout);
  let count: u64 = count_str.trim().parse().map_err(|_| {
    format!("Unexpected git rev-list output: '{}'", count_str.trim())
  })?;

  if count > 0 {
    return Err(format!(
      "Branch has {count} unpushed commit(s). Push your commits to upstream before releasing."
    ));
  }

  Ok(())
}

/// Retrieves all existing semver git tags in the repository and returns the highest version, if any.
pub fn get_latest_semver_tag(
  repo_dir: &Path,
) -> Result<Option<SemVer>, String> {
  let output = Command::new("git")
    .arg("tag")
    .arg("-l")
    .current_dir(repo_dir)
    .output()
    .map_err(|e| format!("Failed to run git tag: {e}"))?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr);
    return Err(format!("git tag failed: {}", stderr.trim()));
  }

  let stdout = String::from_utf8_lossy(&output.stdout);
  let mut highest: Option<SemVer> = None;

  for line in stdout.lines() {
    let tag = line.trim();
    if let Ok(ver) = SemVer::parse(tag) {
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
pub fn check_semver_monotonicity(
  target_ver: &SemVer,
  repo_dir: &Path,
) -> Result<Option<SemVer>, String> {
  let latest_tag = get_latest_semver_tag(repo_dir)?;
  if let Some(ref latest) = latest_tag {
    if target_ver <= latest {
      return Err(format!(
        "Version v{target_ver} is not strictly newer than existing tag v{latest} (semver monotonicity check failed)."
      ));
    }
  }
  Ok(latest_tag)
}

/// Gets the remote origin URL from git config.
pub fn get_remote_origin_url(repo_dir: &Path) -> Result<String, String> {
  let output = Command::new("git")
    .arg("remote")
    .arg("get-url")
    .arg("origin")
    .current_dir(repo_dir)
    .output()
    .map_err(|e| format!("Failed to get remote origin url: {e}"))?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr);
    return Err(format!(
      "git remote get-url origin failed: {}",
      stderr.trim()
    ));
  }

  Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Runs the complete release pipeline: pre-flight checks, tag creation, and push.
pub fn run_release(
  content_path: &Path,
  message: Option<&str>,
  dry_run: bool,
  skip_build: bool,
  quiet: bool,
) -> Result<(), String> {
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
  let target_ver = SemVer::parse(&raw_version)?;

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
    .map_err(|e| format!("Failed to run git tag: {e}"))?;

  if !tag_output.status.success() {
    let stderr = String::from_utf8_lossy(&tag_output.stderr);
    return Err(format!(
      "Failed to create git tag '{tag_name}': {}",
      stderr.trim()
    ));
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
    .map_err(|e| format!("Failed to run git push: {e}"))?;

  if !push_output.status.success() {
    let stderr = String::from_utf8_lossy(&push_output.stderr);
    let _ = Command::new("git")
      .arg("tag")
      .arg("-d")
      .arg(&tag_name)
      .current_dir(repo_dir)
      .output();
    return Err(format!(
      "Failed to push tag '{tag_name}' to origin: {}",
      stderr.trim()
    ));
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
    let v1 = SemVer::parse("1.2.0").unwrap();
    assert_eq!(v1.major, 1);
    assert_eq!(v1.minor, 2);
    assert_eq!(v1.patch, 0);
    assert_eq!(v1.pre, None);
    assert_eq!(v1.to_string(), "1.2.0");

    let v2 = SemVer::parse("v0.1.1").unwrap();
    assert_eq!(v2.to_string(), "0.1.1");

    let v3 = SemVer::parse("V2.0.0-rc.1+build.42").unwrap();
    assert_eq!(v3.pre, Some("rc.1".to_string()));
    assert_eq!(v3.to_string(), "2.0.0-rc.1");

    assert!(SemVer::parse("invalid").is_err());
    assert!(SemVer::parse("1.2").is_err());
    assert!(SemVer::parse("").is_err());
  }

  #[test]
  fn test_semver_comparison() {
    let v1 = SemVer::parse("1.0.0").unwrap();
    let v2 = SemVer::parse("1.1.0").unwrap();
    let v3 = SemVer::parse("1.2.0").unwrap();
    let v3_rc = SemVer::parse("1.2.0-rc.1").unwrap();

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
