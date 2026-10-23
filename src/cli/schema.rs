//! Handler for `rsmk schema`.

use std::path::PathBuf;

use resumake::schema;

use crate::cli::Error;

/// Command line arguments for `rsmk schema`.
#[derive(Debug, Clone, PartialEq, Eq, clap::Args)]
pub struct SchemaArgs {
  /// Destination path for the generated schema JSON (prints to stdout if
  /// omitted)
  #[arg(short, long)]
  pub output: Option<PathBuf>,
}

impl SchemaArgs {
  /// Runs the schema export command.
  pub fn run(self, _quiet: bool) -> Result<(), Error> {
    let schema_str = schema::export_builtin_schema(self.output.as_deref())?;
    if self.output.is_none() {
      println!("{schema_str}");
    }
    Ok(())
  }
}
