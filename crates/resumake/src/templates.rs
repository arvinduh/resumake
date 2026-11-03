//! Résumé template management, discovery, and file ejection.
//!
//! Owns resolving embedded and disk templates, enumerating available templates,
//! and ejecting single-file `.typ` templates into user repositories.

use std::fs;
use std::path;

use thiserror;

/// Default embedded template name.
pub const DEFAULT_TEMPLATE: &str = "classic";

/// Bundled canonical classic Typst template.
pub const EMBEDDED_CLASSIC: &str =
  include_str!("embedded/templates/classic.typ");

/// Known embedded template names.
const KNOWN_TEMPLATES: &[&str] = &[DEFAULT_TEMPLATE];

/// Errors originating from template operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
  /// The requested template could not be found.
  #[error("unknown template '{name}'; available templates: {}", known.join(", "))]
  NotFound {
    /// Requested template identifier.
    name: String,
    /// Registered template names.
    known: Vec<String>,
  },
  /// Target template file already exists and overwrite was not forced.
  #[error("template file '{}' already exists; use --force to overwrite", path.display())]
  AlreadyExists {
    /// Path to existing file.
    path: path::PathBuf,
  },
  /// I/O error during template resolution or ejection.
  #[error(transparent)]
  Io(#[from] std::io::Error),
}

/// Resolved template source: either raw embedded Typst code or a path on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateSource {
  /// Embedded in-memory Typst source string.
  Embedded {
    /// Template identifier.
    name: String,
    /// Raw Typst markup content.
    source: &'static str,
  },
  /// On-disk Typst file.
  File(path::PathBuf),
}

/// Returns the list of bundled embedded template names.
#[must_use]
pub fn list() -> &'static [&'static str] {
  KNOWN_TEMPLATES
}

/// Returns the embedded template source by name if present.
#[must_use]
pub fn get_embedded(name: &str) -> Option<&'static str> {
  let clean = name.trim().trim_end_matches(".typ");
  if clean.eq_ignore_ascii_case(DEFAULT_TEMPLATE) {
    Some(EMBEDDED_CLASSIC)
  } else {
    None
  }
}

/// Resolves a template name or path against project root.
///
/// # Errors
/// Returns [`Error::NotFound`] if the template is not an existing file or
/// known embedded template.
pub fn resolve(
  template_name: &str,
  project_root: &path::Path,
) -> Result<TemplateSource, Error> {
  let direct = path::Path::new(template_name);
  if direct.is_file() {
    return Ok(TemplateSource::File(direct.to_path_buf()));
  }

  if path::Path::new(template_name)
    .extension()
    .is_some_and(|ext| ext.eq_ignore_ascii_case("typ"))
  {
    let joined = project_root.join(template_name);
    if joined.is_file() {
      return Ok(TemplateSource::File(joined));
    }
  }

  let custom_file = project_root
    .join("templates")
    .join(format!("{}.typ", template_name.trim_end_matches(".typ")));
  if custom_file.is_file() {
    return Ok(TemplateSource::File(custom_file));
  }

  let clean = template_name.trim().trim_end_matches(".typ");
  if let Some(src) = get_embedded(clean) {
    return Ok(TemplateSource::Embedded {
      name: clean.to_string(),
      source: src,
    });
  }

  Err(Error::NotFound {
    name: template_name.to_string(),
    known: KNOWN_TEMPLATES.iter().map(ToString::to_string).collect(),
  })
}

/// Ejects an embedded template into `dest_dir/templates/<name>.typ`.
///
/// # Errors
/// Returns [`Error::NotFound`] if template is unknown,
/// [`Error::AlreadyExists`] if file exists without `force`, or [`Error::Io`]
/// on write failure.
pub fn eject(
  template_name: &str,
  dest_dir: &path::Path,
  force: bool,
) -> Result<path::PathBuf, Error> {
  let clean = template_name.trim().trim_end_matches(".typ");
  let Some(source) = get_embedded(clean) else {
    return Err(Error::NotFound {
      name: template_name.to_string(),
      known: KNOWN_TEMPLATES.iter().map(ToString::to_string).collect(),
    });
  };

  let target_dir = dest_dir.join("templates");
  fs::create_dir_all(&target_dir)?;
  let target_file = target_dir.join(format!("{clean}.typ"));

  if target_file.exists() && !force {
    return Err(Error::AlreadyExists { path: target_file });
  }

  fs::write(&target_file, source)?;
  Ok(target_file)
}
