//! Handler for `rsmk schema`.
//!
//! Owns argument definitions and execution for exporting the canonical JSON
//! schema to stdout or a destination file.

use std::fs;
use std::path;

use clap;
use resumake;

use crate::cli;

/// Schema target to generate and export.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum SchemaKind {
  /// Résumé content document schema (`schemas/content.schema.json`).
  Content,
  /// Project manifest schema (`schemas/manifest.schema.json`).
  Manifest,
}

/// Command line arguments for `rsmk schema`.
#[derive(clap::Args)]
pub struct SchemaArgs {
  /// Which JSON Schema to emit (`content` or `manifest`)
  #[arg(long, value_enum, default_value_t = SchemaKind::Content)]
  pub kind: SchemaKind,

  /// Destination path for the generated schema JSON (prints to stdout if
  /// omitted)
  #[arg(short, long)]
  pub output: Option<path::PathBuf>,
}

impl SchemaArgs {
  /// Runs the schema export command.
  ///
  /// # Errors
  /// Returns [`cli::Error`] if JSON serialization or file writing fails.
  #[expect(clippy::print_stdout, reason = "CLI output")]
  pub fn run(self, _quiet: bool) -> Result<(), cli::Error> {
    let schema_val = match self.kind {
      SchemaKind::Content => resumake::schema::generate_schema(),
      SchemaKind::Manifest => resumake::manifest::generate_schema(),
    };
    let mut schema_str = serde_json::to_string_pretty(&schema_val)?;
    schema_str.push('\n');

    if let Some(dest) = self.output {
      if let Some(parent) = dest.parent()
        && !parent.as_os_str().is_empty()
      {
        fs::create_dir_all(parent)?;
      }
      fs::write(dest, schema_str)?;
    } else {
      print!("{schema_str}");
    }

    Ok(())
  }
}
