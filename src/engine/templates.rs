//! Embedded and custom résumé template registry, discovery, and extraction.

use include_dir::{include_dir, Dir};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// A single Typst source file belonging to an embedded template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateFile {
  /// Path relative to the template root, using forward slashes.
  pub rel_path: String,
  /// Embedded file contents.
  pub contents: &'static str,
}

/// A complete named résumé template bundled into the binary.
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
pub static TEMPLATES_DIR: Dir<'_> =
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
  let root = crate::utils::fs::find_project_root();
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
/// Returns [`crate::engine::EngineError::TemplateNotFound`] if `name` is not in the built-in registry.
/// Returns [`crate::engine::EngineError::DestinationAlreadyExists`] if `target_dir` exists and `force` is false.
pub fn eject_template(
  name: &str,
  target_dir: &Path,
  force: bool,
) -> Result<Vec<String>, crate::engine::EngineError> {
  let template = find_embedded_template(name).ok_or_else(|| {
    crate::engine::EngineError::TemplateNotFound {
      name: name.to_string(),
      known: known_template_names(),
    }
  })?;

  if target_dir.exists() && !force {
    return Err(crate::engine::EngineError::DestinationAlreadyExists {
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
