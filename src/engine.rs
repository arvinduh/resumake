pub mod error;
pub mod templates;
pub mod world;

pub use error::EngineError;
pub use templates::{
  eject_template, embedded_templates, find_embedded_template,
  known_template_names, list_templates, list_templates_in, EmbeddedTemplate,
  TemplateFile, TemplateInfo, DEFAULT_TEMPLATE,
};
pub use world::{discover_font_dir, format_diagnostics, ResumakeWorld};

use crate::schema::validate_schema_auto;
use crate::telemetry::{evaluate_telemetry, TelemetryReport};
use std::fs;
use std::path::{Path, PathBuf};
use typst::foundations::{Label, Selector};
use typst::layout::PagedDocument;
use typst::utils::PicoStr;
use typst::Document;

/// The core in-process Typst compiler facade.
pub struct TypstEngine {
  /// User override or auto-discovered font directory.
  pub font_path: Option<PathBuf>,
  /// The project root directory.
  pub root_path: PathBuf,
}

impl TypstEngine {
  /// Constructs a new [`TypstEngine`].
  ///
  /// # Errors
  /// Returns [`EngineError`] if font directory search fails.
  pub fn new(font_override: Option<&Path>) -> Result<Self, EngineError> {
    let root_path = crate::utils::fs::find_project_root();
    let font_path = discover_font_dir(&root_path, font_override)?;
    Ok(Self {
      font_path,
      root_path,
    })
  }

  /// Resolves the Typst entry file from `--template` and optional `--source`.
  ///
  /// # Errors
  /// Returns [`EngineError::TemplateNotFound`] if a named template is unknown.
  pub fn resolve_template(
    &self,
    template_name: &str,
    source: Option<&Path>,
  ) -> Result<PathBuf, EngineError> {
    if let Some(src) = source {
      if src.is_dir() {
        return Ok(src.join("main.typ"));
      }
      return Ok(src.to_path_buf());
    }

    if template_name.ends_with(".typ") {
      let direct = PathBuf::from(template_name);
      if direct.is_file() {
        return Ok(direct);
      }
      let joined = self.root_path.join(template_name);
      if joined.is_file() {
        return Ok(joined);
      }
    }

    let custom_dir = self.root_path.join("templates").join(template_name);
    if custom_dir.is_dir() {
      let custom_main = custom_dir.join("main.typ");
      if custom_main.is_file() {
        return Ok(custom_main);
      }
    }

    let custom_single = self
      .root_path
      .join("templates")
      .join(format!("{template_name}.typ"));
    if custom_single.is_file() {
      return Ok(custom_single);
    }

    if let Some(_template) = find_embedded_template(template_name) {
      return Ok(PathBuf::from(format!("{template_name}/main.typ")));
    }

    Err(EngineError::TemplateNotFound {
      name: template_name.to_string(),
      known: known_template_names(),
    })
  }

  /// Compiles a document to a [`PagedDocument`] in-memory.
  ///
  /// # Errors
  /// Returns [`EngineError`] if Typst compilation produces errors.
  pub fn compile_paged(
    &self,
    template_path: &Path,
    content_path: &Path,
  ) -> Result<PagedDocument, EngineError> {
    let world = ResumakeWorld::new(
      self.root_path.clone(),
      template_path.to_path_buf(),
      content_path.to_path_buf(),
      self.font_path.clone(),
    )?;

    let result = typst::compile(&world);
    let doc = result.output.map_err(|diags| {
      let stderr = format_diagnostics(&world, &diags);
      EngineError::CompilationFailed { stderr }
    })?;

    Ok(doc)
  }

  /// Compiles a document to PDF bytes and writes to `output_path`.
  ///
  /// # Errors
  /// Returns [`EngineError`] if compilation fails or writing to disk fails.
  pub fn compile(
    &self,
    template_path: &Path,
    content_path: &Path,
    output_path: &Path,
  ) -> Result<(), EngineError> {
    let doc = self.compile_paged(template_path, content_path)?;
    let pdf_bytes = typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default())
      .map_err(|diags| {
        let world = ResumakeWorld::new(
          self.root_path.clone(),
          template_path.to_path_buf(),
          content_path.to_path_buf(),
          self.font_path.clone(),
        )
        .unwrap();
        let stderr = format_diagnostics(&world, &diags);
        EngineError::CompilationFailed { stderr }
      })?;

    if let Some(parent) = output_path.parent() {
      fs::create_dir_all(parent)?;
    }
    fs::write(output_path, pdf_bytes)?;

    Ok(())
  }

  /// Compiles document in-memory and queries metadata values for `selector`.
  ///
  /// # Errors
  /// Returns [`EngineError`] if compilation fails or query fails.
  pub fn query_metadata(
    &self,
    template_path: &Path,
    content_path: &Path,
    selector: &str,
  ) -> Result<String, EngineError> {
    let doc = self.compile_paged(template_path, content_path)?;
    query_doc_metadata(&doc, selector)
  }
}

/// Queries metadata value(s) matching `selector` from a [`PagedDocument`] and serializes to JSON.
///
/// # Errors
/// Returns [`EngineError::QueryFailed`] if serialization fails.
pub fn query_doc_metadata(
  doc: &PagedDocument,
  selector: &str,
) -> Result<String, EngineError> {
  let label_str = selector
    .trim()
    .trim_start_matches('<')
    .trim_end_matches('>');
  let label = Label::new(PicoStr::intern(label_str));
  let sel = Selector::Label(label);
  let elems = doc.introspector().query(&sel);

  let mut values = Vec::new();
  for elem in elems {
    if let Some(metadata) =
      elem.to_packed::<typst::introspection::MetadataElem>()
    {
      values.push(metadata.value.clone());
    } else if let Ok(val) = elem.get_by_name("value") {
      values.push(val);
    }
  }

  serde_json::to_string(&values).map_err(|e| EngineError::QueryFailed {
    stderr: e.to_string(),
  })
}

/// Runs a fast, complete layout and content verification check on a document.
///
/// # Errors
/// Returns an [`EngineError`] if validation or compilation fails.
pub fn verify_content(
  content: &Path,
  template_name: &str,
  source: Option<&Path>,
  schema: Option<&Path>,
  font_path: Option<&Path>,
) -> Result<TelemetryReport, EngineError> {
  if !content.exists() {
    return Err(EngineError::ContentNotFound {
      path: content.to_path_buf(),
    });
  }

  validate_schema_auto(content, schema)?;

  let engine = TypstEngine::new(font_path)?;
  let resolved_template = engine.resolve_template(template_name, source)?;
  let doc = engine.compile_paged(&resolved_template, content)?;
  let page_json = query_doc_metadata(&doc, "<pageinfo>")?;
  let bullets_json = query_doc_metadata(&doc, "<bulletinfo>")?;
  let report = evaluate_telemetry(&page_json, &bullets_json)?;

  if !report.is_pass() {
    return Err(EngineError::LayoutConstraintViolation);
  }

  Ok(report)
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::TempDir;

  #[test]
  fn test_resolve_template_builtin_classic() {
    let temp = TempDir::new().unwrap();
    let engine = TypstEngine {
      font_path: None,
      root_path: temp.path().to_path_buf(),
    };

    let resolved = engine.resolve_template(DEFAULT_TEMPLATE, None).unwrap();
    assert_eq!(resolved, PathBuf::from("classic/main.typ"));
  }

  #[test]
  fn test_resolve_template_rejects_unknown_name() {
    let temp = TempDir::new().unwrap();
    let engine = TypstEngine {
      font_path: None,
      root_path: temp.path().to_path_buf(),
    };

    let err = engine.resolve_template("does-not-exist", None).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("does-not-exist"));
    assert!(msg.contains("classic"));
  }

  #[test]
  fn test_embedded_templates_discovery() {
    let templates = embedded_templates();
    assert!(!templates.is_empty());
    let classic = templates
      .iter()
      .find(|t| t.name == "classic")
      .expect("classic template must be found");
    assert_eq!(classic.name, "classic");
    assert!(!classic.entry.is_empty());
    assert!(classic.files.iter().any(|f| f.rel_path == "tokens.typ"));
    assert!(classic.files.iter().any(|f| f.rel_path == "primitives.typ"));
    assert!(classic
      .files
      .iter()
      .any(|f| f.rel_path == "blocks/experience.typ"));
  }

  #[test]
  fn test_all_block_files_are_registered_in_main_typ() {
    let blocks = [
      "education",
      "experience",
      "projects",
      "skills",
      "publications",
      "split_line",
      "references",
      "lines",
    ];

    let classic =
      find_embedded_template("classic").expect("classic template must exist");
    for block in blocks {
      assert!(
        classic.entry.contains(&format!("blocks/{block}.typ")),
        "Block '{block}' is missing an #import in main.typ!"
      );
    }
  }

  #[test]
  fn test_list_templates_builtins_and_custom() {
    let temp = TempDir::new().unwrap();
    let templates_dir = temp.path().join("templates");

    let list = list_templates_in(&templates_dir);
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

    let list2 = list_templates_in(&templates_dir);
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

    let files = eject_template("classic", &target, false).unwrap();
    assert!(files.contains(&"main.typ".to_string()));
    assert!(files.contains(&"tokens.typ".to_string()));
    assert!(files.contains(&"primitives.typ".to_string()));

    let err = eject_template("classic", &target, false).unwrap_err();
    assert!(matches!(err, EngineError::DestinationAlreadyExists { .. }));

    let files_force = eject_template("classic", &target, true).unwrap();
    assert_eq!(files, files_force);
  }

  #[test]
  fn test_eject_template_rejects_unknown_name() {
    let temp = TempDir::new().unwrap();
    let target = temp.path().join("templates").join("fake");
    let err = eject_template("fake", &target, false).unwrap_err();
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

    let engine = TypstEngine {
      font_path: None,
      root_path: root.to_path_buf(),
    };
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

    let report = evaluate_telemetry(&page_json, &bullets_json)
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

    let engine = TypstEngine {
      font_path: None,
      root_path: root.to_path_buf(),
    };
    let output_pdf = root.join("out.pdf");

    let err = engine
      .compile(&broken_template, &content_path, &output_pdf)
      .unwrap_err();

    let msg = err.to_string();
    assert!(msg.contains("Typst compilation failed:"));
  }
}
