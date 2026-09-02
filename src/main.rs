//! Resumake CLI entry point and command router.

use clap::Parser;
use notify_debouncer_mini::{
  new_debouncer, notify::RecursiveMode, DebounceEventResult,
};
use resumake::cli::{Cli, Commands, TemplateCommands};
use resumake::engine::{
  eject_template, list_templates, EngineError, TypstEngine, DEFAULT_TEMPLATE,
};
use resumake::error::{ResumakeError, WatchError};
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
use std::path::Path;
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

fn execute_command(command: Commands, quiet: bool) -> Result<(), ResumakeError> {
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
      resumake::update::run_update(check, force, quiet).map_err(Into::into)
    }
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
) -> Result<(), ResumakeError> {
  if !content.exists() {
    return Err(
      EngineError::ContentNotFound {
        path: content.to_path_buf(),
      }
      .into(),
    );
  }

  // 1. Validate schema
  validate_schema_auto(content, schema)?;

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
    return Err(EngineError::LayoutConstraintViolation.into());
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
) -> Result<(), ResumakeError> {
  if !content.exists() {
    return Err(
      EngineError::ContentNotFound {
        path: content.to_path_buf(),
      }
      .into(),
    );
  }

  // 1. Schema check
  validate_schema_auto(content, schema)?;

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
    return Err(EngineError::LayoutConstraintViolation.into());
  }

  if !quiet {
    print_success("Dry-run check passed: schema & single-page layout valid.");
  }
  Ok(())
}

fn setup_watcher(
  content: &Path,
  template_name: &str,
  source: Option<&Path>,
  schema: Option<&Path>,
  font_path: Option<&Path>,
) -> Result<
  (
    notify_debouncer_mini::Debouncer<
      notify_debouncer_mini::notify::RecommendedWatcher,
    >,
    std::sync::mpsc::Receiver<DebounceEventResult>,
  ),
  ResumakeError,
> {
  let (tx, rx) = std::sync::mpsc::channel();
  let mut debouncer = new_debouncer(std::time::Duration::from_millis(200), tx)
    .map_err(WatchError::Init)?;

  debouncer
    .watcher()
    .watch(content, RecursiveMode::NonRecursive)
    .map_err(|e| WatchError::WatchPath {
      path: content.to_path_buf(),
      source: e,
    })?;

  let root = resumake::engine::find_project_root();

  if let Some(src) = source {
    if src.exists() {
      if src.is_dir() {
        let _ = debouncer.watcher().watch(src, RecursiveMode::Recursive);
      } else {
        let _ = debouncer.watcher().watch(src, RecursiveMode::NonRecursive);
        if let Some(parent) = src
          .parent()
          .filter(|p| !p.as_os_str().is_empty() && p.exists())
        {
          let _ = debouncer.watcher().watch(parent, RecursiveMode::Recursive);
        }
      }
    }
  }

  if let Ok(engine) = TypstEngine::new(font_path) {
    if let Ok(resolved) = engine.resolve_template(template_name, source) {
      if resolved.exists() {
        if let Some(parent) = resolved
          .parent()
          .filter(|p| !p.as_os_str().is_empty() && p.exists())
        {
          let _ = debouncer.watcher().watch(parent, RecursiveMode::Recursive);
        }
      }
    }
    if let Some(ref font_dir) = engine.font_path {
      if font_dir.exists() && font_dir.is_dir() {
        let _ = debouncer
          .watcher()
          .watch(font_dir, RecursiveMode::Recursive);
      }
    }
  }

  let templates_dir = root.join("templates");
  if templates_dir.exists() && templates_dir.is_dir() {
    let _ = debouncer
      .watcher()
      .watch(&templates_dir, RecursiveMode::Recursive);
  }

  if let Some(s) = schema {
    if s.exists() {
      let _ = debouncer.watcher().watch(s, RecursiveMode::NonRecursive);
    }
  } else {
    for candidate in &["resume.schema.json", "schema.json"] {
      let p = root.join(candidate);
      if p.exists() {
        let _ = debouncer.watcher().watch(&p, RecursiveMode::NonRecursive);
      }
    }
  }

  if let Some(f) = font_path {
    if f.exists() && f.is_dir() {
      let _ = debouncer.watcher().watch(f, RecursiveMode::Recursive);
    }
  }

  Ok((debouncer, rx))
}

fn run_watch(
  content: &Path,
  template_name: &str,
  source: Option<&Path>,
  output: Option<&Path>,
  schema: Option<&Path>,
  font_path: Option<&Path>,
  quiet: bool,
) -> Result<(), ResumakeError> {
  if !content.exists() {
    return Err(
      EngineError::ContentNotFound {
        path: content.to_path_buf(),
      }
      .into(),
    );
  }

  let output_pdf = match output {
    Some(out) => out.to_path_buf(),
    None => derive_output_filename(content),
  };

  print_info(&format!(
    "Watching '{}' -> '{}'. Press Ctrl+C to stop.",
    content.display(),
    output_pdf.display()
  ));

  let (_debouncer, rx) =
    setup_watcher(content, template_name, source, schema, font_path)?;

  if let Err(err) = run_build(
    content,
    template_name,
    source,
    Some(&output_pdf),
    schema,
    font_path,
    quiet,
  ) {
    print_error(&format!("{err}"));
  }

  let canonical_output = output_pdf.canonicalize().ok();

  for events_res in rx {
    match events_res {
      Ok(events) => {
        let has_relevant_change = events.iter().any(|event| {
          if let Some(ref canon_out) = canonical_output {
            if let Ok(canon_event) = event.path.canonicalize() {
              if &canon_event == canon_out {
                return false;
              }
            }
          }
          if event.path == output_pdf {
            return false;
          }
          true
        });

        if has_relevant_change {
          if let Err(err) = run_build(
            content,
            template_name,
            source,
            Some(&output_pdf),
            schema,
            font_path,
            quiet,
          ) {
            print_error(&format!("{err}"));
          }
        }
      }
      Err(err) => {
        print_error(&format!("Watch error: {err}"));
      }
    }
  }

  Ok(())
}

fn run_check_watch(
  content: &Path,
  template_name: &str,
  source: Option<&Path>,
  schema: Option<&Path>,
  font_path: Option<&Path>,
  quiet: bool,
) -> Result<(), ResumakeError> {
  if !content.exists() {
    return Err(
      EngineError::ContentNotFound {
        path: content.to_path_buf(),
      }
      .into(),
    );
  }

  print_info(&format!(
    "Watching '{}' in check mode. Press Ctrl+C to stop.",
    content.display()
  ));

  let (_debouncer, rx) =
    setup_watcher(content, template_name, source, schema, font_path)?;

  if let Err(err) =
    run_check(content, template_name, source, schema, font_path, quiet)
  {
    print_error(&format!("{err}"));
  }

  for events_res in rx {
    match events_res {
      Ok(_events) => {
        if let Err(err) =
          run_check(content, template_name, source, schema, font_path, quiet)
        {
          print_error(&format!("{err}"));
        }
      }
      Err(err) => {
        print_error(&format!("Watch error: {err}"));
      }
    }
  }

  Ok(())
}

fn run_template_list() -> Result<(), ResumakeError> {
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
) -> Result<(), ResumakeError> {
  let target_dir = Path::new("templates").join(name);
  let ejected_files = eject_template(name, &target_dir, force)?;

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
