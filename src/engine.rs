//! Typst engine orchestration, embedded component cache, and subprocess
//! execution.

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
/// To stay compatible with `resumake check`/`build` telemetry, a
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

/// Normalizes a content file path into a POSIX-compliant input path for
/// Typst.
pub fn normalize_posix_path(root: &Path, content_path: &Path) -> String {
  if let Ok(rel) = content_path.strip_prefix(root) {
    let posix = rel.to_string_lossy().replace('\\', "/");
    let trimmed = posix.trim_start_matches('/');
    return format!("/{trimmed}");
  }

  let canon_rel = root
    .canonicalize()
    .ok()
    .zip(content_path.canonicalize().ok())
    .and_then(|(r, c)| c.strip_prefix(&r).ok().map(|p| p.to_path_buf()));

  if let Some(rel) = canon_rel {
    let posix = rel.to_string_lossy().replace('\\', "/");
    let trimmed = posix.trim_start_matches('/');
    return format!("/{trimmed}");
  }

  if content_path.is_relative() {
    let raw = content_path.to_string_lossy().replace('\\', "/");
    let trimmed = raw
      .trim_start_matches("./")
      .trim_start_matches(".\\")
      .trim_start_matches('/');
    return format!("/{trimmed}");
  }

  content_path.to_string_lossy().replace('\\', "/")
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

    let template = find_embedded_template(template_name).ok_or_else(|| {
      EngineError::TemplateNotFound {
        name: template_name.to_string(),
        known: known_template_names(),
      }
      .to_string()
    })?;

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
    let content_posix = normalize_posix_path(&self.root_path, content);
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
    let content_posix = normalize_posix_path(&self.root_path, content);
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
    let content_posix = normalize_posix_path(&self.root_path, content);
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
      normalize_posix_path(root, Path::new("content.yaml")),
      "/content.yaml"
    );
    assert_eq!(
      normalize_posix_path(root, Path::new("./content.yaml")),
      "/content.yaml"
    );
  }
}
