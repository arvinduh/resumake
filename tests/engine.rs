//! Integration tests for Typst engine compilation, template ejection, and layout telemetry.

use resumake::engine::error::EngineError;
use resumake::engine::templates;
use resumake::engine::{query_doc_metadata, TypstEngine};
use resumake::telemetry;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_list_templates_builtins_and_custom() {
  let temp = TempDir::new().unwrap();
  let templates_dir = temp.path().join("templates");

  let list = templates::list_templates_in(&templates_dir);
  assert_eq!(list.len(), 1);
  assert_eq!(list[0].name, "classic");
  assert!(list[0].is_builtin);
  assert!(list[0].is_default);
  assert_eq!(list[0].to_string(), "classic (built-in, default)");

  fs::create_dir_all(templates_dir.join("modern")).unwrap();
  fs::create_dir_all(templates_dir.join("minimal")).unwrap();
  fs::write(templates_dir.join("single.typ"), "// single file\n").unwrap();
  fs::write(templates_dir.join("ignore.txt"), "text\n").unwrap();
  fs::create_dir_all(templates_dir.join(".hidden")).unwrap();

  let list2 = templates::list_templates_in(&templates_dir);
  assert_eq!(list2.len(), 4);
  assert_eq!(list2[0].name, "classic");
  assert!(list2[0].is_builtin);
  assert_eq!(list2[1].name, "minimal");
  assert!(!list2[1].is_builtin);
  assert_eq!(list2[2].name, "modern");
  assert!(!list2[2].is_builtin);
  assert_eq!(list2[3].name, "single");
  assert!(!list2[3].is_builtin);
}

#[test]
fn test_eject_template_success_and_collision_rejection() {
  let temp = TempDir::new().unwrap();
  let target = temp.path().join("templates").join("classic");

  let files = templates::eject_template("classic", &target, false).unwrap();
  assert!(files.contains(&"main.typ".to_string()));
  assert!(files.contains(&"tokens.typ".to_string()));
  assert!(files.contains(&"primitives.typ".to_string()));

  let err = templates::eject_template("classic", &target, false).unwrap_err();
  assert!(matches!(err, EngineError::DestinationAlreadyExists { .. }));

  let files_force =
    templates::eject_template("classic", &target, true).unwrap();
  assert_eq!(files, files_force);
}

#[test]
fn test_eject_template_rejects_unknown_name() {
  let temp = TempDir::new().unwrap();
  let target = temp.path().join("templates").join("fake");
  let err = templates::eject_template("fake", &target, false).unwrap_err();
  assert!(matches!(err, EngineError::TemplateNotFound { .. }));
}

#[test]
fn test_in_process_compilation_and_telemetry() {
  let temp = TempDir::new().unwrap();
  let root = temp.path();
  let content_path = root.join("content.yaml");

  let yaml = r#"
meta:
  name: "Dr. Alex Vance"
  version: "1.0.0"
  theme: "classic"
contact:
  email: "alex@blackmesa.org"
sections:
  - heading: "Experience"
    type: "experience"
    items:
      - title: "Senior Scientist"
        organization: "Black Mesa"
        dates: "2000 - Present"
        bullets:
          - "Led quantum teleportation experiments."
"#;
  fs::write(&content_path, yaml).unwrap();

  let engine = TypstEngine::with_root(root.to_path_buf(), None);
  let output_pdf = root.join("resume.pdf");

  engine
    .compile(
      &PathBuf::from("classic/main.typ"),
      &content_path,
      &output_pdf,
    )
    .expect("In-process Typst compilation must succeed");

  assert!(output_pdf.exists());
  assert!(fs::metadata(&output_pdf).unwrap().len() > 1000);

  let page_json = engine
    .query_metadata(
      &PathBuf::from("classic/main.typ"),
      &content_path,
      "<pageinfo>",
    )
    .expect("pageinfo query must succeed");
  let bullets_json = engine
    .query_metadata(
      &PathBuf::from("classic/main.typ"),
      &content_path,
      "<bulletinfo>",
    )
    .expect("bulletinfo query must succeed");

  let report = telemetry::evaluate_telemetry(&page_json, &bullets_json)
    .expect("Telemetry evaluation must succeed");
  assert!(report.is_pass());
  assert_eq!(report.page_count, 1);
}

#[test]
fn test_telemetry_truncates_multibyte_bullets_on_char_boundaries() {
  let temp = TempDir::new().unwrap();
  let root = temp.path();
  let content_path = root.join("content.yaml");

  // Two-byte characters from both an even and an odd byte offset, so the
  // 80-unit telemetry cutoff lands inside a character in one of them.
  let yaml = format!(
    "meta:\n  name: Test\nsections:\n  - title: Projects\n    projects:\n      - name: Demo\n        bullets:\n          - \"{}\"\n          - \"a{}\"\n",
    "é".repeat(60),
    "–×µ".repeat(20),
  );
  fs::write(&content_path, yaml).unwrap();

  let engine = TypstEngine::with_root(root.to_path_buf(), None);
  let doc = engine
    .compile_paged(&PathBuf::from("classic/main.typ"), &content_path)
    .expect("multi-byte bullets must compile");
  let bullets_json = query_doc_metadata(&doc, "<bulletinfo>")
    .expect("bulletinfo query must succeed");
  assert!(bullets_json.contains("é"));
}

#[test]
fn test_typst_engine_render_pdf_and_query_doc_metadata() {
  let temp = TempDir::new().unwrap();
  let root = temp.path();
  let content_path = root.join("content.yaml");
  let output_pdf = root.join("output.pdf");

  fs::write(&content_path, "meta:\n  name: Test\n").unwrap();

  let engine = TypstEngine::with_root(root.to_path_buf(), None);

  let doc = engine
    .compile_paged(&PathBuf::from("classic/main.typ"), &content_path)
    .expect("Paged compilation must succeed");

  engine
    .render_pdf(&doc, &output_pdf)
    .expect("PDF rendering must succeed");

  assert!(output_pdf.exists());
  assert!(fs::metadata(&output_pdf).unwrap().len() > 1000);

  let page_json = query_doc_metadata(&doc, "<pageinfo>")
    .expect("pageinfo query must succeed");
  let bullets_json = query_doc_metadata(&doc, "<bulletinfo>")
    .expect("bulletinfo query must succeed");

  let report = telemetry::evaluate_telemetry(&page_json, &bullets_json)
    .expect("Telemetry evaluation must succeed");
  assert!(report.is_pass());
  assert_eq!(report.page_count, 1);
}

#[test]
fn test_compilation_error_formatting() {
  let temp = TempDir::new().unwrap();
  let root = temp.path();
  let content_path = root.join("content.yaml");
  let broken_template = root.join("broken.typ");

  fs::write(&content_path, "meta:\n  name: Test\n").unwrap();
  fs::write(&broken_template, "#let invalid_syntax = (((").unwrap();

  let engine = TypstEngine::with_root(root.to_path_buf(), None);
  let output_pdf = root.join("out.pdf");

  let err = engine
    .compile(&broken_template, &content_path, &output_pdf)
    .unwrap_err();

  let msg = err.to_string();
  assert!(msg.contains("Typst compilation failed:"));
}
