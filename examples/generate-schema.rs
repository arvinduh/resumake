//! Generates the canonical `resume.schema.json` from the Serde models in
//! `src/models.rs`. The file is never committed: the release workflows run
//! this example to produce the schema fresh from the current types and
//! publish it as a GitHub Release asset, so it cannot drift from the source.

use resumake::schema::export_builtin_schema;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
  // Anchor to the crate root so the file always lands there regardless of
  // the working directory the workflow invokes this from.
  let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("resume.schema.json");

  let schema = match export_builtin_schema(None) {
    Ok(schema) => schema,
    Err(err) => {
      eprintln!("error: failed to generate schema: {err}");
      return ExitCode::FAILURE;
    }
  };

  // Normalize to exactly one trailing LF newline for a stable asset.
  let normalized = format!("{}\n", schema.trim_end());

  if let Err(err) = fs::write(&target, normalized.as_bytes()) {
    eprintln!("error: failed to write {}: {err}", target.display());
    return ExitCode::FAILURE;
  }

  println!("Wrote canonical JSON schema to {}", target.display());
  ExitCode::SUCCESS
}
