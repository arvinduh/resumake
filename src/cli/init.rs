//! Handlers for `rsmk init`.

use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

use resumake::workspace;

use crate::cli::{ui, Error};

/// Command line arguments for `rsmk init`.
#[derive(Debug, Clone, PartialEq, Eq, clap::Args)]
#[expect(clippy::struct_excessive_bools, reason = "CLI argument flags")]
pub struct InitArgs {
  /// Destination directory or file path for the content scaffold (defaults to
  /// `./content.yaml`)
  #[arg(value_name = "DEST")]
  pub dest: Option<PathBuf>,

  /// Candidate display name
  #[arg(short, long)]
  pub name: Option<String>,

  /// Overwrite destination file if it already exists
  #[arg(short, long)]
  pub force: bool,

  /// Skip initializing a git repository and git config files (also skips
  /// `GitHub` Actions workflows, which require a repo; add them later with
  /// `rsmk init --workflows`)
  #[arg(long)]
  pub no_git: bool,

  /// Skip generating `GitHub` Actions CI and Release workflows
  #[arg(long)]
  pub no_workflows: bool,

  /// Create or refresh `GitHub` Actions workflows without modifying
  /// content.yaml
  #[arg(long, visible_alias = "update")]
  pub workflows: bool,
}

impl InitArgs {
  /// Runs the init command.
  ///
  /// # Errors
  ///
  /// Returns an [`Error`] if initialization or workflow generation fails.
  pub fn run(self, quiet: bool) -> Result<(), Error> {
    let resolved_output = workspace::resolve_init_output(self.dest.as_deref());
    let base_dir = resolved_output
      .parent()
      .filter(|p| !p.as_os_str().is_empty())
      .unwrap_or(Path::new("."));

    if self.workflows {
      return run_update_workflows(base_dir, self.force, quiet);
    }

    let candidate_name = resolve_candidate_name(self.name.as_deref());
    let opts = workspace::InitOptions {
      name: &candidate_name,
      output: &resolved_output,
      force: self.force,
      no_git: self.no_git,
      no_workflows: self.no_workflows,
    };

    let outcome = workspace::init(&opts)?;

    if !quiet {
      ui::print_success(&format!(
        "Initialized new résumé content scaffold at '{}'",
        outcome.content_path.display()
      ));

      if outcome.git_initialized {
        ui::print_success("Initialized Git repository");
      }
      if !self.no_git {
        ui::print_success("Created .gitignore and .gitattributes");
      }
      if outcome.workflows_created {
        ui::print_success(
          "Created GitHub Actions workflows in .github/workflows/",
        );
      } else if self.no_git && !self.no_workflows {
        ui::print_info(
          "Skipping GitHub Actions workflows (--no-git). Run `rsmk init \
           --workflows` from the project directory to add CI/Release \
           workflows later.",
        );
      }

      if !self.no_git {
        print_next_steps();
      }
    }

    Ok(())
  }
}

fn resolve_candidate_name(name_arg: Option<&str>) -> String {
  if let Some(name) = name_arg {
    let trimmed = name.trim();
    if !trimmed.is_empty() {
      return trimmed.to_string();
    }
  }

  if io::stdin().is_terminal() && io::stdout().is_terminal() {
    print!("Candidate name [Jane Doe]: ");
    let _ = io::stdout().flush();
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

fn run_update_workflows(
  base_dir: &Path,
  force: bool,
  quiet: bool,
) -> Result<(), Error> {
  let version = env!("CARGO_PKG_VERSION");
  let outcomes = workspace::update_workflows(base_dir, force)?;

  if !quiet {
    for outcome in outcomes {
      match outcome.status {
        workspace::WorkflowUpdateStatus::Created => {
          ui::print_success(&format!(
            "Created workflow '{}' pinned to rsmk v{version}",
            outcome.path.display()
          ));
        }
        workspace::WorkflowUpdateStatus::Updated => {
          ui::print_success(&format!(
            "Updated workflow '{}' to rsmk v{version}",
            outcome.path.display()
          ));
        }
        workspace::WorkflowUpdateStatus::ForceUpdated => {
          ui::print_success(&format!(
            "Force updated workflow '{}' to rsmk v{version}",
            outcome.path.display()
          ));
        }
        workspace::WorkflowUpdateStatus::SkippedLocalModifications => {
          eprintln!(
            "warning: Workflow '{}' has local modifications; skipping update. \
             Use --force to overwrite.",
            outcome.path.display()
          );
        }
      }
    }
  }

  Ok(())
}

fn print_next_steps() {
  println!("\nNext steps to publish your résumé repository:");
  println!("  1. Track and commit your changes:");
  println!("     git add .");
  println!("     git commit -m \"feat: initial résumé scaffold\"");
  println!("  2. Push to GitHub:");
  println!("     gh repo create --source=. --push");
  println!("     # Or configure remote manually:");
  println!("     git remote add origin git@github.com:<username>/<repo>.git");
  println!("     git branch -M main");
  println!("     git push -u origin main");
}
