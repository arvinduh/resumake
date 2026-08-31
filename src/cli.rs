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
    #[arg(long = "content", default_value = "content.yaml")]
    content: PathBuf,

    /// Dry-run verification mode (evaluates schema + telemetry without generating a PDF output)
    #[arg(short, long)]
    check: bool,

    /// Live file watcher mode
    #[arg(short, long)]
    watch: bool,

    /// Named built-in layout to render with or path to template
    #[arg(short, long)]
    template: Option<String>,

    /// Path to a custom Typst template file, bypassing the built-in
    /// registry entirely
    #[arg(short = 's', long = "source")]
    source: Option<PathBuf>,

    /// Custom output PDF path (defaults to `<name>_resume.pdf`)
    #[arg(short = 'o', long = "output")]
    output: Option<PathBuf>,

    /// Path to custom JSON schema file (falls back to built-in schema if
    /// omitted)
    #[arg(long = "schema")]
    schema: Option<PathBuf>,

    /// Custom font directory (auto-detects ./fonts if present)
    #[arg(long = "font-path")]
    font_path: Option<PathBuf>,
  },
  /// Dry-run schema and layout validation without writing a PDF
  Check {
    /// Path to content YAML file
    #[arg(long = "content", default_value = "content.yaml")]
    content: PathBuf,

    /// Live file watcher mode
    #[arg(short, long)]
    watch: bool,

    /// Named built-in layout to validate against
    #[arg(short, long)]
    template: Option<String>,

    /// Path to a custom Typst template file, bypassing the built-in
    /// registry entirely
    #[arg(short = 's', long = "source")]
    source: Option<PathBuf>,

    /// Path to custom JSON schema file (falls back to built-in schema if
    /// omitted)
    #[arg(long = "schema")]
    schema: Option<PathBuf>,

    /// Custom font directory
    #[arg(long = "font-path")]
    font_path: Option<PathBuf>,
  },
  /// Watch content and template for live re-compilation
  Watch {
    /// Path to content YAML file
    #[arg(long = "content", default_value = "content.yaml")]
    content: PathBuf,

    /// Dry-run verification mode (evaluates schema + telemetry without generating a PDF output)
    #[arg(short, long)]
    check: bool,

    /// Named built-in layout to render with
    #[arg(short, long)]
    template: Option<String>,

    /// Path to a custom Typst template file, bypassing the built-in
    /// registry entirely
    #[arg(short = 's', long = "source")]
    source: Option<PathBuf>,

    /// Custom output PDF path
    #[arg(short = 'o', long = "output")]
    output: Option<PathBuf>,

    /// Path to custom JSON schema file (falls back to built-in schema if
    /// omitted)
    #[arg(long = "schema")]
    schema: Option<PathBuf>,

    /// Custom font directory
    #[arg(long = "font-path")]
    font_path: Option<PathBuf>,
  },
  /// Export or inspect the canonical JSON schema (Draft-07)
  Schema {
    /// Path to export the JSON schema file to (prints to stdout if omitted)
    #[arg(short = 'e', long = "export")]
    export: Option<PathBuf>,
  },
  /// Scaffold a new résumé workspace with rich examples, workflows, and git config
  Init {
    /// Candidate display name
    #[arg(short, long)]
    name: Option<String>,

    /// Destination path for the new content file
    #[arg(short, long, default_value = "content.yaml")]
    output: PathBuf,

    /// Overwrite destination file if it already exists
    #[arg(short, long)]
    force: bool,

    /// Skip initializing a git repository and git config files
    #[arg(long)]
    no_git: bool,

    /// Skip generating GitHub Actions CI and Release workflows
    #[arg(long)]
    no_workflows: bool,

    /// Refresh GitHub Actions workflow pins without modifying content.yaml
    #[arg(short, long)]
    update: bool,
  },
  /// Validate repository state, verify semver, and cut a new release tag
  Release {
    /// Path to content YAML file
    #[arg(short, long, default_value = "content.yaml")]
    content: PathBuf,

    /// Optional release message for the annotated git tag
    #[arg(short, long)]
    message: Option<String>,

    /// Dry-run mode: run all pre-flight checks without creating or pushing a git tag
    #[arg(long)]
    dry_run: bool,

    /// Skip compilation and telemetry pre-flight check (rsmk build --check)
    #[arg(long)]
    skip_build: bool,
  },
  /// Manage and eject résumé layout templates
  Template(TemplateArgs),
}

/// Arguments for the `template` subcommand.
#[derive(Debug, Clone, PartialEq, Eq, clap::Args)]
pub struct TemplateArgs {
  /// Template subcommand to execute
  #[command(subcommand)]
  pub command: TemplateCommands,
}

/// Available template subcommands.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum TemplateCommands {
  /// List all available built-in and discovered custom templates
  List,
  /// Eject an embedded template to a local directory for customization
  Eject {
    /// Name of the template to eject (e.g. `classic`)
    name: String,

    /// Overwrite destination directory if it already exists
    #[arg(short, long)]
    force: bool,
  },
}

impl Default for Commands {
  fn default() -> Self {
    Commands::Build {
      content: PathBuf::from("content.yaml"),
      check: false,
      watch: false,
      template: None,
      source: None,
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
      "--content",
      "alt.yaml",
      "--source",
      "alt.typ",
      "--output",
      "out.pdf",
    ]);
    match cli.command.unwrap() {
      Commands::Build {
        content,
        source,
        output,
        template,
        check,
        watch,
        ..
      } => {
        assert_eq!(content, PathBuf::from("alt.yaml"));
        assert_eq!(source, Some(PathBuf::from("alt.typ")));
        assert_eq!(output, Some(PathBuf::from("out.pdf")));
        assert_eq!(template, None);
        assert!(!check);
        assert!(!watch);
      }
      _ => panic!("Expected Build command"),
    }
  }

  #[test]
  fn test_cli_build_template_name_flag() {
    let cli = Cli::parse_from(["rsmk", "build", "--template", "classic"]);
    match cli.command.unwrap() {
      Commands::Build { template, .. } => {
        assert_eq!(template, Some("classic".to_string()));
      }
      _ => panic!("Expected Build command"),
    }
  }

  #[test]
  fn test_cli_build_check_flag() {
    let cli = Cli::parse_from(["rsmk", "build", "--check"]);
    match cli.command.unwrap() {
      Commands::Build { check, watch, .. } => {
        assert!(check);
        assert!(!watch);
      }
      _ => panic!("Expected Build command"),
    }

    let cli_short = Cli::parse_from(["rsmk", "build", "-c"]);
    match cli_short.command.unwrap() {
      Commands::Build { check, watch, .. } => {
        assert!(check);
        assert!(!watch);
      }
      _ => panic!("Expected Build command"),
    }
  }

  #[test]
  fn test_cli_build_watch_flag() {
    let cli = Cli::parse_from(["rsmk", "build", "--watch"]);
    match cli.command.unwrap() {
      Commands::Build { check, watch, .. } => {
        assert!(!check);
        assert!(watch);
      }
      _ => panic!("Expected Build command"),
    }

    let cli_short = Cli::parse_from(["rsmk", "build", "-w"]);
    match cli_short.command.unwrap() {
      Commands::Build { check, watch, .. } => {
        assert!(!check);
        assert!(watch);
      }
      _ => panic!("Expected Build command"),
    }
  }

  #[test]
  fn test_cli_template_list() {
    let cli = Cli::parse_from(["rsmk", "template", "list"]);
    match cli.command.unwrap() {
      Commands::Template(args) => {
        assert_eq!(args.command, TemplateCommands::List);
      }
      _ => panic!("Expected Template command"),
    }
  }

  #[test]
  fn test_cli_template_eject() {
    let cli = Cli::parse_from(["rsmk", "template", "eject", "classic"]);
    match cli.command.unwrap() {
      Commands::Template(args) => match args.command {
        TemplateCommands::Eject { name, force } => {
          assert_eq!(name, "classic");
          assert!(!force);
        }
        _ => panic!("Expected Eject command"),
      },
      _ => panic!("Expected Template command"),
    }
  }

  #[test]
  fn test_cli_template_eject_force_flags() {
    let cli =
      Cli::parse_from(["rsmk", "template", "eject", "classic", "--force"]);
    match cli.command.unwrap() {
      Commands::Template(args) => match args.command {
        TemplateCommands::Eject { name, force } => {
          assert_eq!(name, "classic");
          assert!(force);
        }
        _ => panic!("Expected Eject command"),
      },
      _ => panic!("Expected Template command"),
    }

    let cli_short =
      Cli::parse_from(["rsmk", "template", "eject", "classic", "-f"]);
    match cli_short.command.unwrap() {
      Commands::Template(args) => match args.command {
        TemplateCommands::Eject { name, force } => {
          assert_eq!(name, "classic");
          assert!(force);
        }
        _ => panic!("Expected Eject command"),
      },
      _ => panic!("Expected Template command"),
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
      "--content",
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

    let cli_short = Cli::parse_from([
      "rsmk",
      "release",
      "-c",
      "my_resume.yaml",
      "-m",
      "Short msg",
    ]);
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
        name,
        output,
        force,
        no_git,
        no_workflows,
        update,
      } => {
        assert_eq!(name, None);
        assert_eq!(output, PathBuf::from("content.yaml"));
        assert!(!force);
        assert!(!no_git);
        assert!(!no_workflows);
        assert!(!update);
      }
      _ => panic!("Expected Init command"),
    }

    let cli_custom = Cli::parse_from([
      "rsmk",
      "init",
      "--name",
      "John Smith",
      "--output",
      "custom.yaml",
      "--force",
      "--no-git",
      "--no-workflows",
      "--update",
    ]);
    match cli_custom.command.unwrap() {
      Commands::Init {
        name,
        output,
        force,
        no_git,
        no_workflows,
        update,
      } => {
        assert_eq!(name, Some("John Smith".to_string()));
        assert_eq!(output, PathBuf::from("custom.yaml"));
        assert!(force);
        assert!(no_git);
        assert!(no_workflows);
        assert!(update);
      }
      _ => panic!("Expected Init command"),
    }

    let cli_short_update = Cli::parse_from(["rsmk", "init", "-u"]);
    match cli_short_update.command.unwrap() {
      Commands::Init { update, .. } => {
        assert!(update);
      }
      _ => panic!("Expected Init command"),
    }
  }
}
