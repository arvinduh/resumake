//! Typst engine orchestration, embedded component cache, and subprocess execution.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use which::which;

// Embedded modular Typst engine components
const EMBEDDED_MAIN: &str = include_str!("embedded/main.typ");
const EMBEDDED_TOKENS: &str = include_str!("embedded/tokens.typ");
const EMBEDDED_PRIMITIVES: &str = include_str!("embedded/primitives.typ");
const EMBEDDED_BLOCK_EDU: &str = include_str!("embedded/blocks/education.typ");
const EMBEDDED_BLOCK_EXP: &str = include_str!("embedded/blocks/experience.typ");
const EMBEDDED_BLOCK_PROJ: &str = include_str!("embedded/blocks/projects.typ");
const EMBEDDED_BLOCK_SKILLS: &str = include_str!("embedded/blocks/skills.typ");
const EMBEDDED_BLOCK_PUBS: &str =
  include_str!("embedded/blocks/publications.typ");
const EMBEDDED_BLOCK_SPLIT: &str =
  include_str!("embedded/blocks/split_line.typ");
const EMBEDDED_BLOCK_REFS: &str =
  include_str!("embedded/blocks/references.typ");
const EMBEDDED_BLOCK_LINES: &str = include_str!("embedded/blocks/lines.typ");

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
/// Returns an [`EngineError::TypstNotFound`] if `typst` is not installed on `PATH`.
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

/// Discovers the font directory if present (user override, `./fonts`, or `assets/fonts`).
///
/// # Errors
///
/// Returns an [`EngineError::FontDirNotFound`] if a custom font path was provided
/// but does not exist.
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

/// Normalizes a content file path into a POSIX-compliant input path for Typst.
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

/// Finds project root by checking for markers (`resume.yaml`, `content.yaml`, `Cargo.toml`, `.git`)
/// or defaulting to current directory.
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

/// Engine facade coordinating Typst discovery, embedded modular templates, and subprocess execution.
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
  /// Discovers the Typst binary and optional font directory automatically relative to project root.
  ///
  /// # Errors
  ///
  /// Returns an error string if `typst` cannot be found on `PATH` or font directory is invalid.
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

  /// Resolves the template path. If a custom template is provided and exists, returns it.
  /// Otherwise, extracts the embedded modular component tree into `.resumake/` and returns `main.typ`.
  ///
  /// # Errors
  ///
  /// Returns an error string if extracting embedded files to disk fails.
  pub fn resolve_template(
    &self,
    custom_template: Option<&Path>,
  ) -> Result<PathBuf, String> {
    if let Some(tpl) = custom_template.filter(|p| p.exists()) {
      return Ok(tpl.to_path_buf());
    }

    let cache_dir = self.root_path.join(".resumake");
    let blocks_dir = cache_dir.join("blocks");
    fs::create_dir_all(&blocks_dir).map_err(|e| {
      format!(
        "Failed to create engine cache directory '{}': {}",
        blocks_dir.display(),
        e
      )
    })?;

    // Write modular components to cache
    fs::write(cache_dir.join("main.typ"), EMBEDDED_MAIN)
      .map_err(|e| e.to_string())?;
    fs::write(cache_dir.join("tokens.typ"), EMBEDDED_TOKENS)
      .map_err(|e| e.to_string())?;
    fs::write(cache_dir.join("primitives.typ"), EMBEDDED_PRIMITIVES)
      .map_err(|e| e.to_string())?;
    fs::write(blocks_dir.join("education.typ"), EMBEDDED_BLOCK_EDU)
      .map_err(|e| e.to_string())?;
    fs::write(blocks_dir.join("experience.typ"), EMBEDDED_BLOCK_EXP)
      .map_err(|e| e.to_string())?;
    fs::write(blocks_dir.join("projects.typ"), EMBEDDED_BLOCK_PROJ)
      .map_err(|e| e.to_string())?;
    fs::write(blocks_dir.join("skills.typ"), EMBEDDED_BLOCK_SKILLS)
      .map_err(|e| e.to_string())?;
    fs::write(blocks_dir.join("publications.typ"), EMBEDDED_BLOCK_PUBS)
      .map_err(|e| e.to_string())?;
    fs::write(blocks_dir.join("split_line.typ"), EMBEDDED_BLOCK_SPLIT)
      .map_err(|e| e.to_string())?;
    fs::write(blocks_dir.join("references.typ"), EMBEDDED_BLOCK_REFS)
      .map_err(|e| e.to_string())?;
    fs::write(blocks_dir.join("lines.typ"), EMBEDDED_BLOCK_LINES)
      .map_err(|e| e.to_string())?;

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

  /// Queries Typst metadata elements (e.g. `<bulletinfo>`, `<pageinfo>`) from the document.
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

    let resolved = engine.resolve_template(None).unwrap();
    assert!(resolved.exists());
    assert!(temp.path().join(".resumake").join("main.typ").exists());
    assert!(temp.path().join(".resumake").join("tokens.typ").exists());
    assert!(temp
      .path()
      .join(".resumake")
      .join("primitives.typ")
      .exists());
    assert!(temp
      .path()
      .join(".resumake")
      .join("blocks")
      .join("experience.typ")
      .exists());
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
        EMBEDDED_MAIN.contains(&format!("blocks/{block}.typ")),
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
