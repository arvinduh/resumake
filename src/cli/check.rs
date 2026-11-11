//! Handlers for `rsmk check`.
//!
//! Owns argument definitions for verifying résumé schemas and layout telemetry
//! without writing a PDF.

use clap;

use crate::cli;

/// Command line arguments for `rsmk check`.
#[derive(clap::Args)]
pub struct CheckArgs {
  /// Manifest doc name (e.g. `main`, `ml`) or content file path
  #[arg(value_name = "TARGET")]
  pub target: Option<String>,

  /// Shared template, schema, font, and telemetry flags
  #[command(flatten)]
  pub options: cli::run::PipelineOptions,
}

impl CheckArgs {
  /// Runs the check command.
  ///
  /// # Errors
  /// Returns [`cli::Error`] if target resolution, schema validation, Typst
  /// compilation, or layout telemetry checks fail.
  pub fn run(self, quiet: bool) -> Result<(), cli::Error> {
    cli::run::run(
      self.target.as_deref(),
      &self.options,
      cli::run::Mode::Check,
      quiet,
    )
  }
}
