//! In-process Typst compilation engine and PDF rendering.
//!
//! Owns the [`typst::World`] implementation, font loading via `typst-kit`,
//! executing compilation passes, and exporting PDF documents.

use std::collections;
use std::fmt::Write;
use std::fs;
use std::path;
use std::sync;
use std::time;

use thiserror;
use typst::diag;
use typst::foundations;
use typst::syntax;
use typst::text;
use typst::utils;
use typst::{LibraryExt, World, WorldExt};
use typst_kit::fonts;
use typst_layout;

use crate::schema;
use crate::telemetry;
use crate::templates;

/// Errors originating from Typst rendering or PDF compilation.
#[derive(Debug, thiserror::Error)]
pub enum Error {
  /// Template resolution failure.
  #[error(transparent)]
  Template(#[from] templates::Error),
  /// Typst compilation failed with diagnostics.
  #[error("Typst compilation failed:\n{stderr}")]
  Compilation {
    /// Formatted diagnostics.
    stderr: String,
  },
  /// Telemetry evaluation failed.
  #[error(transparent)]
  Telemetry(#[from] telemetry::Error),
  /// I/O error during file read or write.
  #[error(transparent)]
  Io(#[from] std::io::Error),
}

/// Creates a project-rooted [`syntax::FileId`].
fn project_file_id(vpath: &str) -> syntax::FileId {
  let path = syntax::VirtualPath::new(vpath)
    .or_else(|_| {
      syntax::VirtualPath::new(vpath.rsplit('/').next().unwrap_or(""))
    })
    .unwrap_or_else(|_| {
      syntax::VirtualPath::new("/content.yaml").expect("constant path is valid")
    });
  syntax::FileId::new(syntax::RootedPath::new(
    syntax::VirtualRoot::Project,
    path,
  ))
}

/// Matches fallback `content.{yaml,yml,json}` virtual paths.
fn is_default_content_vpath(trimmed: &str) -> bool {
  matches!(
    trimmed.rsplit('/').next(),
    Some("content.yaml" | "content.yml" | "content.json")
  )
}

/// Converts days since Unix epoch to a [`foundations::Datetime`].
fn days_to_date(days_since_epoch: i64) -> Option<foundations::Datetime> {
  let z = days_since_epoch + 719_468;
  let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
  let doe = u32::try_from(z - era * 146_097).ok()?;
  let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
  let y = i64::from(yoe) + era * 400;
  let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
  let mp = (5 * doy + 2) / 153;
  let d = doy - (153 * mp + 2) / 5 + 1;
  let m = if mp < 10 { mp + 3 } else { mp - 9 };
  let yr = if m <= 2 { y + 1 } else { y };

  let yr_i32 = i32::try_from(yr).ok()?;
  let m_u8 = u8::try_from(m).ok()?;
  let d_u8 = u8::try_from(d).ok()?;

  foundations::Datetime::from_ymd(yr_i32, m_u8, d_u8)
}

/// In-process Typst [`World`] implementation.
struct ResumakeWorld {
  /// Standard library configuration with CLI sys.inputs.
  library: utils::LazyHash<typst::Library>,
  /// Font store containing embedded, system, and local fonts.
  fonts: fonts::FontStore,
  /// Main template entry point `FileId`.
  main_id: syntax::FileId,
  /// Target content YAML `FileId`.
  content_id: syntax::FileId,
  /// Workspace project root directory.
  root_path: path::PathBuf,
  /// Resolved template source (embedded or disk).
  template_source: templates::TemplateSource,
  /// Path to content YAML file.
  content_path: path::PathBuf,
  /// Normalized virtual path to content file.
  content_vpath: String,
  /// Source file cache.
  sources: sync::Mutex<
    collections::HashMap<syntax::FileId, diag::FileResult<syntax::Source>>,
  >,
  /// Binary file cache.
  files: sync::Mutex<
    collections::HashMap<syntax::FileId, diag::FileResult<foundations::Bytes>>,
  >,
  /// Fixed evaluation timestamp for reproducible builds.
  now: time::SystemTime,
}

impl ResumakeWorld {
  /// Constructs a new [`ResumakeWorld`].
  #[must_use]
  fn new(
    project_root: path::PathBuf,
    template_source: templates::TemplateSource,
    content_path: path::PathBuf,
    font_override: Option<&path::Path>,
  ) -> Self {
    let mut font_store = fonts::FontStore::new();
    if let Some(custom) = font_override {
      font_store.extend(fonts::scan(custom));
    }
    let local_fonts = project_root.join("fonts");
    if local_fonts.is_dir() {
      font_store.extend(fonts::scan(&local_fonts));
    }
    font_store.extend(fonts::system());
    font_store.extend(fonts::embedded());

    let content_vpath = content_path.strip_prefix(&project_root).map_or_else(
      |_| {
        format!(
          "/{}",
          content_path
            .file_name()
            .map_or("content.yaml", |n| n.to_str().unwrap_or("content.yaml"))
        )
      },
      |rel| format!("/{}", rel.to_string_lossy().replace('\\', "/")),
    );

    let content_id = project_file_id(&content_vpath);

    let main_vpath = match &template_source {
      templates::TemplateSource::Embedded { name, .. } => {
        format!("/{name}.typ")
      }
      templates::TemplateSource::File(p) => {
        p.strip_prefix(&project_root).map_or_else(
          |_| {
            format!(
              "/{}",
              p.file_name()
                .map_or("main.typ", |n| n.to_str().unwrap_or("main.typ"))
            )
          },
          |rel| format!("/{}", rel.to_string_lossy().replace('\\', "/")),
        )
      }
    };

    let main_id = project_file_id(&main_vpath);

    let mut inputs = foundations::Dict::new();
    inputs.insert(
      foundations::Str::from("content"),
      foundations::Value::Str(foundations::Str::from(content_vpath.as_str())),
    );

    let library = utils::LazyHash::new(
      typst::Library::builder().with_inputs(inputs).build(),
    );

    Self {
      library,
      fonts: font_store,
      main_id,
      content_id,
      root_path: project_root,
      template_source,
      content_path,
      content_vpath,
      sources: sync::Mutex::new(collections::HashMap::new()),
      files: sync::Mutex::new(collections::HashMap::new()),
      now: time::SystemTime::now(),
    }
  }

  /// Resolves raw file bytes without checking cache.
  fn read_bytes_uncached(
    &self,
    id: syntax::FileId,
  ) -> diag::FileResult<foundations::Bytes> {
    if id == self.main_id {
      return match &self.template_source {
        templates::TemplateSource::Embedded { source, .. } => {
          Ok(foundations::Bytes::new(source.as_bytes().to_vec()))
        }
        templates::TemplateSource::File(p) => fs::read(p)
          .map(foundations::Bytes::new)
          .map_err(|e| diag::FileError::from_io(e, p)),
      };
    }

    let trimmed = id.vpath().get_without_slash();
    let rooted = format!("/{trimmed}");

    if id == self.content_id
      || rooted == self.content_vpath
      || is_default_content_vpath(trimmed)
    {
      return fs::read(&self.content_path)
        .map(foundations::Bytes::new)
        .map_err(|e| diag::FileError::from_io(e, &self.content_path));
    }

    let disk_path = self.root_path.join(trimmed);
    if disk_path.is_file() {
      return fs::read(&disk_path)
        .map(foundations::Bytes::new)
        .map_err(|e| diag::FileError::from_io(e, &disk_path));
    }

    Err(diag::FileError::NotFound(path::PathBuf::from(
      id.vpath().get_without_slash(),
    )))
  }

  /// Returns requested font families absent from the loaded font book.
  #[must_use]
  fn missing_fonts(&self, families: &[String]) -> Vec<String> {
    let book = self.book();
    families
      .iter()
      .filter(|fam| !book.contains_family(&fam.to_lowercase()))
      .cloned()
      .collect()
  }
}

impl World for ResumakeWorld {
  fn library(&self) -> &utils::LazyHash<typst::Library> {
    &self.library
  }

  fn book(&self) -> &utils::LazyHash<text::FontBook> {
    self.fonts.book()
  }

  fn main(&self) -> syntax::FileId {
    self.main_id
  }

  fn source(&self, id: syntax::FileId) -> diag::FileResult<syntax::Source> {
    if let Ok(guard) = self.sources.lock()
      && let Some(cached) = guard.get(&id)
    {
      return cached.clone();
    }

    let bytes = self.file(id)?;
    let text = match String::from_utf8(bytes.to_vec()) {
      Ok(s) => s,
      Err(err) => {
        let res = Err(diag::FileError::InvalidUtf8);
        if let Ok(mut guard) = self.sources.lock() {
          guard.insert(id, res.clone());
        }
        let _ = err;
        return res;
      }
    };

    let source = syntax::Source::new(id, text);
    let res = Ok(source);
    if let Ok(mut guard) = self.sources.lock() {
      guard.insert(id, res.clone());
    }
    res
  }

  fn file(&self, id: syntax::FileId) -> diag::FileResult<foundations::Bytes> {
    if let Ok(guard) = self.files.lock()
      && let Some(cached) = guard.get(&id)
    {
      return cached.clone();
    }

    let res = self.read_bytes_uncached(id);
    if let Ok(mut guard) = self.files.lock() {
      guard.insert(id, res.clone());
    }
    res
  }

  fn font(&self, index: usize) -> Option<text::Font> {
    self.fonts.font(index)
  }

  fn today(
    &self,
    offset: Option<foundations::Duration>,
  ) -> Option<foundations::Datetime> {
    let duration = self.now.duration_since(time::UNIX_EPOCH).ok()?;
    let secs = match offset {
      None => i64::try_from(duration.as_secs()).ok()?,
      #[expect(clippy::cast_possible_truncation, reason = "bounded duration")]
      Some(off) => i64::try_from(duration.as_secs())
        .ok()?
        .checked_add(off.seconds() as i64)?,
    };
    days_to_date(secs / 86400)
  }
}

/// Formats Typst compilation diagnostics into a human-readable string.
fn format_diagnostics(
  world: &ResumakeWorld,
  diags: &[diag::SourceDiagnostic],
) -> String {
  let mut msg = String::new();
  for d in diags {
    let severity = match d.severity {
      diag::Severity::Error => "error",
      diag::Severity::Warning => "warning",
    };
    let loc = if let Some(id) = d.span.id() {
      let path = id.vpath().get_with_slash();
      let line_col = world.source(id).ok().and_then(|source| {
        let range = world.range(d.span)?;
        source.lines().byte_to_line_column(range.start)
      });
      match line_col {
        Some((line, col)) => format!("{path}:{}:{}: ", line + 1, col + 1),
        None => format!("{path}: "),
      }
    } else {
      String::new()
    };
    let _ = writeln!(msg, "{loc}{severity}: {}", d.message);
    for hint in &d.hints {
      let _ = writeln!(msg, "  = hint: {}", hint.v);
    }
  }
  msg
}

/// Compiles a résumé to an in-memory [`typst_layout::PagedDocument`] and
/// evaluates telemetry.
///
/// # Errors
/// Returns [`Error`] if template resolution, Typst compilation, or telemetry
/// evaluation fails.
pub fn compile(
  content_path: &path::Path,
  template_name: Option<&str>,
  project_root: &path::Path,
  font_override: Option<&path::Path>,
) -> Result<(typst_layout::PagedDocument, telemetry::TelemetryReport), Error> {
  let tname = template_name.unwrap_or(templates::DEFAULT_TEMPLATE);
  let template_source = templates::resolve(tname, project_root)?;

  let world = ResumakeWorld::new(
    project_root.to_path_buf(),
    template_source,
    content_path.to_path_buf(),
    font_override,
  );

  let result = typst::compile(&world);
  let doc = result.output.map_err(|diags| {
    let stderr = format_diagnostics(&world, &diags);
    Error::Compilation { stderr }
  })?;

  let mut report = telemetry::evaluate(&doc)?;
  let requested =
    schema::requested_fonts_from_file(content_path).unwrap_or_default();
  report.missing_fonts = world.missing_fonts(&requested);
  Ok((doc, report))
}

/// Exports a compiled [`typst_layout::PagedDocument`] as PDF bytes written to
/// `output_path`.
///
/// # Errors
/// Returns [`Error`] if PDF encoding or writing fails.
pub fn export_pdf(
  doc: &typst_layout::PagedDocument,
  output_path: &path::Path,
) -> Result<(), Error> {
  let pdf_bytes = typst_pdf::pdf(doc, &typst_pdf::PdfOptions::default())
    .map_err(|diags| {
      let stderr = diags
        .iter()
        .map(|d| format!("error: {}", d.message))
        .collect::<Vec<_>>()
        .join("\n");
      Error::Compilation { stderr }
    })?;

  if let Some(parent) = output_path.parent() {
    fs::create_dir_all(parent)?;
  }
  fs::write(output_path, pdf_bytes)?;

  Ok(())
}
