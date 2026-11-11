//! Project manifest (`resumake.toml`) parsing, discovery, and schema export.
//!
//! Owns strongly-typed manifest structures for `[build]`, `[telemetry]`, and
//! `[docs.<name>]`. [`parse`] and [`generate_schema`] stay pure and
//! filesystem-free; filesystem discovery and loading live alongside them in
//! dedicated helpers.

use std::collections;
use std::fs;
use std::path;

use schemars;
use serde;
use serde_json;
use thiserror;
use toml;

use crate::telemetry;

/// Default project manifest filename at repository root.
pub const MANIFEST_FILE: &str = "resumake.toml";

/// Canonical JSON Schema URL for `resumake.toml` IDE completion.
pub const SCHEMA_URL: &str = "https://raw.githubusercontent.com/arvinduh/resumake/main/schemas/manifest.schema.json";

/// Errors originating from manifest parsing and filesystem discovery.
#[derive(Debug, thiserror::Error)]
pub enum Error {
  /// Failed to parse TOML manifest syntax or fields.
  #[error("failed to parse manifest at '{}': {source}", path.display())]
  Parse {
    /// Target manifest path.
    path: path::PathBuf,
    /// Underlying TOML deserialization error.
    #[source]
    source: Box<toml::de::Error>,
  },
  /// Standard I/O error reading a manifest file.
  #[error(transparent)]
  Io(#[from] std::io::Error),
}

/// Shared build configuration defaults under `[build]` in `resumake.toml`.
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BuildDefaults {
  /// Default template name, local template directory, or `.typ` file path.
  #[serde(default)]
  pub template: Option<String>,
  /// Custom font directory relative to project root (or absolute).
  #[serde(default)]
  pub font_path: Option<path::PathBuf>,
  /// Custom content JSON Schema path relative to project root (or absolute).
  #[serde(default)]
  pub schema: Option<path::PathBuf>,
  /// Default PDF output directory relative to project root (or absolute).
  #[serde(default)]
  pub out_dir: Option<path::PathBuf>,
}

/// Configuration for a single named document entry under `[docs.<name>]`.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DocConfig {
  /// Path to the résumé content file (`.yaml`, `.yml`, or `.json`).
  pub content: path::PathBuf,
  /// Whether bare `rsmk build` (and release builds) include this document
  /// (default `false` so new variants remain drafts until opted in).
  #[serde(default)]
  pub publish: bool,
  /// Optional explicit output PDF filename or path for this document.
  #[serde(default)]
  pub output: Option<path::PathBuf>,
  /// Optional per-document template override.
  #[serde(default)]
  pub template: Option<String>,
  /// Optional per-document content JSON Schema override.
  #[serde(default)]
  pub schema: Option<path::PathBuf>,
  /// Optional per-document font directory override.
  #[serde(default)]
  pub font_path: Option<path::PathBuf>,
}

/// Top-level `resumake.toml` project manifest.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
  /// Shared build defaults under `[build]`.
  #[serde(default)]
  pub build: BuildDefaults,
  /// Layout telemetry thresholds under `[telemetry]`.
  #[serde(default)]
  pub telemetry: telemetry::TelemetryPolicy,
  /// Named résumé document targets under `[docs.<name>]`.
  #[serde(default)]
  pub docs: collections::BTreeMap<String, DocConfig>,
}

/// Parses a `resumake.toml` manifest string into [`Manifest`].
///
/// # Errors
/// Returns [`Error::Parse`] if the TOML syntax is invalid or contains unknown
/// fields.
pub fn parse(content: &str) -> Result<Manifest, Error> {
  toml::from_str(content).map_err(|source| Error::Parse {
    path: path::PathBuf::from(MANIFEST_FILE),
    source: Box::new(source),
  })
}

/// Generates the canonical JSON Schema (Draft-07) for `resumake.toml`.
#[must_use]
pub fn generate_schema() -> serde_json::Value {
  let settings = schemars::generate::SchemaSettings::draft07().with(|s| {
    s.definitions_path = "#/$defs/".into();
  });
  let generator = settings.into_generator();
  let schema = generator.into_root_schema_for::<Manifest>();
  serde_json::to_value(schema).unwrap_or(serde_json::Value::Null)
}

/// Searches `start` and its ancestors for a `resumake.toml` manifest, halting
/// at a `.git` repository root boundary.
#[must_use]
pub fn discover(start: &path::Path) -> Option<path::PathBuf> {
  let base = if start.is_file() {
    start
      .parent()
      .filter(|p| !p.as_os_str().is_empty())
      .unwrap_or_else(|| path::Path::new("."))
  } else {
    start
  };
  let canonical = base.canonicalize();
  let search_root = canonical.as_deref().unwrap_or(base);

  for dir in search_root.ancestors() {
    let candidate = dir.join(MANIFEST_FILE);
    if candidate.is_file() {
      return Some(candidate);
    }
    if dir.join(".git").exists() {
      break;
    }
  }
  None
}

/// Loads and parses a `resumake.toml` manifest from disk.
///
/// # Errors
/// Returns [`Error::Io`] if reading fails or [`Error::Parse`] if TOML decoding
/// fails.
pub fn load(manifest_path: &path::Path) -> Result<Manifest, Error> {
  let raw = fs::read_to_string(manifest_path)?;
  toml::from_str(&raw).map_err(|source| Error::Parse {
    path: manifest_path.to_path_buf(),
    source: Box::new(source),
  })
}

/// Resolves a manifest-configured path relative to `project_root` when not
/// already absolute.
#[must_use]
pub fn resolve_path(
  project_root: &path::Path,
  path: &path::Path,
) -> path::PathBuf {
  if path.is_absolute() {
    path.to_path_buf()
  } else {
    project_root.join(path)
  }
}
