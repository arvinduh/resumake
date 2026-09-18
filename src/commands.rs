//! CLI command orchestration and dispatcher.

pub(crate) mod build;
pub(crate) mod init;
pub(crate) mod release;
pub(crate) mod template;
pub(crate) mod update;

use crate::cli::Commands;
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
      template,
      output,
      schema,
      font_path,
    } => {
      let template_name =
        template.as_deref().unwrap_or(templates::DEFAULT_TEMPLATE);
      build::run_build(
        &content,
        template_name,
        output.as_deref(),
        schema.as_deref(),
        font_path.as_deref(),
        quiet,
      )
    }
    Commands::Check {
      content,
      template,
      schema,
      font_path,
    } => {
      let template_name =
        template.as_deref().unwrap_or(templates::DEFAULT_TEMPLATE);
      build::run_check(
        &content,
        template_name,
        schema.as_deref(),
        font_path.as_deref(),
        quiet,
      )
    }
    Commands::Init {
      dest,
      name,
      force,
      no_git,
      no_workflows,
      workflows,
    } => {
      let resolved_output = init::resolve_init_output(dest.as_deref());
      init::run_init(InitOptions {
        name: name.as_deref(),
        output: &resolved_output,
        force,
        no_git,
        no_workflows,
        workflows,
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
    Commands::Template { name, list, force } => {
      if list {
        template::run_template_list()
      } else if let Some(name) = name {
        template::run_template_eject(&name, force, quiet)
      } else {
        println!("Usage: rsmk template [NAME] [--list] [--force]\n\nRun `rsmk template --help` for more information.");
        Ok(())
      }
    }
    Commands::Update { check, force } => {
      update::run_update(check, force, quiet).map_err(Into::into)
    }
    Commands::Schema { output } => {
      let schema_str = crate::schema::export_builtin_schema(output.as_deref())
        .map_err(ResumakeError::from)?;
      if output.is_none() {
        println!("{schema_str}");
      }
      Ok(())
    }
  }
}
