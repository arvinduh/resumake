//! Crate-level error types, result aliases, and classification helpers.

pub use crate::commands::build::WatchError;
use crate::commands::init::InitError;
use crate::commands::release::ReleaseError;
use crate::commands::update::UpdateError;
use crate::engine::error::EngineError;
use crate::schema::SchemaError;
use crate::telemetry::TelemetryError;
use crate::utils::git::GitError;

/// A specialized [`Result`](std::result::Result) type for Resumake operations.
pub type Result<T, E = ResumakeError> = std::result::Result<T, E>;

/// Unified umbrella error type covering all failures across Resumake subsystems.
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum ResumakeError {
  /// Engine compilation, template resolution, or Typst error.
  #[error(transparent)]
  Engine(#[from] EngineError),
  /// Content schema validation or inspection error.
  #[error(transparent)]
  Schema(#[from] SchemaError),
  /// Layout geometry or telemetry evaluation error.
  #[error(transparent)]
  Telemetry(#[from] TelemetryError),
  /// Git or GitHub CLI operation error.
  #[error(transparent)]
  Git(#[from] GitError),
  /// Workspace initialization or workflow scaffolding error.
  #[error(transparent)]
  Init(#[from] InitError),
  /// Release pipeline, semver monotonicity, or tag error.
  #[error(transparent)]
  Release(#[from] ReleaseError),
  /// In-place binary update error.
  #[error(transparent)]
  Update(#[from] UpdateError),
  /// File watching or hot-reload error.
  #[error(transparent)]
  Watch(#[from] WatchError),
  /// Underlying standard I/O error.
  #[error("I/O error: {0}")]
  Io(#[from] std::io::Error),
}

impl ResumakeError {
  /// Returns `true` if the error was caused by engine compilation or Typst execution.
  #[inline]
  pub fn is_engine(&self) -> bool {
    matches!(self, Self::Engine(_))
  }

  /// Returns `true` if the error was caused by content schema validation.
  #[inline]
  pub fn is_schema(&self) -> bool {
    matches!(self, Self::Schema(_))
  }

  /// Returns `true` if the error represents a 1-page layout constraint failure.
  #[inline]
  pub fn is_layout_overflow(&self) -> bool {
    matches!(self, Self::Engine(EngineError::LayoutConstraintViolation))
  }
}
