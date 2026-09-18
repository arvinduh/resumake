//! Command-line argument parsing and subcommand definitions.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// High-performance native Rust résumé compilation and telemetry engine.
#[derive(Debug, Parser)]
#[command(
  name = "rsmk",
  version,
  about = "Modular résumé compiler and telemetry engine",
  long_about = "rsmk is a high-performance native Rust CLI for \
    compiling and verifying single-page résumés with strict layout \
    telemetry."
)]
pub struct Cli {
  /// Suppress non-essential terminal output
  #[arg(short = 'q', long = "quiet", global = true)]
  pub quiet: bool,

  /// Subcommand to execute (defaults to build if omitted)
  #[command(subcommand)]
  pub command: Option<Commands>,
}

/// Available CLI subcommands.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Commands {
  /// Compile résumé to PDF and verify layout telemetry
  Build {
    /// Path to content YAML file
    #[arg(value_name = "CONTENT", default_value = "content.yaml")]
    content: PathBuf,

    /// Named built-in layout, local template directory, or .typ file to render with
    #[arg(short, long)]
    template: Option<String>,

    /// Custom output PDF path (defaults to `<name>_resume.pdf`)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Path to custom JSON schema file (falls back to built-in schema if omitted)
    #[arg(long)]
    schema: Option<PathBuf>,

    /// Custom font directory (auto-detects ./fonts if present)
    #[arg(long)]
    font_path: Option<PathBuf>,
  },
  /// Verify schema and single-page layout geometry without generating a PDF
  Check {
    /// Path to content YAML file
    #[arg(value_name = "CONTENT", default_value = "content.yaml")]
    content: PathBuf,

    /// Named built-in layout, local template directory, or .typ file to render with
    #[arg(short, long)]
    template: Option<String>,

    /// Path to custom JSON schema file (falls back to built-in schema if omitted)
    #[arg(long)]
    schema: Option<PathBuf>,

    /// Custom font directory (auto-detects ./fonts if present)
    #[arg(long)]
    font_path: Option<PathBuf>,
  },
  /// Scaffold a new résumé workspace with rich examples, workflows, and git config
  Init {
    /// Destination for the scaffold: a directory (creates it and writes
    /// `content.yaml` inside) or an explicit `*.yaml` file path. Defaults to
    /// `content.yaml` in the current directory.
    #[arg(value_name = "DEST")]
    dest: Option<PathBuf>,

    /// Candidate display name
    #[arg(short, long)]
    name: Option<String>,

    /// Overwrite destination file if it already exists
    #[arg(short, long)]
    force: bool,

    /// Skip initializing a git repository and git config files (also skips
    /// GitHub Actions workflows, which require a repo; add them later with
    /// `rsmk init --workflows`)
    #[arg(long)]
    no_git: bool,

    /// Skip generating GitHub Actions CI and Release workflows
    #[arg(long)]
    no_workflows: bool,

    /// Create or refresh GitHub Actions workflows without modifying content.yaml
    #[arg(long, visible_alias = "update")]
    workflows: bool,
  },
  /// Validate repository state, verify semver, and cut a new release tag
  Release {
    /// Path to content YAML file
    #[arg(value_name = "CONTENT", default_value = "content.yaml")]
    content: PathBuf,

    /// Optional release message for the annotated git tag
    #[arg(short, long)]
    message: Option<String>,

    /// Dry-run mode: run all pre-flight checks without creating or pushing a git tag
    #[arg(long)]
    dry_run: bool,

    /// Skip compilation and telemetry pre-flight check (rsmk check)
    #[arg(long)]
    skip_build: bool,
  },
  /// Manage and eject résumé layout templates
  Template {
    /// Name of the template to eject (e.g. `classic`)
    #[arg(value_name = "NAME", conflicts_with = "list")]
    name: Option<String>,

    /// List all available built-in and discovered custom templates
    #[arg(long, conflicts_with = "name")]
    list: bool,

    /// Overwrite destination directory if it already exists when ejecting
    #[arg(short, long)]
    force: bool,
  },
  /// Replace the installed binary with the latest GitHub release
  Update {
    /// Report whether a newer release exists without installing it
    #[arg(long)]
    check: bool,

    /// Reinstall even if the current version is already latest
    #[arg(short, long)]
    force: bool,
  },
  /// Output canonical JSON schema for content.yaml
  Schema {
    /// Destination path for the generated schema JSON (prints to stdout if omitted)
    #[arg(short, long)]
    output: Option<PathBuf>,
  },
}

impl Default for Commands {
  fn default() -> Self {
    Commands::Build {
      content: PathBuf::from("content.yaml"),
      template: None,
      output: None,
      schema: None,
      font_path: None,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_cli_default_command() {
    let cli = Cli::parse_from(["rsmk"]);
    assert!(cli.command.is_none());
    assert!(!cli.quiet);
  }

  #[test]
  fn test_cli_build_flags() {
    let cli = Cli::parse_from([
      "rsmk",
      "build",
      "alt.yaml",
      "--template",
      "alt.typ",
      "--output",
      "out.pdf",
    ]);
    match cli.command.unwrap() {
      Commands::Build {
        content,
        output,
        template,
        schema,
        font_path,
      } => {
        assert_eq!(content, PathBuf::from("alt.yaml"));
        assert_eq!(template, Some("alt.typ".to_string()));
        assert_eq!(output, Some(PathBuf::from("out.pdf")));
        assert_eq!(schema, None);
        assert_eq!(font_path, None);
      }
      _ => panic!("Expected Build command"),
    }
  }

  #[test]
  fn test_cli_build_defaults() {
    let cli = Cli::parse_from(["rsmk", "build"]);
    match cli.command.unwrap() {
      Commands::Build {
        content,
        template,
        output,
        schema,
        font_path,
      } => {
        assert_eq!(content, PathBuf::from("content.yaml"));
        assert_eq!(template, None);
        assert_eq!(output, None);
        assert_eq!(schema, None);
        assert_eq!(font_path, None);
      }
      _ => panic!("Expected Build command"),
    }
  }

  #[test]
  fn test_cli_check_command() {
    let cli =
      Cli::parse_from(["rsmk", "check", "alt.yaml", "--template", "classic"]);
    match cli.command.unwrap() {
      Commands::Check {
        content,
        template,
        schema,
        font_path,
      } => {
        assert_eq!(content, PathBuf::from("alt.yaml"));
        assert_eq!(template, Some("classic".to_string()));
        assert_eq!(schema, None);
        assert_eq!(font_path, None);
      }
      _ => panic!("Expected Check command"),
    }
  }

  #[test]
  fn test_cli_template_list() {
    let cli = Cli::parse_from(["rsmk", "template", "--list"]);
    match cli.command.unwrap() {
      Commands::Template { list, name, force } => {
        assert!(list);
        assert_eq!(name, None);
        assert!(!force);
      }
      _ => panic!("Expected Template command"),
    }
  }

  #[test]
  fn test_cli_template_eject() {
    let cli = Cli::parse_from(["rsmk", "template", "classic"]);
    match cli.command.unwrap() {
      Commands::Template { name, list, force } => {
        assert_eq!(name, Some("classic".to_string()));
        assert!(!list);
        assert!(!force);
      }
      _ => panic!("Expected Template command"),
    }
  }

  #[test]
  fn test_cli_template_eject_force_flags() {
    let cli = Cli::parse_from(["rsmk", "template", "classic", "--force"]);
    match cli.command.unwrap() {
      Commands::Template { name, force, .. } => {
        assert_eq!(name, Some("classic".to_string()));
        assert!(force);
      }
      _ => panic!("Expected Template command"),
    }

    let cli_short = Cli::parse_from(["rsmk", "template", "classic", "-f"]);
    match cli_short.command.unwrap() {
      Commands::Template { name, force, .. } => {
        assert_eq!(name, Some("classic".to_string()));
        assert!(force);
      }
      _ => panic!("Expected Template command"),
    }
  }

  #[test]
  fn test_cli_template_no_args() {
    let cli = Cli::parse_from(["rsmk", "template"]);
    match cli.command.unwrap() {
      Commands::Template { name, list, force } => {
        assert_eq!(name, None);
        assert!(!list);
        assert!(!force);
      }
      _ => panic!("Expected Template command"),
    }
  }

  #[test]
  fn test_cli_update_flags() {
    let cli = Cli::parse_from(["rsmk", "update"]);
    match cli.command.unwrap() {
      Commands::Update { check, force } => {
        assert!(!check);
        assert!(!force);
      }
      _ => panic!("Expected Update command"),
    }

    let cli_flags = Cli::parse_from(["rsmk", "update", "--check", "--force"]);
    match cli_flags.command.unwrap() {
      Commands::Update { check, force } => {
        assert!(check);
        assert!(force);
      }
      _ => panic!("Expected Update command"),
    }

    let cli_short = Cli::parse_from(["rsmk", "update", "-f"]);
    match cli_short.command.unwrap() {
      Commands::Update { check, force } => {
        assert!(!check);
        assert!(force);
      }
      _ => panic!("Expected Update command"),
    }
  }

  #[test]
  fn test_cli_release_default_flags() {
    let cli = Cli::parse_from(["rsmk", "release"]);
    match cli.command.unwrap() {
      Commands::Release {
        content,
        message,
        dry_run,
        skip_build,
      } => {
        assert_eq!(content, PathBuf::from("content.yaml"));
        assert_eq!(message, None);
        assert!(!dry_run);
        assert!(!skip_build);
      }
      _ => panic!("Expected Release command"),
    }
  }

  #[test]
  fn test_cli_release_custom_flags() {
    let cli = Cli::parse_from([
      "rsmk",
      "release",
      "my_resume.yaml",
      "--message",
      "Version 1.2.0 release",
      "--dry-run",
      "--skip-build",
    ]);
    match cli.command.unwrap() {
      Commands::Release {
        content,
        message,
        dry_run,
        skip_build,
      } => {
        assert_eq!(content, PathBuf::from("my_resume.yaml"));
        assert_eq!(message, Some("Version 1.2.0 release".to_string()));
        assert!(dry_run);
        assert!(skip_build);
      }
      _ => panic!("Expected Release command"),
    }

    let cli_short =
      Cli::parse_from(["rsmk", "release", "my_resume.yaml", "-m", "Short msg"]);
    match cli_short.command.unwrap() {
      Commands::Release {
        content,
        message,
        dry_run,
        skip_build,
      } => {
        assert_eq!(content, PathBuf::from("my_resume.yaml"));
        assert_eq!(message, Some("Short msg".to_string()));
        assert!(!dry_run);
        assert!(!skip_build);
      }
      _ => panic!("Expected Release command"),
    }
  }

  #[test]
  fn test_cli_init_flags() {
    let cli = Cli::parse_from(["rsmk", "init"]);
    match cli.command.unwrap() {
      Commands::Init {
        dest,
        name,
        force,
        no_git,
        no_workflows,
        workflows,
      } => {
        assert_eq!(dest, None);
        assert_eq!(name, None);
        assert!(!force);
        assert!(!no_git);
        assert!(!no_workflows);
        assert!(!workflows);
      }
      _ => panic!("Expected Init command"),
    }

    let cli_custom = Cli::parse_from([
      "rsmk",
      "init",
      "custom.yaml",
      "--name",
      "John Smith",
      "--force",
      "--no-git",
      "--no-workflows",
      "--workflows",
    ]);
    match cli_custom.command.unwrap() {
      Commands::Init {
        dest,
        name,
        force,
        no_git,
        no_workflows,
        workflows,
      } => {
        assert_eq!(dest, Some(PathBuf::from("custom.yaml")));
        assert_eq!(name, Some("John Smith".to_string()));
        assert!(force);
        assert!(no_git);
        assert!(no_workflows);
        assert!(workflows);
      }
      _ => panic!("Expected Init command"),
    }

    let cli_update_alias = Cli::parse_from(["rsmk", "init", "--update"]);
    match cli_update_alias.command.unwrap() {
      Commands::Init { workflows, .. } => {
        assert!(workflows);
      }
      _ => panic!("Expected Init command"),
    }

    let cli_positional = Cli::parse_from(["rsmk", "init", "./arvin_resume"]);
    match cli_positional.command.unwrap() {
      Commands::Init { dest, .. } => {
        assert_eq!(dest, Some(PathBuf::from("./arvin_resume")));
      }
      _ => panic!("Expected Init command"),
    }
  }

  #[test]
  fn test_cli_schema_flags() {
    let cli = Cli::parse_from(["rsmk", "schema", "-o", "schema.json"]);
    match cli.command.unwrap() {
      Commands::Schema { output } => {
        assert_eq!(output, Some(PathBuf::from("schema.json")));
      }
      _ => panic!("Expected Schema command"),
    }
  }
}
