//! CLI command orchestration and dispatcher.

pub(crate) mod build;
pub(crate) mod init;
pub(crate) mod release;
pub(crate) mod template;
pub(crate) mod update;

use crate::cli::{Commands, TemplateCommands};
use crate::commands::init::InitOptions;
use crate::engine::templates;
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
      let template_name =
        template.as_deref().unwrap_or(templates::DEFAULT_TEMPLATE);
      match (watch, check) {
        (true, true) => build::run_check_watch(
          &content,
          template_name,
          source.as_deref(),
          schema.as_deref(),
          font_path.as_deref(),
          quiet,
        ),
        (true, false) => build::run_watch(
          &content,
          template_name,
          source.as_deref(),
          output.as_deref(),
          schema.as_deref(),
          font_path.as_deref(),
          quiet,
        ),
        (false, true) => build::run_check(
          &content,
          template_name,
          source.as_deref(),
          schema.as_deref(),
          font_path.as_deref(),
          quiet,
        ),
        (false, false) => build::run_build(
          &content,
          template_name,
          source.as_deref(),
          output.as_deref(),
          schema.as_deref(),
          font_path.as_deref(),
          quiet,
        ),
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
        init::resolve_init_output(dest.as_deref(), output.as_deref());
      init::run_init(InitOptions {
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
    } => release::run_release(
      &content,
      message.as_deref(),
      dry_run,
      skip_build,
      quiet,
    )
    .map_err(Into::into),
    Commands::Template(args) => match args.command {
      TemplateCommands::List => template::run_template_list(),
      TemplateCommands::Eject { name, force } => {
        template::run_template_eject(&name, force, quiet)
      }
    },
    Commands::Update { check, force } => {
      update::run_update(check, force, quiet).map_err(Into::into)
    }
  }
}
