//! Integration and regression tests for Typst engine compilation, template
//! ejection, and layout telemetry.

use std::fs;
use std::path::PathBuf;

use resumake::{engine, telemetry};
use tempfile::TempDir;

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

  let engine = engine::TypstEngine::with_root(root.to_path_buf(), None);
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
fn test_eject_template() {
  let temp = TempDir::new().unwrap();
  let target = temp.path().join("templates").join("classic");

  let files =
    engine::templates::eject_template("classic", &target, false).unwrap();
  assert!(files.contains(&"main.typ".to_string()));
  assert!(files.contains(&"tokens.typ".to_string()));
  assert!(files.contains(&"primitives.typ".to_string()));

  // Collision rejection without force
  let err =
    engine::templates::eject_template("classic", &target, false).unwrap_err();
  assert!(matches!(
    err,
    engine::Error::DestinationAlreadyExists { .. }
  ));

  // Overwrite succeeds with force
  let files_force =
    engine::templates::eject_template("classic", &target, true).unwrap();
  assert_eq!(files, files_force);

  // Unknown template rejection
  let err_unknown = engine::templates::eject_template(
    "fake",
    &temp.path().join("templates").join("fake"),
    false,
  )
  .unwrap_err();
  assert!(matches!(
    err_unknown,
    engine::Error::TemplateNotFound { .. }
  ));
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

  let engine = engine::TypstEngine::with_root(root.to_path_buf(), None);
  let doc = engine
    .compile_paged(&PathBuf::from("classic/main.typ"), &content_path)
    .expect("multi-byte bullets must compile");
  let bullets_json = engine::query_doc_metadata(&doc, "<bulletinfo>")
    .expect("bulletinfo query must succeed");
  assert!(bullets_json.contains('é'));
}

#[test]
fn test_template_reports_wrapped_from_layout() {
  let temp = TempDir::new().unwrap();
  let root = temp.path();
  let content_path = root.join("content.yaml");

  // Bullets from one to sixty words: short ones fit on a line, long ones
  // break, and the flag must flip exactly once, from fits to wraps.
  let mut bullets = String::new();
  for n in 1..=60 {
    use std::fmt::Write as _;
    let _ =
      writeln!(bullets, "          - \"{}\"", "word ".repeat(n).trim_end());
  }
  let yaml = format!(
    "meta:\n  name: Test\nsections:\n  - title: Projects\n    projects:\n      - name: Demo\n        bullets:\n{bullets}"
  );
  fs::write(&content_path, yaml).unwrap();

  let engine = engine::TypstEngine::with_root(root.to_path_buf(), None);
  let doc = engine
    .compile_paged(&PathBuf::from("classic/main.typ"), &content_path)
    .expect("compilation must succeed");
  let page_json = engine::query_doc_metadata(&doc, "<pageinfo>").unwrap();
  let bullets_json = engine::query_doc_metadata(&doc, "<bulletinfo>").unwrap();
  let report = telemetry::evaluate_telemetry(&page_json, &bullets_json)
    .expect("telemetry must parse");

  let flags: Vec<bool> = report
    .all_bullets
    .iter()
    .map(|b| b.wrapped.expect("classic template must report `wrapped`"))
    .collect();
  let first_wrap = flags.iter().position(|&w| w).expect("long bullets wrap");
  assert!(first_wrap > 0, "a one-word bullet must fit");
  assert!(
    flags[first_wrap..].iter().all(|&w| w),
    "wrapping is monotonic"
  );
  assert!(report.all_bullets[first_wrap].fill >= 99.9);
  assert!(report.all_bullets[first_wrap - 1].fill <= 100.1);
  assert_eq!(report.line_wraps.len(), flags.len() - first_wrap);
}
