//! Strongly-typed domain errors and consolidated result types.

pub use crate::engine::EngineError;
pub use crate::init::InitError;
pub use crate::release::ReleaseError;
pub use crate::schema::SchemaError;
pub use crate::telemetry::TelemetryError;
pub use crate::update::UpdateError;
use std::path::PathBuf;

/// Errors originating from file watching and hot-reload debouncing.
#[derive(thiserror::Error, Debug)]
pub enum WatchError {
  /// Failed to initialize file watcher.
  #[error("Failed to initialize file watcher: {0}")]
  Init(#[source] notify_debouncer_mini::notify::Error),
  /// Failed to register watch path.
  #[error("Failed to watch path '{}': {source}", path.display())]
  WatchPath {
    /// Target path.
    path: PathBuf,
    /// Underlying notify error.
    #[source]
    source: notify_debouncer_mini::notify::Error,
  },
}

/// Unified top-level error enum covering all resumake subsystems.
#[derive(thiserror::Error, Debug)]
pub enum ResumakeError {
  /// Engine compilation, template resolution, or Typst subprocess error.
  #[error(transparent)]
  Engine(#[from] EngineError),
  /// Content schema validation or inspection error.
  #[error(transparent)]
  Schema(#[from] SchemaError),
  /// Layout geometry or telemetry evaluation error.
  #[error(transparent)]
  Telemetry(#[from] TelemetryError),
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
}
