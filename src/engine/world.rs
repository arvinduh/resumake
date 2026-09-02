//! Typst World implementation, virtual file system, font loader, and diagnostic formatting.

use crate::engine::templates::{DEFAULT_TEMPLATE, TEMPLATES_DIR};
use crate::engine::EngineError;
use crate::utils::fs::normalize_posix_path;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use typst::diag::{FileError, FileResult, Severity, SourceDiagnostic};
use typst::foundations::{Bytes, Datetime, Dict, Str, Value};
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, World};
use typst_kit::fonts::FontSlot;

/// In-process [`World`] implementation resolving embedded templates from memory,
/// disk files from the project root, and system/custom fonts.
pub struct ResumakeWorld {
  library: LazyHash<Library>,
  book: LazyHash<FontBook>,
  fonts: Vec<FontSlot>,
  main_id: FileId,
  content_id: FileId,
  root_path: PathBuf,
  template_path: PathBuf,
  content_path: PathBuf,
  content_vpath: String,
  template_name_hint: String,
  sources: Mutex<HashMap<FileId, FileResult<Source>>>,
  files: Mutex<HashMap<FileId, FileResult<Bytes>>>,
  now: std::time::SystemTime,
}

impl ResumakeWorld {
  /// Constructs a new [`ResumakeWorld`].
  ///
  /// # Errors
  /// Returns [`EngineError`] if font initialization fails.
  pub fn new(
    root_path: PathBuf,
    template_path: PathBuf,
    content_path: PathBuf,
    font_path: Option<PathBuf>,
  ) -> Result<Self, EngineError> {
    let mut searcher = typst_kit::fonts::Fonts::searcher();
    searcher.include_system_fonts(true);
    searcher.include_embedded_fonts(true);

    let mut font_dirs = Vec::new();
    if let Some(ref fp) = font_path {
      font_dirs.push(fp.clone());
    }
    let candidate_fonts = root_path.join("fonts");
    if candidate_fonts.is_dir() {
      font_dirs.push(candidate_fonts);
    }
    let candidate_assets = root_path.join("assets").join("fonts");
    if candidate_assets.is_dir() {
      font_dirs.push(candidate_assets);
    }

    let fonts = searcher.search_with(font_dirs);
    let book = LazyHash::new(fonts.book);
    let font_slots = fonts.fonts;

    let content_vpath = normalize_posix_path(&root_path, &content_path);
    let content_id = FileId::new(None, VirtualPath::new(&content_vpath));

    let mut inputs = Dict::new();
    inputs.insert(
      Str::from("content"),
      Value::Str(Str::from(content_vpath.as_str())),
    );
    let library = LazyHash::new(Library::builder().with_inputs(inputs).build());

    let template_str = template_path.to_string_lossy().replace('\\', "/");
    let template_name_hint = template_str
      .split('/')
      .next()
      .unwrap_or(DEFAULT_TEMPLATE)
      .to_string();

    let main_vpath = if template_str.starts_with('/') {
      template_str
    } else if let Ok(rel) = template_path.strip_prefix(&root_path) {
      format!("/{}", rel.to_string_lossy().replace('\\', "/"))
    } else {
      format!("/{template_str}")
    };

    let main_id = FileId::new(None, VirtualPath::new(&main_vpath));

    Ok(Self {
      library,
      book,
      fonts: font_slots,
      main_id,
      content_id,
      root_path,
      template_path,
      content_path,
      content_vpath,
      template_name_hint,
      sources: Mutex::new(HashMap::new()),
      files: Mutex::new(HashMap::new()),
      now: std::time::SystemTime::now(),
    })
  }

  fn read_bytes_uncached(&self, id: FileId) -> FileResult<Bytes> {
    let raw_vpath = id.vpath().as_rootless_path().to_string_lossy();
    let vpath = raw_vpath.replace('\\', "/");
    let trimmed_vpath = vpath.trim_start_matches('/');

    // 1. Resolve from embedded templates in memory
    if let Some(file) = TEMPLATES_DIR.get_file(trimmed_vpath) {
      return Ok(Bytes::new(file.contents()));
    }
    if !self.template_name_hint.is_empty() {
      let scoped_embedded =
        format!("{}/{}", self.template_name_hint, trimmed_vpath);
      if let Some(file) = TEMPLATES_DIR.get_file(&scoped_embedded) {
        return Ok(Bytes::new(file.contents()));
      }
    }

    // 2. Resolve content file
    let rooted_vpath = format!("/{trimmed_vpath}");
    if (id == self.content_id
      || rooted_vpath == self.content_vpath
      || trimmed_vpath == "content.yaml"
      || trimmed_vpath.ends_with("/content.yaml")
      || trimmed_vpath == self.content_vpath.trim_start_matches('/'))
      && self.content_path.is_file()
    {
      return fs::read(&self.content_path)
        .map(Bytes::new)
        .map_err(|e| FileError::from_io(e, &self.content_path));
    }

    // 3. Resolve template entry or custom template parent files
    if id == self.main_id && self.template_path.is_file() {
      return fs::read(&self.template_path)
        .map(Bytes::new)
        .map_err(|e| FileError::from_io(e, &self.template_path));
    }
    if let Some(parent) = self.template_path.parent() {
      let candidate = parent.join(trimmed_vpath);
      if candidate.is_file() {
        return fs::read(&candidate)
          .map(Bytes::new)
          .map_err(|e| FileError::from_io(e, &candidate));
      }
    }

    // 4. Resolve from project root on disk
    let disk_path = self.root_path.join(trimmed_vpath);
    if disk_path.is_file() {
      return fs::read(&disk_path)
        .map(Bytes::new)
        .map_err(|e| FileError::from_io(e, &disk_path));
    }

    // 5. If content_path filename matches
    if let Some(content_name) = self.content_path.file_name() {
      if trimmed_vpath == content_name.to_string_lossy()
        && self.content_path.is_file()
      {
        return fs::read(&self.content_path)
          .map(Bytes::new)
          .map_err(|e| FileError::from_io(e, &self.content_path));
      }
    }

    Err(FileError::NotFound(
      id.vpath().as_rooted_path().to_path_buf(),
    ))
  }
}

impl World for ResumakeWorld {
  fn library(&self) -> &LazyHash<Library> {
    &self.library
  }

  fn book(&self) -> &LazyHash<FontBook> {
    &self.book
  }

  fn main(&self) -> FileId {
    self.main_id
  }

  fn source(&self, id: FileId) -> FileResult<Source> {
    let mut lock = self.sources.lock().unwrap();
    if let Some(res) = lock.get(&id) {
      return res.clone();
    }

    let bytes_res = self.read_bytes_uncached(id);
    let source_res = match bytes_res {
      Ok(bytes) => match std::str::from_utf8(&bytes) {
        Ok(text) => Ok(Source::new(id, text.to_string())),
        Err(_) => Err(FileError::InvalidUtf8),
      },
      Err(err) => Err(err),
    };

    lock.insert(id, source_res.clone());
    source_res
  }

  fn file(&self, id: FileId) -> FileResult<Bytes> {
    let mut lock = self.files.lock().unwrap();
    if let Some(res) = lock.get(&id) {
      return res.clone();
    }

    let res = self.read_bytes_uncached(id);
    lock.insert(id, res.clone());
    res
  }

  fn font(&self, index: usize) -> Option<Font> {
    self.fonts.get(index).and_then(|slot| slot.get())
  }

  fn today(&self, offset: Option<i64>) -> Option<Datetime> {
    let duration = self.now.duration_since(std::time::UNIX_EPOCH).ok()?;
    let secs = match offset {
      None => duration.as_secs() as i64,
      Some(hours) => (duration.as_secs() as i64) + (hours * 3600),
    };
    days_to_date(secs / 86400)
  }
}

fn days_to_date(days_since_epoch: i64) -> Option<Datetime> {
  let z = days_since_epoch + 719468;
  let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
  let doe = (z - era * 146097) as u32;
  let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
  let y = (yoe as i64) + era * 400;
  let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
  let mp = (5 * doy + 2) / 153;
  let d = doy - (153 * mp + 2) / 5 + 1;
  let m = if mp < 10 { mp + 3 } else { mp - 9 };
  let yr = if m <= 2 { y + 1 } else { y };

  Datetime::from_ymd(yr as i32, m as u8, d as u8)
}

/// Formats a list of [`SourceDiagnostic`] into a readable diagnostic string.
pub fn format_diagnostics(
  world: &ResumakeWorld,
  diags: &[SourceDiagnostic],
) -> String {
  let mut out = Vec::new();
  for diag in diags {
    let severity = match diag.severity {
      Severity::Error => "error",
      Severity::Warning => "warning",
    };
    let mut location = String::new();
    if let Some(id) = diag.span.id() {
      let path = id.vpath().as_rooted_path();
      if let Ok(source) = world.source(id) {
        if let Some(range) = source.range(diag.span) {
          let line =
            source.byte_to_line(range.start).map(|l| l + 1).unwrap_or(1);
          let col = source
            .byte_to_column(range.start)
            .map(|c| c + 1)
            .unwrap_or(1);
          location = format!("{}:{}:{}: ", path.display(), line, col);
        } else {
          location = format!("{}: ", path.display());
        }
      } else {
        location = format!("{}: ", path.display());
      }
    }
    let mut msg = format!("{location}{severity}: {}", diag.message);
    for hint in &diag.hints {
      msg.push_str(&format!("\n  = hint: {hint}"));
    }
    out.push(msg);
  }
  out.join("\n")
}

/// Discovers the font directory if present (user override, `./fonts`, or `assets/fonts`).
///
/// # Errors
/// Returns an [`EngineError::FontDirNotFound`] if a custom font path was provided but does not exist.
pub fn discover_font_dir(
  root: &Path,
  user_override: Option<&Path>,
) -> Result<Option<PathBuf>, EngineError> {
  if let Some(custom) = user_override {
    if custom.is_dir() {
      return Ok(Some(custom.to_path_buf()));
    }
    let joined = root.join(custom);
    if joined.is_dir() {
      return Ok(Some(joined));
    }
    return Err(EngineError::FontDirNotFound {
      searched: vec![joined, custom.to_path_buf()],
    });
  }

  let candidate = root.join("fonts");
  if candidate.is_dir() {
    return Ok(Some(candidate));
  }
  let candidate_assets = root.join("assets").join("fonts");
  if candidate_assets.is_dir() {
    return Ok(Some(candidate_assets));
  }

  Ok(None)
}
