//! Integration tests for Git utilities and repository operations.

use resumake::utils::git;
use semver::Version;
use std::path::Path;
use std::process::Command;
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

  assert!(!git::is_inside_work_tree(dir));

  git::init_repo(dir).unwrap();
  assert!(git::is_inside_work_tree(dir));

  let nested = dir.join("sub").join("nested");
  std::fs::create_dir_all(&nested).unwrap();
  assert!(git::is_inside_work_tree(&nested));
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

  assert!(git::create_annotated_tag(dir, "v1.0.0", "Release 1.0.0").is_ok());
  let latest = git::get_latest_semver_tag(dir).unwrap();
  assert_eq!(latest, Some(Version::parse("1.0.0").unwrap()));

  assert!(git::delete_tag(dir, "v1.0.0").is_ok());
  let latest_after_del = git::get_latest_semver_tag(dir).unwrap();
  assert_eq!(latest_after_del, None);
}

#[test]
fn test_check_working_tree_clean() {
  let temp = TempDir::new().unwrap();
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

  assert!(git::check_working_tree_clean(dir).is_ok());

  // Dirty with untracked file
  let untracked = dir.join("untracked.txt");
  std::fs::write(&untracked, "dirty").unwrap();
  assert!(git::check_working_tree_clean(dir).is_err());
  std::fs::remove_file(&untracked).unwrap();
  assert!(git::check_working_tree_clean(dir).is_ok());

  // Dirty with modified file
  std::fs::write(&file, "modified").unwrap();
  assert!(git::check_working_tree_clean(dir).is_err());

  // Staged change is also not clean
  Command::new("git")
    .args(["add", "."])
    .current_dir(dir)
    .output()
    .unwrap();
  assert!(git::check_working_tree_clean(dir).is_err());
}

#[test]
fn test_get_latest_semver_tag() {
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

  // No tags yet
  assert_eq!(git::get_latest_semver_tag(dir).unwrap(), None);

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

  let latest = git::get_latest_semver_tag(dir).unwrap();
  assert_eq!(latest, Some(Version::parse("1.0.0").unwrap()));
}

#[test]
fn test_get_remote_origin_url() {
  let temp = TempDir::new().unwrap();
  let dir = temp.path();
  setup_test_repo(dir);

  // No remote
  assert!(git::get_remote_origin_url(dir).is_err());

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

  let url = git::get_remote_origin_url(dir).unwrap();
  assert_eq!(url, "https://github.com/arvinduh/resumake.git");
}

#[test]
fn test_check_upstream_synced() {
  let temp = TempDir::new().unwrap();
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
  assert!(git::check_upstream_synced(&work_dir).is_err());

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
  assert!(git::check_upstream_synced(&work_dir).is_ok());

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

  let res = git::check_upstream_synced(&work_dir);
  assert!(res.is_err());
  assert!(res.unwrap_err().to_string().contains("1 unpushed commit"));
}
