//! Resumake CLI entry point and process host.

mod cli;

use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
  let cli = cli::Cli::parse();

  match cli.run() {
    Ok(()) => ExitCode::SUCCESS,
    Err(err) => {
      cli::ui::print_error(&format!("{err}"));
      ExitCode::FAILURE
    }
  }
}
