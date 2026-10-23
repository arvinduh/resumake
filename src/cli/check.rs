//! Handlers for `rsmk check`.

use std::path::PathBuf;

use resumake::engine;

use crate::cli::build::compile_and_evaluate;
use crate::cli::Error;

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
  pub fn run(self, quiet: bool) -> Result<(), Error> {
    let template_name = self
      .template
      .as_deref()
      .unwrap_or(engine::templates::DEFAULT_TEMPLATE);
    compile_and_evaluate(
      &self.content,
      template_name,
      self.schema.as_deref(),
      self.font_path.as_deref(),
      None,
      quiet,
    )
  }
}
