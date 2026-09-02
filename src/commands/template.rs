//! Handlers for `rsmk template list` and `rsmk template eject`.

use crate::engine::templates::{eject_template, list_templates};
use crate::error::ResumakeError;
use std::path::Path;

/// Runs `rsmk template list`.
pub fn run_template_list() -> Result<(), ResumakeError> {
  let templates = list_templates();
  println!("Available templates:");
  for tpl in templates {
    println!("  - {tpl}");
  }
  Ok(())
}

/// Runs `rsmk template eject`.
pub fn run_template_eject(
  name: &str,
  force: bool,
  quiet: bool,
) -> Result<(), ResumakeError> {
  let target_dir = Path::new("templates").join(name);
  let ejected_files = eject_template(name, &target_dir, force)?;

  if !quiet {
    println!("✓ Ejected template '{name}' to ./templates/{name}/");
    for file in ejected_files {
      println!("  - {file}");
    }
    println!(
      "Run `rsmk build --template ./templates/{name}/main.typ` to compile with your local template."
    );
  }
  Ok(())
}
