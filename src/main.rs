//! Resumake CLI entry point and command router.

use clap::Parser;
use resumake::cli::{Cli, Commands, TemplateCommands};
use resumake::engine::{
  eject_template, list_templates, TypstEngine, DEFAULT_TEMPLATE,
};
use resumake::init::{resolve_init_output, run_init, InitOptions};
use resumake::release::run_release;
use resumake::schema::{
  derive_output_filename, load_content_name, load_content_version,
  validate_schema_auto,
};
use resumake::telemetry::evaluate_telemetry;
use resumake::ui::{
  print_error, print_info, print_success, print_telemetry_table,
};
use std::fs;
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
  let cli = Cli::parse();
  let command = cli.command.unwrap_or_default();

  match execute_command(command, cli.quiet) {
    Ok(()) => ExitCode::SUCCESS,
    Err(err) => {
      print_error(&err);
      ExitCode::FAILURE
    }
  }
}

fn execute_command(command: Commands, quiet: bool) -> Result<(), String> {
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
            font_path.as_deref(),
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
    }
    Commands::Release {
      content,
      message,
      dry_run,
      skip_build,
    } => run_release(&content, message.as_deref(), dry_run, skip_build, quiet),
    Commands::Template(args) => match args.command {
      TemplateCommands::List => run_template_list(),
      TemplateCommands::Eject { name, force } => {
        run_template_eject(&name, force, quiet)
      }
    },
  }
}

fn run_build(
  content: &Path,
  template_name: &str,
  source: Option<&Path>,
  output: Option<&Path>,
  schema: Option<&Path>,
  font_path: Option<&Path>,
  quiet: bool,
) -> Result<(), String> {
  if !content.exists() {
    return Err(format!(
      "Content file not found: '{}'. Run 'rsmk init' to create one.",
      content.display()
    ));
  }

  // 1. Validate schema
  if let Err(errors) = validate_schema_auto(content, schema) {
    let mut msg =
      format!("Schema validation failed ({} error(s)):\n", errors.len());
    for e in errors {
      msg.push_str(&format!("  - {e}\n"));
    }
    return Err(msg);
  }

  // 2. Resolve paths
  let engine = TypstEngine::new(font_path)?;
  let resolved_template = engine.resolve_template(template_name, source)?;
  let output_pdf = match output {
    Some(out) => out.to_path_buf(),
    None => derive_output_filename(content),
  };

  // 3. Compile document
  engine.compile(&resolved_template, content, &output_pdf)?;

  // 4. Query telemetry
  let page_json =
    engine.query_metadata(&resolved_template, content, "<pageinfo>")?;
  let bullets_json =
    engine.query_metadata(&resolved_template, content, "<bulletinfo>")?;
  let report = evaluate_telemetry(&page_json, &bullets_json)?;

  let name =
    load_content_name(content).unwrap_or_else(|_| "Candidate".to_string());
  let version =
    load_content_version(content).unwrap_or_else(|_| "1.0.0".to_string());

  if !quiet {
    print_telemetry_table(
      &report,
      &name,
      &output_pdf.to_string_lossy(),
      &version,
    );
  }

  if !report.is_pass() {
    return Err(
      "Document failed strict single-page geometry constraints.".to_string(),
    );
  }

  Ok(())
}

fn run_check(
  content: &Path,
  template_name: &str,
  source: Option<&Path>,
  schema: Option<&Path>,
  font_path: Option<&Path>,
  quiet: bool,
) -> Result<(), String> {
  if !content.exists() {
    return Err(format!("Content file not found: '{}'", content.display()));
  }

  // 1. Schema check
  validate_schema_auto(content, schema).map_err(|errors| {
    format!(
      "Schema validation failed:\n{}",
      errors
        .iter()
        .map(|e| format!("  - {e}"))
        .collect::<Vec<_>>()
        .join("\n")
    )
  })?;

  // 2. Layout telemetry check
  let engine = TypstEngine::new(font_path)?;
  let resolved_template = engine.resolve_template(template_name, source)?;
  let page_json =
    engine.query_metadata(&resolved_template, content, "<pageinfo>")?;
  let bullets_json =
    engine.query_metadata(&resolved_template, content, "<bulletinfo>")?;
  let report = evaluate_telemetry(&page_json, &bullets_json)?;

  let name =
    load_content_name(content).unwrap_or_else(|_| "Candidate".to_string());
  let version =
    load_content_version(content).unwrap_or_else(|_| "1.0.0".to_string());

  if !quiet {
    print_telemetry_table(
      &report,
      &name,
      "[dry-run: no PDF written]",
      &version,
    );
  }

  if !report.is_pass() {
    return Err(
      "Dry-run check failed strict single-page layout constraints.".to_string(),
    );
  }

  if !quiet {
    print_success("Dry-run check passed: schema & single-page layout valid.");
  }
  Ok(())
}

fn run_watch(
  content: &Path,
  template_name: &str,
  source: Option<&Path>,
  output: Option<&Path>,
  font_path: Option<&Path>,
) -> Result<(), String> {
  if !content.exists() {
    return Err(format!("Content file not found: '{}'", content.display()));
  }

  let engine = TypstEngine::new(font_path)?;
  let resolved_template = engine.resolve_template(template_name, source)?;
  let output_pdf = match output {
    Some(out) => out.to_path_buf(),
    None => derive_output_filename(content),
  };

  print_info(&format!(
    "Watching '{}' -> '{}'. Press Ctrl+C to stop.",
    content.display(),
    output_pdf.display()
  ));

  engine.watch(&resolved_template, content, &output_pdf)
}

fn run_check_watch(
  content: &Path,
  template_name: &str,
  source: Option<&Path>,
  schema: Option<&Path>,
  font_path: Option<&Path>,
  quiet: bool,
) -> Result<(), String> {
  if !content.exists() {
    return Err(format!("Content file not found: '{}'", content.display()));
  }

  print_info(&format!(
    "Watching '{}' in check mode. Press Ctrl+C to stop.",
    content.display()
  ));

  if let Err(err) =
    run_check(content, template_name, source, schema, font_path, quiet)
  {
    print_error(&err);
  }

  let mut last_content_mtime =
    fs::metadata(content).and_then(|m| m.modified()).ok();
  let mut last_source_mtime =
    source.and_then(|s| fs::metadata(s).and_then(|m| m.modified()).ok());
  let mut last_schema_mtime =
    schema.and_then(|s| fs::metadata(s).and_then(|m| m.modified()).ok());

  loop {
    std::thread::sleep(std::time::Duration::from_millis(500));

    let cur_content_mtime =
      fs::metadata(content).and_then(|m| m.modified()).ok();
    let cur_source_mtime =
      source.and_then(|s| fs::metadata(s).and_then(|m| m.modified()).ok());
    let cur_schema_mtime =
      schema.and_then(|s| fs::metadata(s).and_then(|m| m.modified()).ok());

    if cur_content_mtime != last_content_mtime
      || cur_source_mtime != last_source_mtime
      || cur_schema_mtime != last_schema_mtime
    {
      last_content_mtime = cur_content_mtime;
      last_source_mtime = cur_source_mtime;
      last_schema_mtime = cur_schema_mtime;

      if let Err(err) =
        run_check(content, template_name, source, schema, font_path, quiet)
      {
        print_error(&err);
      }
    }
  }
}

fn run_template_list() -> Result<(), String> {
  let templates = list_templates();
  println!("Available templates:");
  for tpl in templates {
    println!("  - {tpl}");
  }
  Ok(())
}

fn run_template_eject(
  name: &str,
  force: bool,
  quiet: bool,
) -> Result<(), String> {
  let target_dir = Path::new("templates").join(name);
  let ejected_files =
    eject_template(name, &target_dir, force).map_err(|e| e.to_string())?;

  if !quiet {
    println!("✓ Ejected template '{name}' to ./templates/{name}/");
    for file in ejected_files {
      println!("  - {file}");
    }
    println!(
      "Run `rsmk build --template ./templates/{name}/main.typ` to compile with your local template."
    );
  }
  Ok(())
}
