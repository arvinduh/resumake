//! Integration tests for file-based schema validation and export.

use resumake::schema::{
  derive_output_filename, export_builtin_schema, load_content_name,
  load_content_version, validate_schema_auto,
};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_validate_schema_auto_reads_file() {
  let temp = TempDir::new().unwrap();
  let content_file = temp.path().join("content.yaml");

  fs::write(
    &content_file,
    r#"
meta:
  name: "Test User"
  version: "1.2.3"
  contact:
    - name: "test@example.com"
sections: []
"#,
  )
  .unwrap();

  assert!(validate_schema_auto(&content_file, None).is_ok());
}

#[test]
fn test_export_builtin_schema_writes_file() {
  let temp = TempDir::new().unwrap();
  let out_file = temp.path().join("schema.json");
  let res = export_builtin_schema(Some(&out_file));
  assert!(res.is_ok());
  assert!(out_file.exists());
  let content = fs::read_to_string(&out_file).unwrap();
  assert!(content.contains("ResumeDocument"));
}

#[test]
fn test_load_name_version_and_derive_filename_from_file() {
  let temp = TempDir::new().unwrap();
  let content_file = temp.path().join("content.yaml");

  let yaml = r#"
meta:
  name: "Jane Doe"
  version: "2.1.0"
sections: []
"#;
  fs::write(&content_file, yaml).unwrap();

  assert_eq!(load_content_name(&content_file).unwrap(), "Jane Doe");
  assert_eq!(load_content_version(&content_file).unwrap(), "2.1.0");
  assert_eq!(
    derive_output_filename(&content_file),
    PathBuf::from("janedoe_resume.pdf")
  );
}
