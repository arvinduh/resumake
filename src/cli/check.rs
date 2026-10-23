//! Handlers for `rsmk check`.

use std::path::PathBuf;

use resumake::engine;

use crate::cli::{ui, Error};

/// Command line arguments for `rsmk check`.
#[derive(Debug, Clone, PartialEq, Eq, clap::Args)]
pub struct CheckArgs {
  /// Path to content YAML file
  #[arg(value_name = "CONTENT", default_value = "content.yaml")]
  pub content: PathBuf,

  /// Named built-in layout, local template directory, or .typ file to render
  /// with
  #[arg(short, long)]
  pub template: Option<String>,

  /// Path to custom JSON schema file (falls back to built-in schema if
  /// omitted)
  #[arg(long)]
  pub schema: Option<PathBuf>,

  /// Custom font directory (auto-detects ./fonts if present)
  #[arg(long)]
  pub font_path: Option<PathBuf>,
}

impl CheckArgs {
  /// Runs the check command.
  ///
  /// # Errors
  ///
  /// Returns [`Error`] if schema validation, Typst compilation, or layout
  /// constraint verification fails.
  pub fn run(self, quiet: bool) -> Result<(), Error> {
    let options = engine::BuildOptions {
      content: &self.content,
      template: self.template.as_deref(),
      output: None,
      schema: self.schema.as_deref(),
      font_path: self.font_path.as_deref(),
    };

    let outcome = engine::build(&options)?;

    if !quiet {
      ui::print_telemetry_table(
        &outcome.report,
        &outcome.candidate_name,
        "[dry-run: no PDF written]",
        &outcome.candidate_version,
      );
    }

    if !outcome.is_pass() {
      return Err(engine::Error::LayoutConstraintViolation.into());
    }

    Ok(())
  }
}
