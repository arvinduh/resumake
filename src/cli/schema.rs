//! Handler for `rsmk schema`.
//!
//! Owns argument definitions and execution for exporting the canonical JSON
//! schema to stdout or a destination file.

use std::fs;
use std::path;

use clap;
use resumake;

use crate::cli;

/// Command line arguments for `rsmk schema`.
#[derive(clap::Args)]
pub struct SchemaArgs {
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
    let schema_val = resumake::schema::generate_schema();
    let schema_str = serde_json::to_string_pretty(&schema_val)?;

    if let Some(dest) = self.output {
      if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
      }
      fs::write(dest, schema_str)?;
    } else {
      println!("{schema_str}");
    }

    Ok(())
  }
}
