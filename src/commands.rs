//! CLI command orchestration and dispatcher.

pub mod build;
pub mod init;
pub mod release;
pub mod template;
pub mod update;

use crate::cli::{Commands, TemplateCommands};
use crate::commands::build::{
  run_build, run_check, run_check_watch, run_watch,
};
use crate::commands::init::{resolve_init_output, run_init, InitOptions};
use crate::commands::release::run_release;
use crate::commands::template::{run_template_eject, run_template_list};
use crate::commands::update::run_update;
use crate::engine::templates::DEFAULT_TEMPLATE;
use crate::error::ResumakeError;

/// Dispatches a parsed CLI command to its specific handler.
pub fn execute_command(
  command: Commands,
  quiet: bool,
) -> Result<(), ResumakeError> {
  match command {
    Commands::Build {
      content,
      check,
      watch,
      template,
      source,
      output,
      schema,
      font_path,
    } => {
      let template_name = template.as_deref().unwrap_or(DEFAULT_TEMPLATE);
      if watch {
        if check {
          run_check_watch(
            &content,
            template_name,
            source.as_deref(),
            schema.as_deref(),
            font_path.as_deref(),
            quiet,
          )
        } else {
          run_watch(
            &content,
            template_name,
            source.as_deref(),
            output.as_deref(),
            schema.as_deref(),
            font_path.as_deref(),
            quiet,
          )
        }
      } else if check {
        run_check(
          &content,
          template_name,
          source.as_deref(),
          schema.as_deref(),
          font_path.as_deref(),
          quiet,
        )
      } else {
        run_build(
          &content,
          template_name,
          source.as_deref(),
          output.as_deref(),
          schema.as_deref(),
          font_path.as_deref(),
          quiet,
        )
      }
    }
    Commands::Init {
      dest,
      name,
      output,
      force,
      no_git,
      no_workflows,
      update,
    } => {
      let resolved_output =
        resolve_init_output(dest.as_deref(), output.as_deref());
      run_init(InitOptions {
        name: name.as_deref(),
        output: &resolved_output,
        force,
        no_git,
        no_workflows,
        update,
        quiet,
      })
      .map_err(Into::into)
    }
    Commands::Release {
      content,
      message,
      dry_run,
      skip_build,
    } => run_release(&content, message.as_deref(), dry_run, skip_build, quiet)
      .map_err(Into::into),
    Commands::Template(args) => match args.command {
      TemplateCommands::List => run_template_list(),
      TemplateCommands::Eject { name, force } => {
        run_template_eject(&name, force, quiet)
      }
    },
    Commands::Update { check, force } => {
      run_update(check, force, quiet).map_err(Into::into)
    }
  }
}
