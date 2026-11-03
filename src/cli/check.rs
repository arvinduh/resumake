//! Handlers for `rsmk check`.
//!
//! Owns argument definitions and pipeline execution for verifying résumé
//! schema and layout geometry telemetry without writing a PDF.

use std::path;

use clap;
use resumake;

use crate::cli;

/// Command line arguments for `rsmk check`.
#[derive(clap::Args)]
pub struct CheckArgs {
  /// Path to content YAML file
  #[arg(value_name = "CONTENT", default_value = "content.yaml")]
  pub content: path::PathBuf,

  /// Named built-in layout, local template directory, or .typ file to render
  /// with
  #[arg(short, long)]
  pub template: Option<String>,

  /// Path to custom JSON schema file (falls back to built-in schema if
  /// omitted)
  #[arg(long)]
  pub schema: Option<path::PathBuf>,

  /// Custom font directory (auto-detects ./fonts if present)
  #[arg(long)]
  pub font_path: Option<path::PathBuf>,

  /// Fail check if document exceeds this many pages
  #[arg(long)]
  pub max_pages: Option<usize>,
}

impl CheckArgs {
  /// Runs the check command.
  ///
  /// # Errors
  /// Returns [`cli::Error`] if schema validation, Typst compilation, or layout
  /// constraint verification fails.
  #[expect(clippy::print_stderr, reason = "CLI error output")]
  pub fn run(self, quiet: bool) -> Result<(), cli::Error> {
    let project_root = self
      .content
      .parent()
      .filter(|p| !p.as_os_str().is_empty())
      .unwrap_or_else(|| path::Path::new("."));

    resumake::schema::validate_file(&self.content, self.schema.as_deref())?;

    let (_doc, report) = resumake::render::compile(
      &self.content,
      self.template.as_deref(),
      project_root,
      self.font_path.as_deref(),
    )?;

    if !quiet {
      cli::print_report(&report, "ok");
    }

    if let Some(max) = self.max_pages
      && report.page_count > max
    {
      eprintln!(
        "error: document spans {} pages (exceeds --max-pages {max})",
        report.page_count
      );
      return Err(
        std::io::Error::other("page count constraint exceeded").into(),
      );
    }

    Ok(())
  }
}
