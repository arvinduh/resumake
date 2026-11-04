//! Handlers for `rsmk template`.
//!
//! Owns argument definitions and CLI presentation for listing built-in résumé
//! templates and ejecting single-file Typst templates into user projects.

use std::path;

use clap;
use resumake;

use crate::cli;

/// Available CLI arguments for `rsmk template`.
#[derive(clap::Args)]
pub struct TemplateArgs {
  /// Template operation to execute
  #[command(subcommand)]
  pub command: TemplateCommand,
}

/// Subcommands for `rsmk template`.
#[derive(clap::Subcommand)]
pub enum TemplateCommand {
  /// List all available built-in templates
  List,
  /// Eject a single-file Typst template into templates/<name>.typ
  Eject(EjectArgs),
}

/// Arguments for `rsmk template eject`.
#[derive(clap::Args)]
pub struct EjectArgs {
  /// Name of the template to eject (defaults to "classic")
  #[arg(value_name = "NAME", default_value = "classic")]
  pub name: String,

  /// Overwrite destination file if it already exists
  #[arg(short, long)]
  pub force: bool,
}

impl TemplateArgs {
  /// Runs the template command.
  ///
  /// # Errors
  /// Returns [`cli::Error`] if template lookup or disk ejection fails.
  #[expect(clippy::print_stdout, reason = "CLI output")]
  pub fn run(self, quiet: bool) -> Result<(), cli::Error> {
    match self.command {
      TemplateCommand::List => {
        let available = resumake::templates::list();
        println!("Available templates:");
        for tpl in available {
          println!("  - {tpl}");
        }
        Ok(())
      }
      TemplateCommand::Eject(args) => {
        let current_dir = path::Path::new(".");
        let target_file =
          resumake::templates::eject(&args.name, current_dir, args.force)?;

        if !quiet {
          println!(
            "Ejected template '{}' to '{}'",
            args.name,
            target_file.display()
          );
          println!(
            "Run `rsmk build --template {}` to compile with your local template.",
            target_file.display()
          );
        }
        Ok(())
      }
    }
  }
}
