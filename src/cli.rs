//! Command routing for the `rsmk` binary.
//!
//! Owns CLI arguments, subcommand dispatch, and top-level error mapping.
//! Process management and terminal execution live in `main.rs`.

mod build;
mod check;
mod init;
mod run;
mod schema;
mod template;
mod update;

use std::io;
use std::path;

use clap;
use resumake;
use thiserror;

/// Cargo-style colors for `--help`; clap drops them for non-TTY output and
/// under `NO_COLOR`.
const STYLES: clap::builder::Styles = clap::builder::Styles::styled()
  .header(clap::builder::styling::AnsiColor::Green.on_default().bold())
  .usage(clap::builder::styling::AnsiColor::Green.on_default().bold())
  .literal(clap::builder::styling::AnsiColor::Cyan.on_default().bold())
  .placeholder(clap::builder::styling::AnsiColor::Cyan.on_default());

/// CLI-level errors from subcommand dispatch.
#[derive(Debug, thiserror::Error)]
pub enum Error {
  /// Typst rendering or compilation failure.
  #[error(transparent)]
  Render(#[from] resumake::render::Error),
  /// Schema validation or inspection failure.
  #[error(transparent)]
  Schema(#[from] resumake::schema::Error),
  /// Scaffolding or initialization failure.
  #[error(transparent)]
  Scaffold(#[from] resumake::scaffold::Error),
  /// Template resolution or ejection failure.
  #[error(transparent)]
  Template(#[from] resumake::templates::Error),
  /// Project manifest parsing or loading failure.
  #[error(transparent)]
  Manifest(#[from] resumake::manifest::Error),
  /// Self-update failure.
  #[error(transparent)]
  Update(#[from] update::Error),
  /// JSON serialization failure.
  #[error(transparent)]
  Json(#[from] serde_json::Error),
  /// Standard I/O failure.
  #[error(transparent)]
  Io(#[from] io::Error),
  /// CLI configuration or target selection error.
  #[error("{0}")]
  Config(String),
  /// A document broke one or more layout telemetry gates.
  #[error("layout check failed for '{doc}':\n{reasons}")]
  Layout {
    /// Target document label.
    doc: String,
    /// Formatted telemetry failure reasons.
    reasons: String,
  },
  /// One or more documents failed during a multi-document run.
  #[error("{failed} of {total} document(s) failed:\n{details}")]
  BatchFailed {
    /// Count of failed documents.
    failed: usize,
    /// Total documents attempted.
    total: usize,
    /// Per-document formatted errors.
    details: String,
  },
}

impl Error {
  /// Returns the CLI flag that resolves this error, when one exists.
  ///
  /// The library reports what went wrong; only the CLI knows its flags.
  pub fn hint(&self) -> Option<&'static str> {
    match self {
      Self::Scaffold(
        resumake::scaffold::Error::AlreadyExists { .. }
        | resumake::scaffold::Error::Template(
          resumake::templates::Error::AlreadyExists { .. },
        ),
      )
      | Self::Template(resumake::templates::Error::AlreadyExists { .. }) => {
        Some("use --force to overwrite")
      }
      _ => None,
    }
  }
}

/// High-performance native Rust résumé compilation and telemetry engine.
#[derive(clap::Parser)]
#[command(
  name = "rsmk",
  version,
  styles = STYLES,
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
#[derive(clap::Subcommand)]
pub enum Command {
  /// Compile résumé to PDF and verify layout telemetry
  Build(build::BuildArgs),
  /// Verify schema and single-page layout geometry without generating a PDF
  Check(check::CheckArgs),
  /// Scaffold a new résumé workspace with rich examples, workflows, and git
  /// config
  Init(init::InitArgs),
  /// Manage and eject résumé layout templates
  Template(template::TemplateArgs),
  /// Replace the installed binary with the latest `GitHub` release
  Update(update::UpdateArgs),
  /// Output canonical JSON Schema for content or manifest files
  Schema(schema::SchemaArgs),
}

impl Default for Command {
  fn default() -> Self {
    Self::Build(build::BuildArgs::default())
  }
}

impl Cli {
  /// Parses CLI arguments and executes the selected subcommand.
  ///
  /// # Errors
  /// Returns [`Error`] if subcommand execution fails.
  pub fn run(self) -> Result<(), Error> {
    let quiet = self.quiet;
    let command = self.command.unwrap_or_default();

    match command {
      Command::Build(args) => Ok(args.run(quiet)?),
      Command::Check(args) => Ok(args.run(quiet)?),
      Command::Init(args) => Ok(args.run(quiet)?),
      Command::Template(args) => Ok(args.run(quiet)?),
      Command::Update(args) => Ok(args.run(quiet)?),
      Command::Schema(args) => Ok(args.run(quiet)?),
    }
  }
}

/// Finds the project a content file belongs to.
///
/// Walks up from the content file's directory to the nearest one holding
/// `resumake.toml`, `.git`, or `content.{yaml,yml,json}` so `fonts/` and
/// `templates/` resolve uniformly for track sub-files. Falls back to the
/// content file's own directory.
pub(crate) fn project_root(content: &path::Path) -> path::PathBuf {
  let dir = content
    .parent()
    .filter(|p| !p.as_os_str().is_empty())
    .unwrap_or_else(|| path::Path::new("."));
  let Ok(dir) = dir.canonicalize() else {
    return dir.to_path_buf();
  };
  dir
    .ancestors()
    .find(|d| {
      d.join(resumake::manifest::MANIFEST_FILE).is_file()
        || d.join(".git").exists()
        || d.join("content.yaml").is_file()
        || d.join("content.yml").is_file()
        || d.join("content.json").is_file()
    })
    .unwrap_or(&dir)
    .to_path_buf()
}

/// Truncates a string to at most `max_chars` UTF-8 characters with an ellipsis.
fn truncate_ellipsis(s: &str, max_chars: usize) -> String {
  let count = s.chars().count();
  if count <= max_chars {
    s.to_string()
  } else {
    let truncated: String =
      s.chars().take(max_chars.saturating_sub(3)).collect();
    format!("{truncated}...")
  }
}

/// Prints a concise, unbadged layout telemetry summary to stdout.
#[expect(clippy::print_stdout, reason = "CLI output")]
pub(crate) fn print_report(
  report: &resumake::telemetry::TelemetryReport,
  outcome: &resumake::telemetry::PolicyOutcome,
  prefix: &str,
) {
  if report.page_count == 1 {
    println!("{prefix} (1 page, {:.1}% fill)", report.fill_pct);
  } else {
    println!("{prefix} ({} pages):", report.page_count);
    for (i, fill) in report.page_fills.iter().enumerate() {
      let page_num = i + 1;
      if page_num == report.page_count {
        println!(
          "  page {page_num}: {fill:.1}% fill ({:.2} in spare)",
          report.spare_in
        );
      } else {
        println!("  page {page_num}: {fill:.1}% fill");
      }
    }
  }

  for wrap in &report.line_wraps {
    let lines = wrap.line_count();
    let tail_note = wrap
      .tail_fill
      .map_or_else(String::new, |t| format!(", tail {t:.0}%"));
    if let Some(b) = wrap.wrap_boundary() {
      println!(
        "  note: {lines} lines{tail_note}, wrap at {:.0}% (fits to char {}): \
         spills \"{}\"",
        wrap.fill,
        b.fit_char_idx,
        truncate_ellipsis(b.spill_text, 40)
      );
    } else {
      println!(
        "  note: {lines} lines{tail_note}, wrap at {:.0}%: \"{}\"",
        wrap.fill,
        truncate_ellipsis(&wrap.text, 50)
      );
    }
  }

  for warning in &outcome.warnings {
    println!("  warning: {warning}");
  }
}
