//! Handlers for `rsmk template --list` and `rsmk template <NAME>`.

use std::path::Path;

use resumake::engine::templates;

use crate::cli::{ui, Error};

/// Available CLI arguments for `rsmk template`.
#[derive(Debug, Clone, PartialEq, Eq, clap::Args)]
pub struct TemplateArgs {
  /// Name of the template to eject (e.g. `classic`)
  #[arg(value_name = "NAME", conflicts_with = "list")]
  pub name: Option<String>,

  /// List all available built-in and discovered custom templates
  #[arg(long, conflicts_with = "name")]
  pub list: bool,

  /// Overwrite destination directory if it already exists when ejecting
  #[arg(short, long)]
  pub force: bool,
}

impl TemplateArgs {
  /// Runs the template command.
  pub fn run(self, quiet: bool) -> Result<(), Error> {
    if self.list {
      let available = templates::list_templates();
      println!("Available templates:");
      for tpl in available {
        println!("  - {tpl}");
      }
      Ok(())
    } else if let Some(name) = self.name {
      let target_dir = Path::new("templates").join(&name);
      let ejected_files =
        templates::eject_template(&name, &target_dir, self.force)?;

      if !quiet {
        ui::print_success(&format!(
          "Ejected template '{name}' to ./templates/{name}/"
        ));
        for file in ejected_files {
          println!("  - {file}");
        }
        println!(
          "Run `rsmk build --template ./templates/{name}/main.typ` to compile \
           with your local template."
        );
      }
      Ok(())
    } else {
      println!(
        "Usage: rsmk template [NAME] [--list] [--force]\n\nRun `rsmk template \
         --help` for more information."
      );
      Ok(())
    }
  }
}
