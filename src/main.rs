//! Resumake CLI entry point and command router.

use clap::Parser;
use resumake::cli::Cli;
use resumake::commands;
use resumake::utils::ui;
use std::process::ExitCode;

fn main() -> ExitCode {
  let cli = Cli::parse();
  let command = cli.command.unwrap_or_default();

  match commands::execute_command(command, cli.quiet) {
    Ok(()) => ExitCode::SUCCESS,
    Err(err) => {
      ui::print_error(&format!("{err}"));
      ExitCode::FAILURE
    }
  }
}
