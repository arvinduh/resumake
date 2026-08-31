//! In-process Typst engine orchestration, embedded templates, and layout telemetry introspection.

use crate::schema::validate_schema_auto;
use crate::telemetry::{evaluate_telemetry, TelemetryReport};
use include_dir::{include_dir, Dir};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use typst::diag::{FileError, FileResult, Severity, SourceDiagnostic};
use typst::foundations::{Bytes, Datetime, Dict, Label, Selector, Str, Value};
use typst::layout::PagedDocument;
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::{LazyHash, PicoStr};
use typst::{Document, Library, World};
use typst_kit::fonts::FontSlot;

/// A single Typst source file belonging to an embedded template, keyed by
/// its path relative to the template's root directory (e.g.
/// `"blocks/experience.typ"`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateFile {
  /// Path relative to the template root, using forward slashes.
  pub rel_path: String,
  /// Embedded file contents.
  pub contents: &'static str,
}

/// A complete named résumé template bundled into the binary. Every
/// template is a self-contained Typst module tree with its own
/// `main.typ` entry point, so multiple visual layouts (single-column,
/// sidebar, etc.) can coexist and be selected at the CLI without
/// touching the data model in `models.rs`.
///
/// # Template contract
///
/// To stay compatible with `rsmk build` (and `rsmk build --check`)
/// telemetry, a template's `main.typ` must still emit the `<pageinfo>`
/// metadata tag (see `templates/classic/main.typ`) and route bullet-like content
/// through the `guard()` primitive to emit `<bulletinfo>` tags. Layout is
/// otherwise entirely up to the template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedTemplate {
  /// Registry name selected via `--template <name>` (e.g. `"classic"`).
  pub name: String,
  /// Entry point file, always extracted as `main.typ`.
  pub entry: &'static str,
  /// All other files in the template tree (tokens, primitives, blocks/*).
  pub files: Vec<TemplateFile>,
}

/// Embedded directory containing all built-in résumé templates.
/// To add a new built-in template, simply drop a directory under
/// `src/embedded/templates/<name>/` containing `main.typ` and any supporting files;
/// it will be automatically discovered and registered at compile time.
static TEMPLATES_DIR: Dir<'_> =
  include_dir!("$CARGO_MANIFEST_DIR/src/embedded/templates");

fn relative_posix_path(path: &Path, root: &Path) -> String {
  if let Ok(rel) = path.strip_prefix(root) {
    let s = rel.to_string_lossy().replace('\\', "/");
    let trimmed = s.trim_start_matches('/');
    if !trimmed.is_empty() {
      return trimmed.to_string();
    }
  }
  let path_str = path.to_string_lossy().replace('\\', "/");
  let root_str = root.to_string_lossy().replace('\\', "/");
  let trimmed_root = root_str.trim_matches('/');
  if !trimmed_root.is_empty() {
    if let Some(rest) = path_str.strip_prefix(&format!("{trimmed_root}/")) {
      return rest.to_string();
    }
  }
  path_str.trim_start_matches('/').to_string()
}

fn collect_template_files(
  template_root: &Dir<'static>,
  current_dir: &Dir<'static>,
  files: &mut Vec<TemplateFile>,
  entry: &mut Option<&'static str>,
) {
  for file in current_dir.files() {
    let rel_path = relative_posix_path(file.path(), template_root.path());
    let contents = file.contents_utf8().unwrap_or("");
    if rel_path == "main.typ" {
      *entry = Some(contents);
    } else {
      files.push(TemplateFile { rel_path, contents });
    }
  }

  for sub_dir in current_dir.dirs() {
    collect_template_files(template_root, sub_dir, files, entry);
  }
}

/// Discovers all embedded templates compiled into the binary from `src/embedded/templates/`.
pub fn embedded_templates() -> Vec<EmbeddedTemplate> {
  let mut templates = Vec::new();
  for dir in TEMPLATES_DIR.dirs() {
    let name = dir
      .path()
      .file_name()
      .and_then(|s| s.to_str())
      .unwrap_or_else(|| dir.path().to_str().unwrap_or_default());
    if name.is_empty() || name.starts_with('.') {
      continue;
    }

    let mut files = Vec::new();
    let mut entry = None;
    collect_template_files(dir, dir, &mut files, &mut entry);

    if let Some(entry) = entry {
      files.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
      templates.push(EmbeddedTemplate {
        name: name.to_string(),
        entry,
        files,
      });
    }
  }
  templates.sort_by(|a, b| a.name.cmp(&b.name));
  templates
}

/// The default template name used when `--template` is not specified.
pub const DEFAULT_TEMPLATE: &str = "classic";

/// Looks up a bundled template by registry name.
pub fn find_embedded_template(name: &str) -> Option<EmbeddedTemplate> {
  embedded_templates().into_iter().find(|t| t.name == name)
}

/// Lists the names of all bundled templates, for error messages.
pub fn known_template_names() -> Vec<String> {
  embedded_templates().into_iter().map(|t| t.name).collect()
}

/// Summary information about a template (built-in or discovered on disk).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateInfo {
  /// Name of the template.
  pub name: String,
  /// Whether the template is embedded in the binary.
  pub is_builtin: bool,
  /// Whether the template is the default built-in template.
  pub is_default: bool,
  /// Local filesystem path for custom templates, if discovered on disk.
  pub path: Option<PathBuf>,
}

impl fmt::Display for TemplateInfo {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if self.is_builtin {
      if self.is_default {
        write!(f, "{} (built-in, default)", self.name)
      } else {
        write!(f, "{} (built-in)", self.name)
      }
    } else {
      write!(f, "{} (custom)", self.name)
    }
  }
}

/// Lists all built-in templates and any custom templates discovered in `./templates/`.
pub fn list_templates() -> Vec<TemplateInfo> {
  let root = find_project_root();
  list_templates_in(&root.join("templates"))
}

/// Lists all built-in templates and any custom templates discovered in the given directory.
pub fn list_templates_in(templates_dir: &Path) -> Vec<TemplateInfo> {
  let mut results = Vec::new();

  for template in embedded_templates() {
    let is_default = template.name == DEFAULT_TEMPLATE;
    results.push(TemplateInfo {
      name: template.name,
      is_builtin: true,
      is_default,
      path: None,
    });
  }

  if templates_dir.is_dir() {
    if let Ok(entries) = fs::read_dir(templates_dir) {
      let mut custom_templates = Vec::new();
      for entry in entries.flatten() {
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();
        if file_name.starts_with('.') {
          continue;
        }

        if path.is_dir() {
          custom_templates.push(TemplateInfo {
            name: file_name,
            is_builtin: false,
            is_default: false,
            path: Some(path),
          });
        } else if path.is_file()
          && path.extension().is_some_and(|ext| ext == "typ")
        {
          let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or(file_name);
          custom_templates.push(TemplateInfo {
            name: stem,
            is_builtin: false,
            is_default: false,
            path: Some(path),
          });
        }
      }
      custom_templates.sort_by(|a, b| a.name.cmp(&b.name));
      results.extend(custom_templates);
    }
  }

  results
}

/// Extracts all embedded Typst component files for the template into `target_dir`.
///
/// # Errors
///
/// Returns [`EngineError::TemplateNotFound`] if `name` is not in the built-in registry.
/// Returns [`EngineError::DestinationAlreadyExists`] if `target_dir` exists and `force` is false.
/// Returns [`EngineError::Io`] if directory creation or file writing fails.
pub fn eject_template(
  name: &str,
  target_dir: &Path,
  force: bool,
) -> Result<Vec<String>, EngineError> {
  let template = find_embedded_template(name).ok_or_else(|| {
    EngineError::TemplateNotFound {
      name: name.to_string(),
      known: known_template_names(),
    }
  })?;

  if target_dir.exists() && !force {
    return Err(EngineError::DestinationAlreadyExists {
      path: target_dir.to_path_buf(),
    });
  }

  fs::create_dir_all(target_dir)?;

  let mut ejected_files = Vec::new();

  let entry_dest = target_dir.join("main.typ");
  fs::write(&entry_dest, template.entry)?;
  ejected_files.push("main.typ".to_string());

  for file in template.files {
    let dest = target_dir.join(&file.rel_path);
    if let Some(parent) = dest.parent() {
      fs::create_dir_all(parent)?;
    }
    fs::write(&dest, file.contents)?;
    ejected_files.push(file.rel_path);
  }

  Ok(ejected_files)
}

/// Converts Unix days since epoch to a Gregorian [`Datetime`].
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
  let y = if m <= 2 { y + 1 } else { y };
  Datetime::from_ymd(y as i32, m as u8, d as u8)
}

/// Errors originating from the Typst execution engine.
#[derive(thiserror::Error, Debug)]
pub enum EngineError {
  /// A user-specified font directory was not found.
  #[error("No valid font directory found. Searched locations:\n{}", .searched.iter().map(|p| format!("  - {}", p.display())).collect::<Vec<_>>().join("\n"))]
  FontDirNotFound {
    /// Searched locations.
    searched: Vec<PathBuf>,
  },
  /// The requested `--template <name>` is not registered.
  #[error("Unknown template '{name}'. Available templates: {}", .known.join(", "))]
  TemplateNotFound {
    /// The requested template name.
    name: String,
    /// Names of templates actually bundled into the binary.
    known: Vec<String>,
  },
  /// In-process Typst compilation failed with diagnostics.
  #[error("Typst compilation failed:\n{stderr}")]
  CompilationFailed {
    /// Captured diagnostic messages.
    stderr: String,
  },
  /// Metadata query failed.
  #[error("Typst query failed:\n{stderr}")]
  QueryFailed {
    /// Diagnostic error message.
    stderr: String,
  },
  /// Destination directory already exists and `--force` was not specified.
  #[error(
    "Destination directory '{}' already exists. Use --force to overwrite.",
    display_path(path)
  )]
  DestinationAlreadyExists {
    /// Destination directory path.
    path: PathBuf,
  },
  /// Content file was not found.
  #[error("Content file not found: '{}'", display_path(path))]
  ContentNotFound {
    /// Path to content file.
    path: PathBuf,
  },
  /// Document failed strict single-page layout geometry constraints.
  #[error("Dry-run check failed strict single-page layout constraints.")]
  LayoutConstraintViolation,
  /// Schema validation error.
  #[error(transparent)]
  Schema(#[from] crate::schema::SchemaError),
  /// Telemetry error.
  #[error(transparent)]
  Telemetry(#[from] crate::telemetry::TelemetryError),
  /// Underlying I/O error.
  #[error("I/O error: {0}")]
  Io(#[from] std::io::Error),
}

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
  ///
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

    let content_vpath = normalize_posix_path(&root_path, &content_path)
      .unwrap_or_else(|_| "/content.yaml".to_string());
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

/// Queries metadata value(s) matching `selector` from a [`PagedDocument`] and serializes to JSON.
///
/// # Errors
///
/// Returns [`EngineError::QueryFailed`] if serialization fails.
pub fn query_doc_metadata(
  doc: &PagedDocument,
  selector: &str,
) -> Result<String, EngineError> {
  let label_str = selector
    .trim()
    .trim_start_matches('<')
    .trim_end_matches('>');
  let label = Label::new(PicoStr::intern(label_str));
  let sel = Selector::Label(label);
  let elems = doc.introspector().query(&sel);

  let mut values = Vec::new();
  for elem in elems {
    if let Some(metadata) =
      elem.to_packed::<typst::introspection::MetadataElem>()
    {
      values.push(metadata.value.clone());
    } else if let Ok(val) = elem.get_by_name("value") {
      values.push(val);
    }
  }

  serde_json::to_string(&values).map_err(|e| EngineError::QueryFailed {
    stderr: e.to_string(),
  })
}

/// Discovers the font directory if present (user override, `./fonts`, or
/// `assets/fonts`).
///
/// # Errors
///
/// Returns an [`EngineError::FontDirNotFound`] if a custom font path was
/// provided but does not exist.
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

/// Strips the Windows `\\?\` verbatim prefix (and the `UNC\` marker) from a
/// path so it reads naturally in user-facing messages. A no-op on paths
/// without the prefix and on non-Windows platforms.
fn display_path(path: &Path) -> String {
  let s = path.to_string_lossy();
  #[cfg(windows)]
  {
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
      return format!(r"\\{rest}");
    }
    if let Some(rest) = s.strip_prefix(r"\\?\") {
      return rest.to_string();
    }
  }
  s.into_owned()
}

/// Normalizes a content file path into a POSIX virtual path.
pub fn normalize_posix_path(
  root: &Path,
  content_path: &Path,
) -> Result<String, EngineError> {
  if let Ok(rel) = content_path.strip_prefix(root) {
    let s = rel.to_string_lossy().replace('\\', "/");
    let trimmed = s.trim_start_matches('/');
    return Ok(format!("/{trimmed}"));
  }

  if let (Ok(canon_root), Ok(canon_content)) =
    (root.canonicalize(), content_path.canonicalize())
  {
    if let Ok(rel) = canon_content.strip_prefix(&canon_root) {
      let s = rel.to_string_lossy().replace('\\', "/");
      let trimmed = s.trim_start_matches('/');
      return Ok(format!("/{trimmed}"));
    }
  }

  if content_path.is_relative() {
    let raw = content_path.to_string_lossy().replace('\\', "/");
    let trimmed = raw
      .trim_start_matches("./")
      .trim_start_matches('/')
      .trim_start_matches('\\');
    return Ok(format!("/{trimmed}"));
  }

  if let Some(file_name) = content_path.file_name() {
    return Ok(format!("/{}", file_name.to_string_lossy()));
  }

  Ok("/content.yaml".to_string())
}

/// Finds project root by checking for markers (`resume.yaml`,
/// `content.yaml`, `Cargo.toml`, `.git`) or defaulting to current
/// directory.
pub fn find_project_root() -> PathBuf {
  let mut curr = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
  loop {
    if curr.join("resume.yaml").exists()
      || curr.join("content.yaml").exists()
      || curr.join("Cargo.toml").exists()
      || curr.join(".git").exists()
    {
      return curr;
    }
    if !curr.pop() {
      break;
    }
  }
  std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Engine facade coordinating in-process Typst compilation, embedded modular templates,
/// and metadata introspection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypstEngine {
  /// Optional font directory passed to Typst via `--font-path`.
  pub font_path: Option<PathBuf>,
  /// Project root path.
  pub root_path: PathBuf,
}

impl TypstEngine {
  /// Discovers the project root and optional font directory.
  ///
  /// # Errors
  ///
  /// Returns an [`EngineError`] if the font directory is invalid.
  pub fn new(font_path_override: Option<&Path>) -> Result<Self, EngineError> {
    let root_path = find_project_root();
    let font_path = discover_font_dir(&root_path, font_path_override)?;
    Ok(Self {
      font_path,
      root_path,
    })
  }

  /// Resolves the template path. If a custom template file is provided
  /// and exists, returns it directly. Otherwise, looks up `template_name` in the
  /// built-in template registry or `./templates/` on disk.
  ///
  /// # Errors
  ///
  /// Returns an [`EngineError::TemplateNotFound`] if `template_name` is not a registered
  /// built-in or custom template.
  pub fn resolve_template(
    &self,
    template_name: &str,
    custom_template: Option<&Path>,
  ) -> Result<PathBuf, EngineError> {
    if let Some(tpl) = custom_template.filter(|p| p.exists()) {
      return Ok(tpl.to_path_buf());
    }

    let direct_path = Path::new(template_name);
    if direct_path.exists() && direct_path.is_file() {
      return Ok(direct_path.to_path_buf());
    }
    let rooted_direct = self.root_path.join(template_name);
    if rooted_direct.exists() && rooted_direct.is_file() {
      return Ok(rooted_direct);
    }

    if find_embedded_template(template_name).is_some() {
      return Ok(PathBuf::from(format!("{template_name}/main.typ")));
    }

    let custom_main = self
      .root_path
      .join("templates")
      .join(template_name)
      .join("main.typ");
    if custom_main.exists() && custom_main.is_file() {
      return Ok(custom_main);
    }

    Err(EngineError::TemplateNotFound {
      name: template_name.to_string(),
      known: known_template_names(),
    })
  }

  /// Instantiates a new [`ResumakeWorld`] for the given template and content.
  ///
  /// # Errors
  ///
  /// Returns an [`EngineError`] if world creation fails.
  pub fn create_world(
    &self,
    template: &Path,
    content: &Path,
  ) -> Result<ResumakeWorld, EngineError> {
    ResumakeWorld::new(
      self.root_path.clone(),
      template.to_path_buf(),
      content.to_path_buf(),
      self.font_path.clone(),
    )
  }

  /// Compiles a Typst template and content file into a layouted [`PagedDocument`].
  ///
  /// # Errors
  ///
  /// Returns an [`EngineError::CompilationFailed`] if Typst compilation produces errors.
  pub fn compile_paged(
    &self,
    template: &Path,
    content: &Path,
  ) -> Result<PagedDocument, EngineError> {
    let world = self.create_world(template, content)?;
    let warned = typst::compile::<PagedDocument>(&world);
    match warned.output {
      Ok(doc) => Ok(doc),
      Err(errors) => {
        let stderr = format_diagnostics(&world, &errors);
        Err(EngineError::CompilationFailed { stderr })
      }
    }
  }

  /// Compiles a Typst template and content file into an output PDF document.
  ///
  /// # Errors
  ///
  /// Returns an [`EngineError`] if Typst compilation or PDF export fails.
  pub fn compile(
    &self,
    template: &Path,
    content: &Path,
    output: &Path,
  ) -> Result<(), EngineError> {
    let world = self.create_world(template, content)?;
    let warned = typst::compile::<PagedDocument>(&world);
    let doc = match warned.output {
      Ok(doc) => doc,
      Err(errors) => {
        let stderr = format_diagnostics(&world, &errors);
        return Err(EngineError::CompilationFailed { stderr });
      }
    };

    let pdf_bytes = typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default())
      .map_err(|errors| {
        let stderr = format_diagnostics(&world, &errors);
        EngineError::CompilationFailed { stderr }
      })?;

    if let Some(parent) = output.parent() {
      if !parent.as_os_str().is_empty() {
        fs::create_dir_all(parent)?;
      }
    }

    fs::write(output, pdf_bytes)?;
    Ok(())
  }

  /// Queries Typst metadata elements (e.g. `<bulletinfo>`, `<pageinfo>`)
  /// from the document.
  ///
  /// # Errors
  ///
  /// Returns an [`EngineError`] if compilation or querying fails.
  pub fn query_metadata(
    &self,
    template: &Path,
    content: &Path,
    selector: &str,
  ) -> Result<String, EngineError> {
    let doc = self.compile_paged(template, content)?;
    query_doc_metadata(&doc, selector)
  }
}

/// Validates content schema and runs layout telemetry evaluation without generating a PDF.
///
/// # Errors
///
/// Returns an [`EngineError`] if the content file does not exist, fails schema validation,
/// fails Typst compilation/querying, or violates single-page geometry constraints.
pub fn verify_content(
  content: &Path,
  template_name: &str,
  source: Option<&Path>,
  schema: Option<&Path>,
  font_path: Option<&Path>,
) -> Result<TelemetryReport, EngineError> {
  if !content.exists() {
    return Err(EngineError::ContentNotFound {
      path: content.to_path_buf(),
    });
  }

  // 1. Schema check
  validate_schema_auto(content, schema)?;

  // 2. In-process layout telemetry check
  let engine = TypstEngine::new(font_path)?;
  let resolved_template = engine.resolve_template(template_name, source)?;
  let doc = engine.compile_paged(&resolved_template, content)?;
  let page_json = query_doc_metadata(&doc, "<pageinfo>")?;
  let bullets_json = query_doc_metadata(&doc, "<bulletinfo>")?;
  let report = evaluate_telemetry(&page_json, &bullets_json)?;

  if !report.is_pass() {
    return Err(EngineError::LayoutConstraintViolation);
  }

  Ok(report)
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::TempDir;

  #[test]
  fn test_resolve_template_builtin_classic() {
    let temp = TempDir::new().unwrap();
    let engine = TypstEngine {
      font_path: None,
      root_path: temp.path().to_path_buf(),
    };

    let resolved = engine.resolve_template(DEFAULT_TEMPLATE, None).unwrap();
    assert_eq!(resolved, PathBuf::from("classic/main.typ"));
  }

  #[test]
  fn test_resolve_template_rejects_unknown_name() {
    let temp = TempDir::new().unwrap();
    let engine = TypstEngine {
      font_path: None,
      root_path: temp.path().to_path_buf(),
    };

    let err = engine.resolve_template("does-not-exist", None).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("does-not-exist"));
    assert!(msg.contains("classic"));
  }

  #[test]
  fn test_embedded_templates_discovery() {
    let templates = embedded_templates();
    assert!(!templates.is_empty());
    let classic = templates
      .iter()
      .find(|t| t.name == "classic")
      .expect("classic template must be found");
    assert_eq!(classic.name, "classic");
    assert!(!classic.entry.is_empty());
    assert!(classic.files.iter().any(|f| f.rel_path == "tokens.typ"));
    assert!(classic.files.iter().any(|f| f.rel_path == "primitives.typ"));
    assert!(classic
      .files
      .iter()
      .any(|f| f.rel_path == "blocks/experience.typ"));
  }

  #[test]
  fn test_all_block_files_are_registered_in_main_typ() {
    let blocks = [
      "education",
      "experience",
      "projects",
      "skills",
      "publications",
      "split_line",
      "references",
      "lines",
    ];

    let classic =
      find_embedded_template("classic").expect("classic template must exist");
    for block in blocks {
      assert!(
        classic.entry.contains(&format!("blocks/{block}.typ")),
        "Block '{block}' is missing an #import in main.typ!"
      );
    }
  }

  #[test]
  fn test_normalize_posix_path() {
    let root = Path::new("/workspace/project");
    assert_eq!(
      normalize_posix_path(root, Path::new("content.yaml")).unwrap(),
      "/content.yaml"
    );
    assert_eq!(
      normalize_posix_path(root, Path::new("./content.yaml")).unwrap(),
      "/content.yaml"
    );
  }

  #[test]
  fn test_normalize_posix_path_absolute_under_root() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    let nested = root.join("resume").join("content.yaml");
    fs::create_dir_all(nested.parent().unwrap()).unwrap();
    fs::write(&nested, "name: Test\n").unwrap();

    assert_eq!(
      normalize_posix_path(root, &nested).unwrap(),
      "/resume/content.yaml"
    );
  }

  #[test]
  fn test_list_templates_builtins_and_custom() {
    let temp = TempDir::new().unwrap();
    let templates_dir = temp.path().join("templates");

    // Initially without templates dir
    let list = list_templates_in(&templates_dir);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "classic");
    assert!(list[0].is_builtin);
    assert!(list[0].is_default);
    assert_eq!(list[0].to_string(), "classic (built-in, default)");

    // Add custom directories and files
    fs::create_dir_all(templates_dir.join("modern")).unwrap();
    fs::create_dir_all(templates_dir.join("minimal")).unwrap();
    fs::write(templates_dir.join("single.typ"), "// single file\n").unwrap();
    fs::write(templates_dir.join("ignore.txt"), "text\n").unwrap();
    fs::create_dir_all(templates_dir.join(".hidden")).unwrap();

    let list2 = list_templates_in(&templates_dir);
    assert_eq!(list2.len(), 4);
    assert_eq!(list2[0].name, "classic");
    assert!(list2[0].is_builtin);

    assert_eq!(list2[1].name, "minimal");
    assert!(!list2[1].is_builtin);
    assert_eq!(list2[1].to_string(), "minimal (custom)");

    assert_eq!(list2[2].name, "modern");
    assert!(!list2[2].is_builtin);
    assert_eq!(list2[2].to_string(), "modern (custom)");

    assert_eq!(list2[3].name, "single");
    assert!(!list2[3].is_builtin);
    assert_eq!(list2[3].to_string(), "single (custom)");
  }

  #[test]
  fn test_eject_template_success_and_collision_rejection() {
    let temp = TempDir::new().unwrap();
    let target = temp.path().join("templates").join("classic");

    // 1. First eject should succeed
    let files = eject_template("classic", &target, false).unwrap();
    assert!(files.contains(&"main.typ".to_string()));
    assert!(files.contains(&"tokens.typ".to_string()));
    assert!(files.contains(&"primitives.typ".to_string()));
    assert!(files.contains(&"blocks/experience.typ".to_string()));

    assert!(target.join("main.typ").exists());
    assert!(target.join("tokens.typ").exists());
    assert!(target.join("primitives.typ").exists());
    assert!(target.join("blocks").join("experience.typ").exists());

    // 2. Second eject without force must fail with DestinationAlreadyExists
    let err = eject_template("classic", &target, false).unwrap_err();
    match err {
      EngineError::DestinationAlreadyExists { path } => {
        assert_eq!(path, target);
      }
      other => panic!("expected DestinationAlreadyExists, got {other:?}"),
    }

    // 3. Eject with force should succeed
    let files_forced = eject_template("classic", &target, true).unwrap();
    assert_eq!(files, files_forced);
  }

  #[test]
  fn test_eject_template_rejects_unknown_name() {
    let temp = TempDir::new().unwrap();
    let target = temp.path().join("templates").join("unknown");
    let err = eject_template("unknown", &target, false).unwrap_err();
    match err {
      EngineError::TemplateNotFound { name, .. } => {
        assert_eq!(name, "unknown");
      }
      other => panic!("expected TemplateNotFound, got {other:?}"),
    }
  }

  #[test]
  fn test_resolve_template_direct_path_and_custom() {
    let temp = TempDir::new().unwrap();
    let custom_main = temp
      .path()
      .join("templates")
      .join("custom")
      .join("main.typ");
    fs::create_dir_all(custom_main.parent().unwrap()).unwrap();
    fs::write(&custom_main, "// custom template\n").unwrap();

    let engine = TypstEngine {
      font_path: None,
      root_path: temp.path().to_path_buf(),
    };

    // Resolving via direct path
    let resolved_direct = engine
      .resolve_template(&custom_main.to_string_lossy(), None)
      .unwrap();
    assert_eq!(resolved_direct, custom_main);

    // Resolving via custom template name under root/templates/custom
    let resolved_custom = engine.resolve_template("custom", None).unwrap();
    assert_eq!(resolved_custom, custom_main);
  }

  #[test]
  fn test_in_process_compilation_and_telemetry() {
    let temp = TempDir::new().unwrap();
    let content_file = temp.path().join("content.yaml");
    let output_pdf = temp.path().join("output.pdf");

    let content_yaml = r#"
meta:
  name: "Jane Doe"
  version: "1.0.0"
  title: "Systems Engineer"
  contact:
    - name: "jane@example.com"
sections:
  - type: "experience"
    title: "Experience"
    items:
      - role: "Staff Engineer"
        org: "Acme Corp"
        date: "2020 - Present"
        bullets:
          - "Engineered high-throughput streaming systems in Rust."
"#;
    fs::write(&content_file, content_yaml).unwrap();

    let engine = TypstEngine {
      font_path: None,
      root_path: temp.path().to_path_buf(),
    };

    let template = engine.resolve_template("classic", None).unwrap();
    engine
      .compile(&template, &content_file, &output_pdf)
      .unwrap();
    assert!(output_pdf.exists());
    assert!(fs::metadata(&output_pdf).unwrap().len() > 0);

    let page_json = engine
      .query_metadata(&template, &content_file, "<pageinfo>")
      .unwrap();
    assert!(page_json.contains("pages"));

    let bullets_json = engine
      .query_metadata(&template, &content_file, "<bulletinfo>")
      .unwrap();
    assert!(
      bullets_json.contains("Engineered high-throughput streaming systems")
    );

    let report = evaluate_telemetry(&page_json, &bullets_json).unwrap();
    assert_eq!(report.page_count, 1);
    assert!(report.is_pass());
  }

  #[test]
  fn test_compilation_error_formatting() {
    let temp = TempDir::new().unwrap();
    let content_file = temp.path().join("content.yaml");
    let broken_template = temp.path().join("broken.typ");
    let output_pdf = temp.path().join("output.pdf");

    fs::write(&content_file, "meta:\n  name: Test\n").unwrap();
    fs::write(&broken_template, "#undefined_function_call()\n").unwrap();

    let engine = TypstEngine {
      font_path: None,
      root_path: temp.path().to_path_buf(),
    };

    let err = engine
      .compile(&broken_template, &content_file, &output_pdf)
      .unwrap_err();
    let err_str = err.to_string();
    assert!(
      err_str.contains("undefined_function_call")
        || err_str.contains("unknown variable")
    );
  }
}
