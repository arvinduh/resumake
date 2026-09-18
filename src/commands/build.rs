//! Handlers for `rsmk build` and check mode.

use crate::engine;
use crate::engine::error::EngineError;
use crate::engine::TypstEngine;
use crate::error::ResumakeError;
use crate::schema;
use crate::telemetry;
use crate::utils::ui;
use std::path::Path;

/// Compiles the document, optionally renders PDF, evaluates layout telemetry, and prints report.
fn compile_and_evaluate(
  content: &Path,
  template_name: &str,
  source: Option<&Path>,
  schema: Option<&Path>,
  font_path: Option<&Path>,
  output_pdf: Option<&Path>,
  quiet: bool,
) -> Result<(), ResumakeError> {
  if !content.exists() {
    return Err(
      EngineError::ContentNotFound {
        path: content.to_path_buf(),
      }
      .into(),
    );
  }

  // 1. Validate schema
  schema::validate_schema_auto(content, schema)?;

  // 2. Resolve paths and compile document once in-memory
  let engine = TypstEngine::new(font_path)?;
  let resolved_template = engine.resolve_template(template_name, source)?;
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
    return Err(EngineError::LayoutConstraintViolation.into());
  }

  Ok(())
}

/// Runs `rsmk build` to compile the document to a PDF and verify layout telemetry.
pub(crate) fn run_build(
  content: &Path,
  template_name: &str,
  source: Option<&Path>,
  output: Option<&Path>,
  schema: Option<&Path>,
  font_path: Option<&Path>,
  quiet: bool,
) -> Result<(), ResumakeError> {
  let output_pdf = match output {
    Some(out) => out.to_path_buf(),
    None => schema::derive_output_filename(content),
  };
  compile_and_evaluate(
    content,
    template_name,
    source,
    schema,
    font_path,
    Some(&output_pdf),
    quiet,
  )
}

/// Runs `rsmk build --check` to verify schema and layout geometry without generating a PDF.
pub(crate) fn run_check(
  content: &Path,
  template_name: &str,
  source: Option<&Path>,
  schema: Option<&Path>,
  font_path: Option<&Path>,
  quiet: bool,
) -> Result<(), ResumakeError> {
  compile_and_evaluate(
    content,
    template_name,
    source,
    schema,
    font_path,
    None,
    quiet,
  )?;

  if !quiet {
    ui::print_success(
      "Dry-run check passed: schema & single-page layout valid.",
    );
  }
  Ok(())
}
