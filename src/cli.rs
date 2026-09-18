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
