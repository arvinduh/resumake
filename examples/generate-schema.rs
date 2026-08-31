//! Regenerates the canonical `resume.schema.json` at the repository root.
//!
//! The schema is derived directly from the Serde models in `src/models.rs`,
//! so this binary is the single source of truth for the committed file.
//! Run it with `cargo run --example generate-schema` whenever the models
//! change; CI fails if the committed copy drifts (see the drift test in
//! `src/schema.rs`).

use resumake::schema::export_builtin_schema;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
  let target = Path::new("resume.schema.json");

  let schema = match export_builtin_schema(None) {
    Ok(schema) => schema,
    Err(err) => {
      eprintln!("error: failed to generate schema: {err}");
      return ExitCode::FAILURE;
    }
  };

  // Normalize to exactly one trailing LF newline so the committed file is
  // stable across platforms and editors.
  let normalized = format!("{}\n", schema.trim_end());

  if let Err(err) = fs::write(target, normalized.as_bytes()) {
    eprintln!("error: failed to write {}: {err}", target.display());
    return ExitCode::FAILURE;
  }

  println!("Wrote canonical JSON schema to {}", target.display());
  ExitCode::SUCCESS
}
