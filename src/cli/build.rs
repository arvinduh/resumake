//! Handlers for `rsmk build`.
//!
//! Owns argument definitions and pipeline execution for compiling a résumé to
//! PDF and verifying layout telemetry.

use std::fs;
use std::path;

use clap;
use resumake;

use crate::cli;

/// Command line arguments for `rsmk build`.
#[derive(clap::Args)]
pub struct BuildArgs {
  /// Path to content YAML file
  #[arg(value_name = "CONTENT", default_value = "content.yaml")]
  pub content: path::PathBuf,

  /// Named built-in layout, local template directory, or .typ file to render
  /// with
  #[arg(short, long)]
  pub template: Option<String>,

  /// Custom output PDF path (defaults to `<name>_resume.pdf`)
  #[arg(short, long)]
  pub output: Option<path::PathBuf>,

  /// Path to custom JSON schema file (falls back to built-in schema if
  /// omitted)
  #[arg(long)]
  pub schema: Option<path::PathBuf>,

  /// Custom font directory (auto-detects ./fonts if present)
  #[arg(long)]
  pub font_path: Option<path::PathBuf>,

  /// Most pages allowed before the build fails (default 1)
  #[arg(long)]
  pub max_pages: Option<usize>,
}

impl Default for BuildArgs {
  fn default() -> Self {
    Self {
      content: path::PathBuf::from("content.yaml"),
      template: None,
      output: None,
      schema: None,
      font_path: None,
      max_pages: None,
    }
  }
}

/// Derives default output PDF filename from candidate name in YAML content.
fn derive_output_name(content_path: &path::Path) -> path::PathBuf {
  if let Ok(content_str) = fs::read_to_string(content_path)
    && let Ok(meta) = resumake::schema::load_meta(&content_str)
  {
    let slug = meta
      .name
      .trim()
      .to_lowercase()
      .replace(|c: char| !c.is_alphanumeric(), "_");
    let clean = slug.trim_matches('_');
    if !clean.is_empty() {
      return path::PathBuf::from(format!("{clean}_resume.pdf"));
    }
  }
  path::PathBuf::from("resume.pdf")
}

impl BuildArgs {
  /// Runs the build command.
  ///
  /// # Errors
  /// Returns [`cli::Error`] if schema validation, Typst compilation, or layout
  /// constraint verification fails.
  pub fn run(self, quiet: bool) -> Result<(), cli::Error> {
    let output_pdf = self
      .output
      .unwrap_or_else(|| derive_output_name(&self.content));

    let project_root = cli::project_root(&self.content);

    resumake::schema::validate_file(&self.content, self.schema.as_deref())?;

    let (doc, report) = resumake::render::compile(
      &self.content,
      self.template.as_deref(),
      &project_root,
      self.font_path.as_deref(),
    )?;

    resumake::render::export_pdf(&doc, &output_pdf)?;

    if !quiet {
      cli::print_report(&report, &format!("built {}", output_pdf.display()));
    }

    cli::enforce_layout(&report, self.max_pages)
  }
}
