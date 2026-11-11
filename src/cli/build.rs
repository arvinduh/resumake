//! Handlers for `rsmk build`.
//!
//! Owns argument definitions for compiling résumé documents to PDF and
//! verifying layout telemetry.

use std::path;

use clap;

use crate::cli;

/// Command line arguments for `rsmk build`.
#[derive(clap::Args, Default)]
pub struct BuildArgs {
  /// Manifest doc name (e.g. `main`, `ml`) or content file path
  #[arg(value_name = "TARGET", conflicts_with = "all")]
  pub target: Option<String>,

  /// Build all documents in `resumake.toml` regardless of `publish`
  #[arg(long)]
  pub all: bool,

  /// Custom output PDF path when building a single target
  #[arg(short, long, conflicts_with = "out_dir")]
  pub output: Option<path::PathBuf>,

  /// Output directory for built PDFs (overrides `[build].out_dir`)
  #[arg(long, value_name = "DIR")]
  pub out_dir: Option<path::PathBuf>,

  /// Shared template, schema, font, and telemetry flags
  #[command(flatten)]
  pub options: cli::run::PipelineOptions,
}

impl BuildArgs {
  /// Runs the build command.
  ///
  /// # Errors
  /// Returns [`cli::Error`] if target resolution, schema validation, Typst
  /// compilation, or layout telemetry checks fail.
  pub fn run(self, quiet: bool) -> Result<(), cli::Error> {
    cli::run::run(
      self.target.as_deref(),
      &self.options,
      cli::run::Mode::Build {
        output: self.output.as_deref(),
        out_dir: self.out_dir.as_deref(),
        all: self.all,
      },
      quiet,
    )
  }
}
