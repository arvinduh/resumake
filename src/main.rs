//! Resumake CLI entry point and command router.

use clap::Parser;
use resumake::cli::Cli;
use resumake::commands::execute_command;
use resumake::utils::ui::print_error;
use std::process::ExitCode;

fn main() -> ExitCode {
  let cli = Cli::parse();
  let command = cli.command.unwrap_or_default();

  match execute_command(command, cli.quiet) {
    Ok(()) => ExitCode::SUCCESS,
    Err(err) => {
      print_error(&format!("{err}"));
      ExitCode::FAILURE
    }
  }
}
