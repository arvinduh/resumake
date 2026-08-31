//! End-to-end integration tests for rsmk CLI.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_cli_help() {
  let mut cmd = Command::cargo_bin("rsmk").unwrap();
  cmd
    .arg("--help")
    .assert()
    .success()
    .stdout(predicate::str::contains("rsmk"))
    .stdout(predicate::str::contains("Compile"));
}

#[test]
fn test_cli_init_and_schema_export() {
  let temp = TempDir::new().unwrap();
  let content_file = temp.path().join("content.yaml");
  let schema_file = temp.path().join("schema.json");

  // 1. Test init
  let mut cmd_init = Command::cargo_bin("rsmk").unwrap();
  cmd_init
    .arg("init")
    .arg("--name")
    .arg("Jane Doe")
    .arg("--output")
    .arg(&content_file)
    .assert()
    .success()
    .stdout(predicate::str::contains("[PASS]"));

  assert!(content_file.exists());
  let content = fs::read_to_string(&content_file).unwrap();
  assert!(content.contains("Jane Doe"));
  assert!(content.contains("Libertinus Serif"));
  assert!(!content.contains("Linux Libertine"));

  // 2. Test schema export
  let mut cmd_schema = Command::cargo_bin("rsmk").unwrap();
  cmd_schema
    .arg("schema")
    .arg("--export")
    .arg(&schema_file)
    .assert()
    .success()
    .stdout(predicate::str::contains("[PASS]"));

  assert!(schema_file.exists());
  let schema = fs::read_to_string(&schema_file).unwrap();
  assert!(schema.contains("ResumeDocument"));
}

#[test]
fn test_cli_init_then_build_succeeds_end_to_end() {
  // This test shells out to `typst` via `rsmk check`. On a machine
  // without the Typst compiler on PATH, skip loudly rather than failing so
  // `cargo test --all-targets` stays green on a fresh clone. CI installs
  // typst and still exercises the real path.
  if which::which("typst").is_err() {
    eprintln!(
      "skipping test_cli_init_then_build_succeeds_end_to_end: typst not on PATH"
    );
    return;
  }

  // Regression test: the default `init` scaffold must actually compile.
  // Earlier versions emitted <bulletinfo> probes without a required `id`
  // field, which made `build`/`check` fail on every real resume (i.e. any
  // content with bullets) despite `init` and schema export succeeding in
  // isolation.
  let temp = TempDir::new().unwrap();
  let content_file = temp.path().join("content.yaml");

  Command::cargo_bin("rsmk")
    .unwrap()
    .arg("init")
    .arg("--name")
    .arg("Jane Doe")
    .arg("--output")
    .arg(&content_file)
    .assert()
    .success();

  Command::cargo_bin("rsmk")
    .unwrap()
    .current_dir(temp.path())
    .arg("check")
    .arg("--content")
    .arg(&content_file)
    .assert()
    .success()
    .stdout(predicate::str::contains("SUCCESS"));
}

#[test]
fn test_cli_check_detects_invalid_yaml() {
  let temp = TempDir::new().unwrap();
  let invalid_file = temp.path().join("invalid.yaml");
  fs::write(&invalid_file, "invalid: [yaml").unwrap();

  let mut cmd = Command::cargo_bin("rsmk").unwrap();
  cmd
    .arg("check")
    .arg("--content")
    .arg(&invalid_file)
    .assert()
    .failure()
    .stderr(predicate::str::contains("[FAIL]"));
}

#[test]
fn test_cli_check_fails_on_unrecognized_meta_field() {
  // Regression test for the exact bug that motivated
  // `#[serde(deny_unknown_fields)]` on the model structs: an old
  // `meta.role` (the model now expects `meta.title`) must fail loudly
  // instead of silently vanishing.
  let temp = TempDir::new().unwrap();
  let content_file = temp.path().join("content.yaml");
  fs::write(
    &content_file,
    r#"
meta:
  name: "Jane Doe"
  version: "1.0.0"
  role: "Staff Engineer"
  contact:
    - name: "jane@example.com"
sections: []
"#,
  )
  .unwrap();

  Command::cargo_bin("rsmk")
    .unwrap()
    .current_dir(temp.path())
    .arg("check")
    .arg("--content")
    .arg(&content_file)
    .assert()
    .failure()
    .stderr(predicate::str::contains("role"));
}

#[test]
fn test_cli_check_succeeds_with_meta_extra() {
  if which::which("typst").is_err() {
    eprintln!(
      "skipping test_cli_check_succeeds_with_meta_extra: typst not on PATH"
    );
    return;
  }

  let temp = TempDir::new().unwrap();
  let content_file = temp.path().join("content.yaml");
  fs::write(
    &content_file,
    r#"
meta:
  name: "Jane Doe"
  version: "1.0.0"
  contact:
    - name: "jane@example.com"
  extra:
    clearance: "Secret"
    relocation: false
    custom_metrics:
      github_followers: 500
sections: []
"#,
  )
  .unwrap();

  Command::cargo_bin("rsmk")
    .unwrap()
    .current_dir(temp.path())
    .arg("check")
    .arg("--content")
    .arg(&content_file)
    .assert()
    .success()
    .stdout(predicate::str::contains("SUCCESS"));
}

#[test]
fn test_cli_template_list() {
  let temp = TempDir::new().unwrap();

  let mut cmd = Command::cargo_bin("rsmk").unwrap();
  cmd
    .current_dir(temp.path())
    .arg("template")
    .arg("list")
    .assert()
    .success()
    .stdout(predicate::str::contains("Available templates:"))
    .stdout(predicate::str::contains("classic (built-in, default)"));

  // Add custom template in ./templates/
  let custom_tpl_dir = temp.path().join("templates").join("custom_theme");
  fs::create_dir_all(&custom_tpl_dir).unwrap();

  let mut cmd2 = Command::cargo_bin("rsmk").unwrap();
  cmd2
    .current_dir(temp.path())
    .arg("template")
    .arg("list")
    .assert()
    .success()
    .stdout(predicate::str::contains("classic (built-in, default)"))
    .stdout(predicate::str::contains("custom_theme (custom)"));
}

#[test]
fn test_cli_template_eject_classic() {
  let temp = TempDir::new().unwrap();

  let mut cmd = Command::cargo_bin("rsmk").unwrap();
  cmd
    .current_dir(temp.path())
    .arg("template")
    .arg("eject")
    .arg("classic")
    .assert()
    .success()
    .stdout(predicate::str::contains(
      "✓ Ejected template 'classic' to ./templates/classic/",
    ))
    .stdout(predicate::str::contains("main.typ"))
    .stdout(predicate::str::contains("tokens.typ"))
    .stdout(predicate::str::contains("primitives.typ"))
    .stdout(predicate::str::contains("blocks/experience.typ"))
    .stdout(predicate::str::contains(
      "Run `rsmk build --template ./templates/classic/main.typ` to compile with your local template.",
    ));

  let target_dir = temp.path().join("templates").join("classic");
  assert!(target_dir.join("main.typ").exists());
  assert!(target_dir.join("tokens.typ").exists());
  assert!(target_dir.join("primitives.typ").exists());
  assert!(target_dir.join("blocks").join("experience.typ").exists());
}

#[test]
fn test_cli_template_eject_collision_without_force() {
  let temp = TempDir::new().unwrap();

  // First eject
  Command::cargo_bin("rsmk")
    .unwrap()
    .current_dir(temp.path())
    .arg("template")
    .arg("eject")
    .arg("classic")
    .assert()
    .success();

  // Second eject without force should fail
  Command::cargo_bin("rsmk")
    .unwrap()
    .current_dir(temp.path())
    .arg("template")
    .arg("eject")
    .arg("classic")
    .assert()
    .failure()
    .stderr(predicate::str::contains("already exists"))
    .stderr(predicate::str::contains("--force"));

  // Eject with force should succeed
  Command::cargo_bin("rsmk")
    .unwrap()
    .current_dir(temp.path())
    .arg("template")
    .arg("eject")
    .arg("classic")
    .arg("--force")
    .assert()
    .success()
    .stdout(predicate::str::contains("✓ Ejected template 'classic'"));
}

#[test]
fn test_cli_template_eject_unknown_template() {
  let temp = TempDir::new().unwrap();

  Command::cargo_bin("rsmk")
    .unwrap()
    .current_dir(temp.path())
    .arg("template")
    .arg("eject")
    .arg("nonexistent")
    .assert()
    .failure()
    .stderr(predicate::str::contains("Unknown template 'nonexistent'"));
}

#[test]
fn test_cli_build_and_check_flags_integration() {
  if which::which("typst").is_err() {
    eprintln!(
      "skipping test_cli_build_and_check_flags_integration: typst not on PATH"
    );
    return;
  }

  let temp = TempDir::new().unwrap();
  let content_file = temp.path().join("content.yaml");
  let output_pdf = temp.path().join("custom_resume.pdf");

  // 1. Scaffold content
  Command::cargo_bin("rsmk")
    .unwrap()
    .arg("init")
    .arg("--name")
    .arg("Jane Doe")
    .arg("--output")
    .arg(&content_file)
    .assert()
    .success();

  // 2. rsmk build (standard compilation to custom output)
  Command::cargo_bin("rsmk")
    .unwrap()
    .current_dir(temp.path())
    .arg("build")
    .arg("--content")
    .arg(&content_file)
    .arg("--output")
    .arg(&output_pdf)
    .assert()
    .success()
    .stdout(predicate::str::contains("SUCCESS"));

  assert!(output_pdf.exists());

  // 3. rsmk build --check
  Command::cargo_bin("rsmk")
    .unwrap()
    .current_dir(temp.path())
    .arg("build")
    .arg("--check")
    .arg("--content")
    .arg(&content_file)
    .assert()
    .success()
    .stdout(predicate::str::contains("[dry-run: no PDF written]"))
    .stdout(predicate::str::contains("Dry-run check passed"));

  // 4. rsmk build -c
  Command::cargo_bin("rsmk")
    .unwrap()
    .current_dir(temp.path())
    .arg("build")
    .arg("-c")
    .arg("--content")
    .arg(&content_file)
    .assert()
    .success()
    .stdout(predicate::str::contains("[dry-run: no PDF written]"))
    .stdout(predicate::str::contains("Dry-run check passed"));

  // 5. rsmk build --template classic
  Command::cargo_bin("rsmk")
    .unwrap()
    .current_dir(temp.path())
    .arg("build")
    .arg("--template")
    .arg("classic")
    .arg("--content")
    .arg(&content_file)
    .assert()
    .success()
    .stdout(predicate::str::contains("SUCCESS"));

  // 6. Legacy rsmk check
  Command::cargo_bin("rsmk")
    .unwrap()
    .current_dir(temp.path())
    .arg("check")
    .arg("--content")
    .arg(&content_file)
    .assert()
    .success()
    .stdout(predicate::str::contains("[dry-run: no PDF written]"))
    .stdout(predicate::str::contains("Dry-run check passed"));

  // 7. Bare rsmk with no subcommand defaults to build
  Command::cargo_bin("rsmk")
    .unwrap()
    .current_dir(temp.path())
    .assert()
    .success()
    .stdout(predicate::str::contains("SUCCESS"));
}

fn setup_git_repo_with_remote(
  dir: &std::path::Path,
  content_yaml: &str,
) -> std::path::PathBuf {
  let origin_dir = dir.join("remote.git");
  let work_dir = dir.join("repo");
  fs::create_dir_all(&origin_dir).unwrap();
  fs::create_dir_all(&work_dir).unwrap();

  // 1. Init bare remote
  std::process::Command::new("git")
    .arg("init")
    .arg("--bare")
    .current_dir(&origin_dir)
    .output()
    .unwrap();

  // 2. Init work repo
  std::process::Command::new("git")
    .arg("init")
    .current_dir(&work_dir)
    .output()
    .unwrap();
  std::process::Command::new("git")
    .args(["config", "user.name", "Test User"])
    .current_dir(&work_dir)
    .output()
    .unwrap();
  std::process::Command::new("git")
    .args(["config", "user.email", "test@example.com"])
    .current_dir(&work_dir)
    .output()
    .unwrap();
  std::process::Command::new("git")
    .args(["config", "commit.gpgsign", "false"])
    .current_dir(&work_dir)
    .output()
    .unwrap();

  // Add remote origin
  let origin_path_str = origin_dir.to_str().unwrap();
  std::process::Command::new("git")
    .args(["remote", "add", "origin", origin_path_str])
    .current_dir(&work_dir)
    .output()
    .unwrap();

  // Write content.yaml
  let content_file = work_dir.join("content.yaml");
  fs::write(&content_file, content_yaml).unwrap();

  // Commit and push
  std::process::Command::new("git")
    .args(["add", "."])
    .current_dir(&work_dir)
    .output()
    .unwrap();
  std::process::Command::new("git")
    .args(["commit", "-m", "chore: initial commit"])
    .current_dir(&work_dir)
    .output()
    .unwrap();
  std::process::Command::new("git")
    .args(["branch", "-M", "main"])
    .current_dir(&work_dir)
    .output()
    .unwrap();
  std::process::Command::new("git")
    .args(["push", "-u", "origin", "main"])
    .current_dir(&work_dir)
    .output()
    .unwrap();

  work_dir
}

const SAMPLE_CONTENT: &str = r#"
meta:
  name: "Jane Doe"
  version: "1.2.0"
  contact:
    - name: "jane@example.com"
sections: []
"#;

#[test]
fn test_cli_release_dry_run_success() {
  let temp = TempDir::new().unwrap();
  let work_dir = setup_git_repo_with_remote(temp.path(), SAMPLE_CONTENT);

  // Add prior tag v1.1.0
  std::process::Command::new("git")
    .args(["tag", "-a", "v1.1.0", "-m", "v1.1.0"])
    .current_dir(&work_dir)
    .output()
    .unwrap();
  std::process::Command::new("git")
    .args(["push", "origin", "v1.1.0"])
    .current_dir(&work_dir)
    .output()
    .unwrap();

  Command::cargo_bin("rsmk")
    .unwrap()
    .current_dir(&work_dir)
    .arg("release")
    .arg("--dry-run")
    .arg("--skip-build")
    .assert()
    .success()
    .stdout(predicate::str::contains("Résumé Release v1.2.0"))
    .stdout(predicate::str::contains("working tree clean"))
    .stdout(predicate::str::contains(
      "upstream branch synced (nothing unpushed)",
    ))
    .stdout(predicate::str::contains("v1.2.0 is new, ahead of v1.1.0"))
    .stdout(predicate::str::contains(
      "pre-flight check skipped (--skip-build)",
    ));

  // Verify tag v1.2.0 was not created in dry-run
  let tags_out = std::process::Command::new("git")
    .args(["tag", "-l"])
    .current_dir(&work_dir)
    .output()
    .unwrap();
  let tags_str = String::from_utf8_lossy(&tags_out.stdout);
  assert!(!tags_str.contains("v1.2.0"));
}

#[test]
fn test_cli_release_dirty_working_tree_fails() {
  let temp = TempDir::new().unwrap();
  let work_dir = setup_git_repo_with_remote(temp.path(), SAMPLE_CONTENT);

  // Dirty the tree with an untracked file
  fs::write(work_dir.join("untracked.txt"), "dirty").unwrap();

  Command::cargo_bin("rsmk")
    .unwrap()
    .current_dir(&work_dir)
    .arg("release")
    .arg("--dry-run")
    .arg("--skip-build")
    .assert()
    .failure()
    .stderr(predicate::str::contains("uncommitted changes"));
}

#[test]
fn test_cli_release_invalid_semver_fails() {
  let temp = TempDir::new().unwrap();
  let invalid_content = r#"
meta:
  name: "Jane Doe"
  version: "not-a-semver"
  contact:
    - name: "jane@example.com"
sections: []
"#;
  let work_dir = setup_git_repo_with_remote(temp.path(), invalid_content);

  Command::cargo_bin("rsmk")
    .unwrap()
    .current_dir(&work_dir)
    .arg("release")
    .arg("--dry-run")
    .arg("--skip-build")
    .assert()
    .failure()
    .stderr(predicate::str::contains("Invalid semver"));
}

#[test]
fn test_cli_release_semver_monotonicity_fails() {
  let temp = TempDir::new().unwrap();
  let work_dir = setup_git_repo_with_remote(temp.path(), SAMPLE_CONTENT); // v1.2.0

  // Add higher tag v1.3.0
  std::process::Command::new("git")
    .args(["tag", "-a", "v1.3.0", "-m", "v1.3.0"])
    .current_dir(&work_dir)
    .output()
    .unwrap();
  std::process::Command::new("git")
    .args(["push", "origin", "v1.3.0"])
    .current_dir(&work_dir)
    .output()
    .unwrap();

  Command::cargo_bin("rsmk")
    .unwrap()
    .current_dir(&work_dir)
    .arg("release")
    .arg("--dry-run")
    .arg("--skip-build")
    .assert()
    .failure()
    .stderr(predicate::str::contains("semver monotonicity check failed"));
}

#[test]
fn test_cli_release_unpushed_commits_fails() {
  let temp = TempDir::new().unwrap();
  let work_dir = setup_git_repo_with_remote(temp.path(), SAMPLE_CONTENT);

  // Make an unpushed commit
  fs::write(work_dir.join("note.txt"), "hello").unwrap();
  std::process::Command::new("git")
    .args(["add", "."])
    .current_dir(&work_dir)
    .output()
    .unwrap();
  std::process::Command::new("git")
    .args(["commit", "-m", "unpushed"])
    .current_dir(&work_dir)
    .output()
    .unwrap();

  Command::cargo_bin("rsmk")
    .unwrap()
    .current_dir(&work_dir)
    .arg("release")
    .arg("--dry-run")
    .arg("--skip-build")
    .assert()
    .failure()
    .stderr(predicate::str::contains("unpushed commit"));
}

#[test]
fn test_cli_release_no_upstream_fails() {
  let temp = TempDir::new().unwrap();
  let work_dir = temp.path().join("local_repo");
  fs::create_dir_all(&work_dir).unwrap();

  std::process::Command::new("git")
    .arg("init")
    .current_dir(&work_dir)
    .output()
    .unwrap();
  std::process::Command::new("git")
    .args(["config", "user.name", "Test User"])
    .current_dir(&work_dir)
    .output()
    .unwrap();
  std::process::Command::new("git")
    .args(["config", "user.email", "test@example.com"])
    .current_dir(&work_dir)
    .output()
    .unwrap();
  std::process::Command::new("git")
    .args(["config", "commit.gpgsign", "false"])
    .current_dir(&work_dir)
    .output()
    .unwrap();

  fs::write(work_dir.join("content.yaml"), SAMPLE_CONTENT).unwrap();
  std::process::Command::new("git")
    .args(["add", "."])
    .current_dir(&work_dir)
    .output()
    .unwrap();
  std::process::Command::new("git")
    .args(["commit", "-m", "initial"])
    .current_dir(&work_dir)
    .output()
    .unwrap();

  Command::cargo_bin("rsmk")
    .unwrap()
    .current_dir(&work_dir)
    .arg("release")
    .arg("--dry-run")
    .arg("--skip-build")
    .assert()
    .failure()
    .stderr(predicate::str::contains("upstream"));
}

#[test]
fn test_cli_release_actual_tag_and_push_success() {
  let temp = TempDir::new().unwrap();
  let work_dir = setup_git_repo_with_remote(temp.path(), SAMPLE_CONTENT);

  Command::cargo_bin("rsmk")
    .unwrap()
    .current_dir(&work_dir)
    .arg("release")
    .arg("--skip-build")
    .arg("-m")
    .arg("Release v1.2.0")
    .assert()
    .success()
    .stdout(predicate::str::contains("created tag v1.2.0"))
    .stdout(predicate::str::contains("pushed tag to origin"))
    .stdout(predicate::str::contains("Release workflow triggered:"));

  // Verify tag v1.2.0 exists in work repo
  let tags_out = std::process::Command::new("git")
    .args(["tag", "-l"])
    .current_dir(&work_dir)
    .output()
    .unwrap();
  let tags_str = String::from_utf8_lossy(&tags_out.stdout);
  assert!(tags_str.contains("v1.2.0"));

  // Verify tag v1.2.0 exists in bare remote
  let remote_tags_out = std::process::Command::new("git")
    .args(["tag", "-l"])
    .current_dir(temp.path().join("remote.git"))
    .output()
    .unwrap();
  let remote_tags_str = String::from_utf8_lossy(&remote_tags_out.stdout);
  assert!(remote_tags_str.contains("v1.2.0"));
}
