//! Handlers for `rsmk build`.

use std::path::{Path, PathBuf};

use resumake::{engine, schema, telemetry};

use crate::cli::{ui, Error};

/// Command line arguments for `rsmk build`.
#[derive(Debug, Clone, PartialEq, Eq, clap::Args)]
pub struct BuildArgs {
  /// Path to content YAML file
  #[arg(value_name = "CONTENT", default_value = "content.yaml")]
  pub content: PathBuf,

  /// Named built-in layout, local template directory, or .typ file to render
  /// with
  #[arg(short, long)]
  pub template: Option<String>,

  /// Custom output PDF path (defaults to `<name>_resume.pdf`)
  #[arg(short, long)]
  pub output: Option<PathBuf>,

  /// Path to custom JSON schema file (falls back to built-in schema if
  /// omitted)
  #[arg(long)]
  pub schema: Option<PathBuf>,

  /// Custom font directory (auto-detects ./fonts if present)
  #[arg(long)]
  pub font_path: Option<PathBuf>,
}

impl Default for BuildArgs {
  fn default() -> Self {
    Self {
      content: PathBuf::from("content.yaml"),
      template: None,
      output: None,
      schema: None,
      font_path: None,
    }
  }
}

/// Compiles the document, optionally renders PDF, evaluates layout telemetry,
/// and prints report.
pub(crate) fn compile_and_evaluate(
  content: &Path,
  template_name: &str,
  schema: Option<&Path>,
  font_path: Option<&Path>,
  output_pdf: Option<&Path>,
  quiet: bool,
) -> Result<(), Error> {
  if !content.exists() {
    return Err(
      engine::Error::ContentNotFound {
        path: content.to_path_buf(),
      }
      .into(),
    );
  }

  // 1. Validate schema
  schema::validate_schema_auto(content, schema)?;

  // 2. Resolve paths and compile document once in-memory
  let engine = engine::TypstEngine::new(font_path)?;
  let resolved_template = engine.resolve_template(template_name)?;
  let doc = engine.compile_paged(&resolved_template, content)?;

  // 3. Render PDF directly from the in-memory document if requested
  if let Some(out) = output_pdf {
    engine.render_pdf(&doc, out)?;
  }

  // 4. Query telemetry directly from the same in-memory document
  let page_json = engine::query_doc_metadata(&doc, "<pageinfo>")?;
  let bullets_json = engine::query_doc_metadata(&doc, "<bulletinfo>")?;
  let report = telemetry::evaluate_telemetry(&page_json, &bullets_json)?;

  let name = schema::load_content_name(content)
    .unwrap_or_else(|_| "Candidate".to_string());
  let version = schema::load_content_version(content)
    .unwrap_or_else(|_| "1.0.0".to_string());

  let output_display = match output_pdf {
    Some(out) => out.to_string_lossy(),
    None => "[dry-run: no PDF written]".into(),
  };

  if !quiet {
    ui::print_telemetry_table(&report, &name, &output_display, &version);
  }

  if !report.is_pass() {
    return Err(engine::Error::LayoutConstraintViolation.into());
  }

  Ok(())
}

impl BuildArgs {
  /// Runs the build command.
  pub fn run(self, quiet: bool) -> Result<(), Error> {
    let template_name = self
      .template
      .as_deref()
      .unwrap_or(engine::templates::DEFAULT_TEMPLATE);
    let output_pdf = match self.output {
      Some(out) => out,
      None => schema::derive_output_filename(&self.content),
    };
    compile_and_evaluate(
      &self.content,
      template_name,
      self.schema.as_deref(),
      self.font_path.as_deref(),
      Some(&output_pdf),
      quiet,
    )
  }
}
