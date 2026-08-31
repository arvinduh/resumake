//! Typst engine orchestration, embedded component cache, and subprocess
//! execution.

use crate::schema::validate_schema_auto;
use crate::telemetry::{evaluate_telemetry, TelemetryReport};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use which::which;

/// A single Typst source file belonging to an embedded template, keyed by
/// its path relative to the template's root directory (e.g.
/// `"blocks/experience.typ"`).
struct TemplateFile {
  /// Path relative to the template root, using forward slashes.
  rel_path: &'static str,
  /// Embedded file contents.
  contents: &'static str,
}

/// A complete named résumé template bundled into the binary. Every
/// template is a self-contained Typst module tree with its own
/// `main.typ` entry point, so multiple visual layouts (single-column,
/// sidebar, etc.) can coexist and be selected at the CLI without
/// touching the data model in `models.rs`.
///
/// # Template contract
///
/// To stay compatible with `rsmk check`/`build` telemetry, a
/// template's `main.typ` must still emit the `<pageinfo>` metadata tag
/// (see `templates/classic/main.typ`) and route bullet-like content
/// through the `guard()` primitive to emit `<bulletinfo>` tags. Layout is
/// otherwise entirely up to the template.
struct EmbeddedTemplate {
  /// Registry name selected via `--template <name>` (e.g. `"classic"`).
  name: &'static str,
  /// Entry point file, always extracted as `main.typ`.
  entry: &'static str,
  /// All other files in the template tree (tokens, primitives, blocks/*).
  files: &'static [TemplateFile],
}

const CLASSIC_TEMPLATE: EmbeddedTemplate = EmbeddedTemplate {
  name: "classic",
  entry: include_str!("embedded/templates/classic/main.typ"),
  files: &[
    TemplateFile {
      rel_path: "tokens.typ",
      contents: include_str!("embedded/templates/classic/tokens.typ"),
    },
    TemplateFile {
      rel_path: "primitives.typ",
      contents: include_str!("embedded/templates/classic/primitives.typ"),
    },
    TemplateFile {
      rel_path: "blocks/education.typ",
      contents: include_str!("embedded/templates/classic/blocks/education.typ"),
    },
    TemplateFile {
      rel_path: "blocks/experience.typ",
      contents: include_str!(
        "embedded/templates/classic/blocks/experience.typ"
      ),
    },
    TemplateFile {
      rel_path: "blocks/projects.typ",
      contents: include_str!("embedded/templates/classic/blocks/projects.typ"),
    },
    TemplateFile {
      rel_path: "blocks/skills.typ",
      contents: include_str!("embedded/templates/classic/blocks/skills.typ"),
    },
    TemplateFile {
      rel_path: "blocks/publications.typ",
      contents: include_str!(
        "embedded/templates/classic/blocks/publications.typ"
      ),
    },
    TemplateFile {
      rel_path: "blocks/split_line.typ",
      contents: include_str!(
        "embedded/templates/classic/blocks/split_line.typ"
      ),
    },
    TemplateFile {
      rel_path: "blocks/references.typ",
      contents: include_str!(
        "embedded/templates/classic/blocks/references.typ"
      ),
    },
    TemplateFile {
      rel_path: "blocks/lines.typ",
      contents: include_str!("embedded/templates/classic/blocks/lines.typ"),
    },
  ],
};

/// Registry of all templates bundled into the binary. Add a new entry
/// here (and a matching `embedded/templates/<name>/` tree) to register
/// another built-in layout.
const TEMPLATE_REGISTRY: &[&EmbeddedTemplate] = &[&CLASSIC_TEMPLATE];

/// The default template name used when `--template` is not specified.
pub const DEFAULT_TEMPLATE: &str = "classic";

/// Looks up a bundled template by registry name.
fn find_embedded_template(name: &str) -> Option<&'static EmbeddedTemplate> {
  TEMPLATE_REGISTRY.iter().find(|t| t.name == name).copied()
}

/// Lists the names of all bundled templates, for error messages.
fn known_template_names() -> Vec<&'static str> {
  TEMPLATE_REGISTRY.iter().map(|t| t.name).collect()
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

  for template in TEMPLATE_REGISTRY {
    results.push(TemplateInfo {
      name: template.name.to_string(),
      is_builtin: true,
      is_default: template.name == DEFAULT_TEMPLATE,
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
    let dest = target_dir.join(file.rel_path);
    if let Some(parent) = dest.parent() {
      fs::create_dir_all(parent)?;
    }
    fs::write(&dest, file.contents)?;
    ejected_files.push(file.rel_path.to_string());
  }

  Ok(ejected_files)
}

/// Errors originating from the Typst execution engine.
#[derive(Debug)]
pub enum EngineError {
  /// The `typst` binary could not be located on `PATH`.
  TypstNotFound {
    /// Platform-specific installation instructions shown to the user.
    instructions: String,
  },
  /// A user-specified font directory was not found.
  FontDirNotFound {
    /// Searched locations.
    searched: Vec<PathBuf>,
  },
  /// The requested `--template <name>` is not registered.
  TemplateNotFound {
    /// The requested template name.
    name: String,
    /// Names of templates actually bundled into the binary.
    known: Vec<&'static str>,
  },
  /// `typst compile` exited with a non-zero status.
  CompilationFailed {
    /// Captured stderr or stdout from the subprocess.
    stderr: String,
  },
  /// `typst query` exited with a non-zero status.
  QueryFailed {
    /// Captured stderr or stdout from the subprocess.
    stderr: String,
  },
  /// `typst watch` exited with a non-zero status.
  WatchFailed {
    /// Captured stderr or stdout from the subprocess.
    stderr: String,
  },
  /// The resolved `--content` file lies outside the discovered project
  /// root. Typst can only read files under `--root`, so no spelling of
  /// the path would let it load the file.
  ContentOutsideRoot {
    /// The content file, canonicalized where possible.
    content: PathBuf,
    /// The discovered project root, canonicalized where possible.
    root: PathBuf,
  },
  /// Destination directory already exists and `--force` was not specified.
  DestinationAlreadyExists {
    /// Destination directory path.
    path: PathBuf,
  },
  /// The subprocess could not be spawned or its output could not be read.
  Io(std::io::Error),
}

impl fmt::Display for EngineError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      EngineError::TypstNotFound { instructions } => {
        write!(f, "typst executable not found on PATH.\n{instructions}")
      }
      EngineError::FontDirNotFound { searched } => {
        write!(
          f,
          "No valid font directory found. Searched locations:\n{}",
          searched
            .iter()
            .map(|p| format!("  - {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n")
        )
      }
      EngineError::TemplateNotFound { name, known } => {
        write!(
          f,
          "Unknown template '{name}'. Available templates: {}",
          known.join(", ")
        )
      }
      EngineError::CompilationFailed { stderr } => {
        write!(f, "Typst compilation failed:\n{stderr}")
      }
      EngineError::QueryFailed { stderr } => {
        write!(f, "Typst query failed:\n{stderr}")
      }
      EngineError::WatchFailed { stderr } => {
        write!(f, "Typst watch process failed:\n{stderr}")
      }
      EngineError::ContentOutsideRoot { content, root } => {
        write!(
          f,
          "`{}` is outside the project root (`{}`).\n\
           Run rsmk from the directory containing your résumé, or pass \
           `--source` for a template elsewhere.",
          display_path(content),
          display_path(root)
        )
      }
      EngineError::DestinationAlreadyExists { path } => {
        write!(
          f,
          "Destination directory '{}' already exists. Use --force to overwrite.",
          display_path(path)
        )
      }
      EngineError::Io(err) => write!(f, "I/O error: {err}"),
    }
  }
}

impl std::error::Error for EngineError {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    match self {
      EngineError::Io(err) => Some(err),
      _ => None,
    }
  }
}

impl From<std::io::Error> for EngineError {
  fn from(err: std::io::Error) -> Self {
    EngineError::Io(err)
  }
}

impl From<EngineError> for String {
  fn from(err: EngineError) -> Self {
    err.to_string()
  }
}

/// Locates the `typst` binary on the system PATH using the `which` crate.
///
/// # Errors
///
/// Returns an [`EngineError::TypstNotFound`] if `typst` is not installed
/// on `PATH`.
pub fn find_typst_binary() -> Result<PathBuf, EngineError> {
  which("typst").map_err(|_| EngineError::TypstNotFound {
    instructions: concat!(
      "  Windows: winget install --id Typst.Typst\n",
      "  macOS:   brew install typst\n",
      "  Linux:   cargo install --locked typst-cli"
    )
    .to_string(),
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

/// Renders a path already known to be relative to the Typst root as an
/// absolute, forward-slashed virtual path (e.g. `blocks/x.yaml` ->
/// `/blocks/x.yaml`).
fn to_rooted_posix(rel: &Path) -> String {
  let posix = rel.to_string_lossy().replace('\\', "/");
  let trimmed = posix.trim_start_matches('/');
  format!("/{trimmed}")
}

/// Normalizes a content file path into a POSIX-compliant `--input` virtual
/// path for Typst, resolved against `root` (which Typst is given as
/// `--root`).
///
/// # Errors
///
/// Returns [`EngineError::ContentOutsideRoot`] when `content_path` resolves
/// outside `root`. Typst refuses to read files outside `--root`, so there
/// is no string that would make such a path load; rejecting it up front
/// yields a message that names the file the user actually passed instead of
/// a misleading error pointing at the template's `main.typ`.
pub fn normalize_posix_path(
  root: &Path,
  content_path: &Path,
) -> Result<String, EngineError> {
  // Fast path: the content path is already lexically under the root.
  if let Ok(rel) = content_path.strip_prefix(root) {
    return Ok(to_rooted_posix(rel));
  }

  // Resolve `..`, symlinks, drive-letter casing and mixed separators by
  // canonicalizing both sides. On Windows this also gives both paths a
  // matching `\\?\` verbatim prefix so `strip_prefix` can line them up.
  if let Some((canon_root, canon_content)) = root
    .canonicalize()
    .ok()
    .zip(content_path.canonicalize().ok())
  {
    return match canon_content.strip_prefix(&canon_root) {
      Ok(rel) => Ok(to_rooted_posix(rel)),
      Err(_) => Err(EngineError::ContentOutsideRoot {
        content: canon_content,
        root: canon_root,
      }),
    };
  }

  // The paths could not both be canonicalized (the file does not exist yet,
  // or the root does not). A relative path is still meaningful: treat it as
  // root-relative, matching Typst's own resolution rules.
  if content_path.is_relative() {
    let raw = content_path.to_string_lossy().replace('\\', "/");
    let trimmed = raw.trim_start_matches("./").trim_start_matches('/');
    return Ok(format!("/{trimmed}"));
  }

  // An absolute path that neither strips under the root nor canonicalizes
  // cannot be read by Typst under `--root`. Reject it rather than handing
  // Typst a drive-prefixed string it will mangle against `--root`.
  Err(EngineError::ContentOutsideRoot {
    content: content_path.to_path_buf(),
    root: root.to_path_buf(),
  })
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

/// Engine facade coordinating Typst discovery, embedded modular templates,
/// and subprocess execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypstEngine {
  /// Absolute path to the discovered `typst` binary.
  pub typst_binary: PathBuf,
  /// Optional font directory passed to Typst via `--font-path`.
  pub font_path: Option<PathBuf>,
  /// Project root passed to Typst via `--root`.
  pub root_path: PathBuf,
}

impl TypstEngine {
  /// Discovers the Typst binary and optional font directory automatically
  /// relative to project root.
  ///
  /// # Errors
  ///
  /// Returns an error string if `typst` cannot be found on `PATH` or font
  /// directory is invalid.
  pub fn new(font_path_override: Option<&Path>) -> Result<Self, String> {
    let root_path = find_project_root();
    let typst_binary = find_typst_binary().map_err(|e| e.to_string())?;
    let font_path = discover_font_dir(&root_path, font_path_override)
      .map_err(|e| e.to_string())?;
    Ok(Self {
      typst_binary,
      font_path,
      root_path,
    })
  }

  /// Resolves the template path. If a custom template file is provided
  /// and exists, returns it directly (the `template_name` registry pick
  /// is ignored in that case). Otherwise, looks up `template_name` in the
  /// built-in template registry, extracts its modular component tree
  /// into `.resumake/<template_name>/`, and returns its `main.typ`.
  ///
  /// # Errors
  ///
  /// Returns an error string if `template_name` is not a registered
  /// template, or if extracting embedded files to disk fails.
  pub fn resolve_template(
    &self,
    template_name: &str,
    custom_template: Option<&Path>,
  ) -> Result<PathBuf, String> {
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

    let template = match find_embedded_template(template_name) {
      Some(t) => t,
      None => {
        let custom_main = self
          .root_path
          .join("templates")
          .join(template_name)
          .join("main.typ");
        if custom_main.exists() && custom_main.is_file() {
          return Ok(custom_main);
        }

        return Err(
          EngineError::TemplateNotFound {
            name: template_name.to_string(),
            known: known_template_names(),
          }
          .to_string(),
        );
      }
    };

    let cache_dir = self.root_path.join(".resumake").join(template.name);
    fs::create_dir_all(&cache_dir).map_err(|e| {
      format!(
        "Failed to create engine cache directory '{}': {}",
        cache_dir.display(),
        e
      )
    })?;

    fs::write(cache_dir.join("main.typ"), template.entry)
      .map_err(|e| e.to_string())?;

    for file in template.files {
      let dest = cache_dir.join(file.rel_path);
      if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
      }
      fs::write(&dest, file.contents).map_err(|e| e.to_string())?;
    }

    Ok(cache_dir.join("main.typ"))
  }

  /// Compiles a Typst template and content file into an output PDF document.
  ///
  /// # Errors
  ///
  /// Returns an error string if `typst compile` fails.
  pub fn compile(
    &self,
    template: &Path,
    content: &Path,
    output: &Path,
  ) -> Result<(), String> {
    let content_posix = normalize_posix_path(&self.root_path, content)?;
    let mut cmd = Command::new(&self.typst_binary);
    cmd.arg("compile").arg("--root").arg(&self.root_path);

    if let Some(ref fp) = self.font_path {
      cmd.arg("--font-path").arg(fp);
    }

    cmd
      .arg("--input")
      .arg(format!("content={content_posix}"))
      .arg(template)
      .arg(output);

    let res = cmd.output().map_err(|e| e.to_string())?;
    if !res.status.success() {
      let stderr = String::from_utf8_lossy(&res.stderr).trim().to_string();
      let stdout = String::from_utf8_lossy(&res.stdout).trim().to_string();
      let err_msg = if stderr.is_empty() { stdout } else { stderr };
      return Err(format!("Typst compilation failed:\n{err_msg}"));
    }
    Ok(())
  }

  /// Queries Typst metadata elements (e.g. `<bulletinfo>`, `<pageinfo>`)
  /// from the document.
  ///
  /// # Errors
  ///
  /// Returns an error string if `typst query` fails.
  pub fn query_metadata(
    &self,
    template: &Path,
    content: &Path,
    selector: &str,
  ) -> Result<String, String> {
    let content_posix = normalize_posix_path(&self.root_path, content)?;
    let mut cmd = Command::new(&self.typst_binary);
    cmd.arg("query").arg("--root").arg(&self.root_path);

    if let Some(ref fp) = self.font_path {
      cmd.arg("--font-path").arg(fp);
    }

    cmd
      .arg("--input")
      .arg(format!("content={content_posix}"))
      .arg(template)
      .arg(selector)
      .arg("--field")
      .arg("value");

    let res = cmd.output().map_err(|e| e.to_string())?;
    if !res.status.success() {
      let stderr = String::from_utf8_lossy(&res.stderr).trim().to_string();
      let stdout = String::from_utf8_lossy(&res.stdout).trim().to_string();
      let err_msg = if stderr.is_empty() { stdout } else { stderr };
      return Err(format!("Typst query failed:\n{err_msg}"));
    }

    Ok(String::from_utf8_lossy(&res.stdout).trim().to_string())
  }

  /// Starts Typst live watch mode for real-time document recompilation.
  ///
  /// # Errors
  ///
  /// Returns an error string if `typst watch` fails to start.
  pub fn watch(
    &self,
    template: &Path,
    content: &Path,
    output: &Path,
  ) -> Result<(), String> {
    let content_posix = normalize_posix_path(&self.root_path, content)?;
    let mut cmd = Command::new(&self.typst_binary);
    cmd.arg("watch").arg("--root").arg(&self.root_path);

    if let Some(ref fp) = self.font_path {
      cmd.arg("--font-path").arg(fp);
    }

    cmd
      .arg("--input")
      .arg(format!("content={content_posix}"))
      .arg(template)
      .arg(output)
      .stdin(Stdio::inherit())
      .stdout(Stdio::inherit())
      .stderr(Stdio::inherit());

    cmd.status().map_err(|e| e.to_string())?;
    Ok(())
  }
}

/// Validates content schema and runs layout telemetry evaluation without generating a PDF.
///
/// # Errors
///
/// Returns an error string if the content file does not exist, fails schema validation,
/// fails Typst compilation/querying, or violates single-page geometry constraints.
pub fn verify_content(
  content: &Path,
  template_name: &str,
  source: Option<&Path>,
  schema: Option<&Path>,
  font_path: Option<&Path>,
) -> Result<TelemetryReport, String> {
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
  let engine = TypstEngine::new(font_path).map_err(|e| e.to_string())?;
  let resolved_template = engine
    .resolve_template(template_name, source)
    .map_err(|e| e.to_string())?;
  let page_json =
    engine.query_metadata(&resolved_template, content, "<pageinfo>")?;
  let bullets_json =
    engine.query_metadata(&resolved_template, content, "<bulletinfo>")?;
  let report = evaluate_telemetry(&page_json, &bullets_json)?;

  if !report.is_pass() {
    return Err(
      "Dry-run check failed strict single-page layout constraints.".to_string(),
    );
  }

  Ok(report)
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::TempDir;

  #[test]
  fn test_resolve_template_extracts_modular_components() {
    let temp = TempDir::new().unwrap();
    let engine = TypstEngine {
      typst_binary: PathBuf::from("typst"),
      font_path: None,
      root_path: temp.path().to_path_buf(),
    };

    let resolved = engine.resolve_template(DEFAULT_TEMPLATE, None).unwrap();
    assert!(resolved.exists());
    let cache_dir = temp.path().join(".resumake").join("classic");
    assert!(cache_dir.join("main.typ").exists());
    assert!(cache_dir.join("tokens.typ").exists());
    assert!(cache_dir.join("primitives.typ").exists());
    assert!(cache_dir.join("blocks").join("experience.typ").exists());
  }

  #[test]
  fn test_resolve_template_rejects_unknown_name() {
    let temp = TempDir::new().unwrap();
    let engine = TypstEngine {
      typst_binary: PathBuf::from("typst"),
      font_path: None,
      root_path: temp.path().to_path_buf(),
    };

    let err = engine.resolve_template("does-not-exist", None).unwrap_err();
    assert!(err.contains("does-not-exist"));
    assert!(err.contains("classic"));
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

    for block in blocks {
      assert!(
        CLASSIC_TEMPLATE
          .entry
          .contains(&format!("blocks/{block}.typ")),
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
  fn test_normalize_posix_path_rejects_content_outside_root() {
    let root_dir = TempDir::new().unwrap();
    let other_dir = TempDir::new().unwrap();
    let outside = other_dir.path().join("content.yaml");
    fs::write(&outside, "name: Test\n").unwrap();

    let err = normalize_posix_path(root_dir.path(), &outside).unwrap_err();
    match err {
      EngineError::ContentOutsideRoot { .. } => {}
      other => panic!("expected ContentOutsideRoot, got {other:?}"),
    }

    let msg = err.to_string();
    assert!(
      msg.contains("outside the project root"),
      "unexpected message: {msg}"
    );
    assert!(msg.contains("--source"), "unexpected message: {msg}");
  }

  #[test]
  fn test_normalize_posix_path_rejects_nonexistent_absolute_outside_root() {
    let root_dir = TempDir::new().unwrap();
    // An absolute path that does not exist and does not strip under root:
    // still rejected rather than mangled into a drive-prefixed string.
    #[cfg(windows)]
    let outside = Path::new(r"C:\definitely\not\here\content.yaml");
    #[cfg(not(windows))]
    let outside = Path::new("/definitely/not/here/content.yaml");

    let err = normalize_posix_path(root_dir.path(), outside).unwrap_err();
    assert!(matches!(err, EngineError::ContentOutsideRoot { .. }));
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
      typst_binary: PathBuf::from("typst"),
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
}
