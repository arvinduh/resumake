//! Errors originating from the Typst execution engine.

use crate::utils::fs::display_path;
use std::path::PathBuf;

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
