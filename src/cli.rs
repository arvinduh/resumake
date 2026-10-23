//! CLI router, command definitions, and binary error types.

mod build;
mod check;
mod init;
mod release;
mod schema;
mod template;
pub(crate) mod ui;
#[cfg(feature = "update")]
mod update;

/// CLI-level errors from subcommand dispatch.
#[derive(Debug, thiserror::Error)]
pub enum Error {
  /// Engine compilation or verification error.
  #[error(transparent)]
  Engine(#[from] resumake::engine::Error),
  /// Schema validation or inspection error.
  #[error(transparent)]
  Schema(#[from] resumake::schema::Error),
  /// Telemetry evaluation error.
  #[error(transparent)]
  Telemetry(#[from] resumake::telemetry::Error),
  /// Git operation error.
  #[error(transparent)]
  Git(#[from] resumake::utils::git::Error),
  /// Workspace initialization failure.
  #[error(transparent)]
  Workspace(#[from] resumake::workspace::Error),
  /// Release pipeline failure.
  #[error(transparent)]
  Release(#[from] resumake::release::Error),
  #[cfg(feature = "update")]
  /// Self-update failure.
  #[error(transparent)]
  Update(#[from] update::Error),
  /// Standard I/O failure.
  #[error("I/O error: {0}")]
  Io(#[from] std::io::Error),
}

/// High-performance native Rust résumé compilation and telemetry engine.
#[derive(Debug, clap::Parser)]
#[command(
  name = "rsmk",
  version,
  about = "Modular résumé compiler and telemetry engine",
  long_about = "rsmk is a high-performance native Rust CLI for compiling and \
                verifying single-page résumés with strict layout telemetry."
)]
pub struct Cli {
  /// Suppress non-essential terminal output
  #[arg(short = 'q', long = "quiet", global = true)]
  pub quiet: bool,

  /// Subcommand to execute (defaults to build if omitted)
  #[command(subcommand)]
  pub command: Option<Command>,
}

/// Available CLI subcommands.
#[derive(Debug, Clone, PartialEq, Eq, clap::Subcommand)]
pub enum Command {
  /// Compile résumé to PDF and verify layout telemetry
  Build(build::BuildArgs),
  /// Verify schema and single-page layout geometry without generating a PDF
  Check(check::CheckArgs),
  /// Scaffold a new résumé workspace with rich examples, workflows, and git
  /// config
  Init(init::InitArgs),
  /// Validate repository state, verify semver, and cut a new release tag
  Release(release::ReleaseArgs),
  /// Manage and eject résumé layout templates
  Template(template::TemplateArgs),
  #[cfg(feature = "update")]
  /// Replace the installed binary with the latest `GitHub` release
  Update(update::UpdateArgs),
  /// Output canonical JSON schema for content.yaml
  Schema(schema::SchemaArgs),
}

impl Default for Command {
  fn default() -> Self {
    Command::Build(build::BuildArgs::default())
  }
}

impl Cli {
  /// Parses CLI arguments and executes the selected subcommand.
  ///
  /// # Errors
  ///
  /// Returns an [`Error`] if subcommand execution fails.
  pub fn run(self) -> Result<(), Error> {
    let quiet = self.quiet;
    let command = self.command.unwrap_or_default();

    match command {
      Command::Build(args) => Ok(args.run(quiet)?),
      Command::Check(args) => Ok(args.run(quiet)?),
      Command::Init(args) => Ok(args.run(quiet)?),
      Command::Release(args) => Ok(args.run(quiet)?),
      Command::Template(args) => Ok(args.run(quiet)?),
      #[cfg(feature = "update")]
      Command::Update(args) => Ok(args.run(quiet)?),
      Command::Schema(args) => Ok(args.run(quiet)?),
    }
  }
}
