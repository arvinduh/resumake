//! Handlers for `rsmk build`.

use std::path::PathBuf;

use resumake::{engine, schema};

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

impl BuildArgs {
  /// Runs the build command.
  ///
  /// # Errors
  ///
  /// Returns [`Error`] if schema validation, Typst compilation, or layout
  /// constraint verification fails.
  pub fn run(self, quiet: bool) -> Result<(), Error> {
    let output_pdf = match self.output {
      Some(out) => out,
      None => schema::derive_output_filename(&self.content),
    };

    let options = engine::BuildOptions {
      content: &self.content,
      template: self.template.as_deref(),
      output: Some(&output_pdf),
      schema: self.schema.as_deref(),
      font_path: self.font_path.as_deref(),
    };

    let outcome = engine::build(&options)?;

    if !quiet {
      let output_display = output_pdf.to_string_lossy();
      ui::print_telemetry_table(
        &outcome.report,
        &outcome.candidate_name,
        &output_display,
        &outcome.candidate_version,
      );
    }

    if !outcome.is_pass() {
      return Err(engine::Error::LayoutConstraintViolation.into());
    }

    Ok(())
  }
}
