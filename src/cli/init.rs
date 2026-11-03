//! Handlers for `rsmk init`.
//!
//! Owns argument definitions and execution for scaffolding a new résumé
//! workspace with example content, workflows, and git configurations.

use std::io;
use std::io::IsTerminal;
use std::io::Write;
use std::path;

use clap;
use resumake;

use crate::cli;

/// Command line arguments for `rsmk init`.
#[derive(clap::Args)]
#[expect(clippy::struct_excessive_bools, reason = "CLI argument flags")]
pub struct InitArgs {
  /// Destination directory for the scaffold (defaults to `.`)
  #[arg(value_name = "DEST")]
  pub dest: Option<path::PathBuf>,

  /// Candidate display name
  #[arg(short, long)]
  pub name: Option<String>,

  /// Overwrite destination files if they already exist
  #[arg(short, long)]
  pub force: bool,

  /// Skip initializing a git repository and git config files
  #[arg(long)]
  pub no_git: bool,

  /// Skip generating GitHub Actions CI and Release workflows
  #[arg(long)]
  pub no_workflows: bool,

  /// Eject the template into templates/<name>.typ during initialization
  #[arg(short = 'e', long)]
  pub eject: bool,

  /// Template to initialize with or eject (defaults to "classic")
  #[arg(short = 't', long, default_value = "classic")]
  pub template: String,
}

impl InitArgs {
  /// Runs the init command.
  ///
  /// # Errors
  /// Returns [`cli::Error`] if file creation or directory setup fails.
  #[expect(clippy::print_stdout, reason = "CLI output")]
  pub fn run(self, quiet: bool) -> Result<(), cli::Error> {
    let target_dir = self.dest.unwrap_or_else(|| path::PathBuf::from("."));
    let candidate_name = resolve_candidate_name(self.name.as_deref());

    let opts = resumake::scaffold::InitOptions {
      name: &candidate_name,
      target_dir: &target_dir,
      force: self.force,
      no_git: self.no_git,
      no_workflows: self.no_workflows,
      eject_template: self.eject,
      template: &self.template,
    };

    let outcome = resumake::scaffold::init(&opts)?;

    if !quiet {
      println!(
        "Initialized résumé repository at '{}'",
        outcome.content_path.display()
      );
      if outcome.git_initialized {
        println!("  initialized git repository");
      }
      for f in &outcome.created_files {
        println!("  created {}", f.display());
      }
    }

    Ok(())
  }
}

/// Resolves candidate display name from flag, environment, or prompt.
fn resolve_candidate_name(name_arg: Option<&str>) -> String {
  if let Some(n) = name_arg
    && !n.trim().is_empty()
  {
    return n.trim().to_string();
  }

  if let Ok(author) = std::env::var("GIT_AUTHOR_NAME")
    && !author.trim().is_empty()
  {
    return author.trim().to_string();
  }

  if io::stdin().is_terminal() {
    #[expect(clippy::print_stdout, reason = "Interactive CLI prompt")]
    {
      print!("Candidate Name [Jane Doe]: ");
      let _ = io::stdout().flush();
    }
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
      let trimmed = input.trim();
      if !trimmed.is_empty() {
        return trimmed.to_string();
      }
    }
  }

  "Jane Doe".to_string()
}
