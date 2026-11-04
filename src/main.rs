//! The `rsmk` binary: the process host.
//!
//! Owns process-level side effects: argument parsing, logger installation,
//! terminal output formatting, and mapping the command's outcome onto an exit
//! code. Commands live in `cli`.

mod cli;

use std::process;

use clap::Parser;
use env_logger;
use log;

#[expect(clippy::print_stderr, reason = "Process host error output")]
fn main() -> process::ExitCode {
  let cli = cli::Cli::parse();

  let level = if cli.quiet {
    log::LevelFilter::Error
  } else {
    log::LevelFilter::Warn
  };

  env_logger::Builder::new()
    .filter_level(level)
    .parse_default_env()
    .format_timestamp(None)
    .format_target(false)
    .init();

  match cli.run() {
    Ok(()) => process::ExitCode::SUCCESS,
    Err(err) => {
      eprintln!("{err}");
      if let Some(hint) = err.hint() {
        eprintln!("hint: {hint}");
      }
      process::ExitCode::FAILURE
    }
  }
}
