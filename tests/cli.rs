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
    .stdout(predicate::str::contains("✓ Ejected template 'classic' to ./templates/classic/"))
    .stdout(predicate::str::contains("main.typ"))
    .stdout(predicate::str::contains("tokens.typ"))
    .stdout(predicate::str::contains("primitives.typ"))
    .stdout(predicate::str::contains("blocks/experience.typ"))
    .stdout(predicate::str::contains("Run `rsmk build --template ./templates/classic/main.typ` to compile with your local template."));

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
