//! End-to-end integration tests for resumake CLI.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_cli_help() {
  let mut cmd = Command::cargo_bin("resumake").unwrap();
  cmd
    .arg("--help")
    .assert()
    .success()
    .stdout(predicate::str::contains("resumake"))
    .stdout(predicate::str::contains("Compile"));
}

#[test]
fn test_cli_init_and_schema_export() {
  let temp = TempDir::new().unwrap();
  let content_file = temp.path().join("content.yaml");
  let schema_file = temp.path().join("schema.json");

  // 1. Test init
  let mut cmd_init = Command::cargo_bin("resumake").unwrap();
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

  // 2. Test schema export
  let mut cmd_schema = Command::cargo_bin("resumake").unwrap();
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
fn test_cli_check_detects_invalid_yaml() {
  let temp = TempDir::new().unwrap();
  let invalid_file = temp.path().join("invalid.yaml");
  fs::write(&invalid_file, "invalid: [yaml").unwrap();

  let mut cmd = Command::cargo_bin("resumake").unwrap();
  cmd
    .arg("check")
    .arg("--content")
    .arg(&invalid_file)
    .assert()
    .failure()
    .stderr(predicate::str::contains("[FAIL]"));
}
