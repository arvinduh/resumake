//! Command-line argument parsing and subcommand definitions.

use crate::engine::DEFAULT_TEMPLATE;
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
    #[arg(short = 'c', long = "content", default_value = "content.yaml")]
    content: PathBuf,

    /// Named built-in layout to render with (currently only `classic` is
    /// bundled; the registry exists so more layouts can be added later)
    #[arg(short = 't', long = "template", default_value = DEFAULT_TEMPLATE)]
    template_name: String,

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
    #[arg(short = 'c', long = "content", default_value = "content.yaml")]
    content: PathBuf,

    /// Named built-in layout to validate against
    #[arg(short = 't', long = "template", default_value = DEFAULT_TEMPLATE)]
    template_name: String,

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
    #[arg(short = 'c', long = "content", default_value = "content.yaml")]
    content: PathBuf,

    /// Named built-in layout to render with
    #[arg(short = 't', long = "template", default_value = DEFAULT_TEMPLATE)]
    template_name: String,

    /// Path to a custom Typst template file, bypassing the built-in
    /// registry entirely
    #[arg(short = 's', long = "source")]
    source: Option<PathBuf>,

    /// Custom output PDF path
    #[arg(short = 'o', long = "output")]
    output: Option<PathBuf>,

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
  /// Scaffold a new résumé content YAML with rich examples and schema
  /// directives
  Init {
    /// Candidate display name
    #[arg(short = 'n', long = "name")]
    name: Option<String>,

    /// Destination path for the new content file
    #[arg(short = 'o', long = "output", default_value = "content.yaml")]
    output: PathBuf,

    /// Overwrite destination file if it already exists
    #[arg(short = 'f', long = "force")]
    force: bool,
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
      template_name: DEFAULT_TEMPLATE.to_string(),
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
        template_name,
        ..
      } => {
        assert_eq!(content, PathBuf::from("alt.yaml"));
        assert_eq!(source, Some(PathBuf::from("alt.typ")));
        assert_eq!(output, Some(PathBuf::from("out.pdf")));
        assert_eq!(template_name, DEFAULT_TEMPLATE);
      }
      _ => panic!("Expected Build command"),
    }
  }

  #[test]
  fn test_cli_build_template_name_flag() {
    let cli = Cli::parse_from(["rsmk", "build", "--template", "classic"]);
    match cli.command.unwrap() {
      Commands::Build { template_name, .. } => {
        assert_eq!(template_name, "classic");
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
}
