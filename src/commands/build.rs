//! Handlers for `rsmk build`, watch mode, and check mode.

use crate::engine;
use crate::engine::error::EngineError;
use crate::engine::TypstEngine;
use crate::error::ResumakeError;
use crate::schema;
use crate::telemetry;
use crate::utils::fs;
use crate::utils::ui;
use notify_debouncer_mini::{
  new_debouncer, notify::RecursiveMode, DebounceEventResult,
};
use std::path::{Path, PathBuf};

/// Errors originating from file watching and hot-reload debouncing.
#[derive(thiserror::Error, Debug)]
pub enum WatchError {
  /// Failed to initialize file watcher.
  #[error("Failed to initialize file watcher: {0}")]
  Init(#[source] notify_debouncer_mini::notify::Error),
  /// Failed to register watch path.
  #[error("Failed to watch path '{}': {source}", path.display())]
  WatchPath {
    /// Target path.
    path: PathBuf,
    /// Underlying notify error.
    #[source]
    source: notify_debouncer_mini::notify::Error,
  },
}

/// Compiles the document, optionally renders PDF, evaluates layout telemetry, and prints report.
fn compile_and_evaluate(
  content: &Path,
  template_name: &str,
  source: Option<&Path>,
  schema: Option<&Path>,
  font_path: Option<&Path>,
  output_pdf: Option<&Path>,
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
  schema::validate_schema_auto(content, schema)?;

  // 2. Resolve paths and compile document once in-memory
  let engine = TypstEngine::new(font_path)?;
  let resolved_template = engine.resolve_template(template_name, source)?;
  let doc = engine.compile_paged(&resolved_template, content)?;

  // 3. Render PDF directly from the in-memory document if requested
  if let Some(out) = output_pdf {
    engine.render_pdf(&doc, out)?;
  }

  // 4. Query telemetry directly from the same in-memory document
  let page_json = engine::query_doc_metadata(&doc, "<pageinfo>")?;
  let bullets_json = engine::query_doc_metadata(&doc, "<bulletinfo>")?;
  let report = telemetry::evaluate_telemetry(&page_json, &bullets_json)?;

  let name = schema::load_content_name(content)
    .unwrap_or_else(|_| "Candidate".to_string());
  let version = schema::load_content_version(content)
    .unwrap_or_else(|_| "1.0.0".to_string());

  let output_display = match output_pdf {
    Some(out) => out.to_string_lossy(),
    None => "[dry-run: no PDF written]".into(),
  };

  if !quiet {
    ui::print_telemetry_table(&report, &name, &output_display, &version);
  }

  if !report.is_pass() {
    return Err(EngineError::LayoutConstraintViolation.into());
  }

  Ok(())
}

/// Runs `rsmk build` to compile the document to a PDF and verify layout telemetry.
pub(crate) fn run_build(
  content: &Path,
  template_name: &str,
  source: Option<&Path>,
  output: Option<&Path>,
  schema: Option<&Path>,
  font_path: Option<&Path>,
  quiet: bool,
) -> Result<(), ResumakeError> {
  let output_pdf = match output {
    Some(out) => out.to_path_buf(),
    None => schema::derive_output_filename(content),
  };
  compile_and_evaluate(
    content,
    template_name,
    source,
    schema,
    font_path,
    Some(&output_pdf),
    quiet,
  )
}

/// Runs `rsmk build --check` to verify schema and layout geometry without generating a PDF.
pub(crate) fn run_check(
  content: &Path,
  template_name: &str,
  source: Option<&Path>,
  schema: Option<&Path>,
  font_path: Option<&Path>,
  quiet: bool,
) -> Result<(), ResumakeError> {
  compile_and_evaluate(
    content,
    template_name,
    source,
    schema,
    font_path,
    None,
    quiet,
  )?;

  if !quiet {
    ui::print_success(
      "Dry-run check passed: schema & single-page layout valid.",
    );
  }
  Ok(())
}

/// Options passed to the file watcher and debounced watch loop.
struct WatchOptions<'a> {
  content: &'a Path,
  template_name: &'a str,
  source: Option<&'a Path>,
  schema: Option<&'a Path>,
  font_path: Option<&'a Path>,
  ignored_output: Option<&'a Path>,
  watch_msg: &'a str,
}

fn setup_watcher(
  opts: &WatchOptions,
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
    .watch(opts.content, RecursiveMode::NonRecursive)
    .map_err(|e| WatchError::WatchPath {
      path: opts.content.to_path_buf(),
      source: e,
    })?;

  let root = fs::find_project_root();

  if let Some(src) = opts.source {
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

  if let Ok(engine) = TypstEngine::new(opts.font_path) {
    if let Ok(resolved) =
      engine.resolve_template(opts.template_name, opts.source)
    {
      if resolved.exists() {
        if let Some(parent) = resolved
          .parent()
          .filter(|p| !p.as_os_str().is_empty() && p.exists())
        {
          let _ = debouncer.watcher().watch(parent, RecursiveMode::Recursive);
        }
      }
    }
    if let Some(font_dir) = engine.font_path() {
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

  if let Some(s) = opts.schema {
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

  if let Some(f) = opts.font_path {
    if f.exists() && f.is_dir() {
      let _ = debouncer.watcher().watch(f, RecursiveMode::Recursive);
    }
  }

  Ok((debouncer, rx))
}

/// Runs a shared debounced file-watching loop for build and check commands.
fn run_watch_loop<F>(
  opts: &WatchOptions,
  mut action: F,
) -> Result<(), ResumakeError>
where
  F: FnMut() -> Result<(), ResumakeError>,
{
  if !opts.content.exists() {
    return Err(
      EngineError::ContentNotFound {
        path: opts.content.to_path_buf(),
      }
      .into(),
    );
  }

  ui::print_info(opts.watch_msg);

  let (_debouncer, rx) = setup_watcher(opts)?;

  if let Err(err) = action() {
    ui::print_error(&format!("{err}"));
  }

  let canonical_ignored =
    opts.ignored_output.and_then(|p| p.canonicalize().ok());

  for events_res in rx {
    match events_res {
      Ok(events) => {
        let has_relevant_change = match opts.ignored_output {
          Some(ignored) => events.iter().any(|event| {
            if let Some(ref canon_out) = canonical_ignored {
              if let Ok(canon_event) = event.path.canonicalize() {
                if &canon_event == canon_out {
                  return false;
                }
              }
            }
            if event.path == ignored {
              return false;
            }
            true
          }),
          None => true,
        };

        if has_relevant_change {
          if let Err(err) = action() {
            ui::print_error(&format!("{err}"));
          }
        }
      }
      Err(err) => {
        ui::print_error(&format!("Watch error: {err}"));
      }
    }
  }

  Ok(())
}

/// Runs `rsmk build --watch` to continuously recompile on file changes.
pub(crate) fn run_watch(
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
    None => schema::derive_output_filename(content),
  };

  let msg = format!(
    "Watching '{}' -> '{}'. Press Ctrl+C to stop.",
    content.display(),
    output_pdf.display()
  );

  let opts = WatchOptions {
    content,
    template_name,
    source,
    schema,
    font_path,
    ignored_output: Some(&output_pdf),
    watch_msg: &msg,
  };

  run_watch_loop(&opts, || {
    run_build(
      content,
      template_name,
      source,
      Some(&output_pdf),
      schema,
      font_path,
      quiet,
    )
  })
}

/// Runs `rsmk build --check --watch` to continuously verify layout on file changes.
pub(crate) fn run_check_watch(
  content: &Path,
  template_name: &str,
  source: Option<&Path>,
  schema: Option<&Path>,
  font_path: Option<&Path>,
  quiet: bool,
) -> Result<(), ResumakeError> {
  let msg = format!(
    "Watching '{}' in check mode. Press Ctrl+C to stop.",
    content.display()
  );

  let opts = WatchOptions {
    content,
    template_name,
    source,
    schema,
    font_path,
    ignored_output: None,
    watch_msg: &msg,
  };

  run_watch_loop(&opts, || {
    run_check(content, template_name, source, schema, font_path, quiet)
  })
}
