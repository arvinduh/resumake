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

  /// Most pages allowed before the check fails (default 1)
  #[arg(long)]
  pub max_pages: Option<usize>,
}

impl CheckArgs {
  /// Runs the check command.
  ///
  /// # Errors
  /// Returns [`cli::Error`] if schema validation, Typst compilation, or layout
  /// constraint verification fails.
  pub fn run(self, quiet: bool) -> Result<(), cli::Error> {
    let project_root = cli::project_root(&self.content);

    resumake::schema::validate_file(&self.content, self.schema.as_deref())?;

    let (_doc, report) = resumake::render::compile(
      &self.content,
      self.template.as_deref(),
      &project_root,
      self.font_path.as_deref(),
    )?;

    if !quiet {
      cli::print_report(&report, "ok");
    }

    cli::enforce_layout(&report, self.max_pages)
  }
}
