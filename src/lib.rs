//! Resumake core library.
//!
//! Provides the in-process Typst engine compiler, résumé data structures,
//! layout geometry telemetry evaluator, JSON schema validator, workspace
//! scaffolding, release orchestration, and core utilities.

/// In-process Typst engine orchestration, template registry, and build
/// pipeline.
pub mod engine;
/// Canonical résumé data structures and Serde models.
pub mod models;
/// Release pre-flight verification, semver validation, and tag orchestration.
pub mod release;
/// JSON Schema generator and validation logic.
pub mod schema;
/// 1-page layout geometry calculations and telemetry evaluation.
pub mod telemetry;
/// Cross-cutting shared utilities (filesystem, git).
pub mod utils;
/// Workspace scaffolding, workflow lifecycle, and git configuration.
pub mod workspace;

// Primary compiler facade and build pipeline
pub use engine::{build, BuildOptions, BuildOutcome, TypstEngine};

// Canonical résumé data model
pub use models::ResumeDocument;

// JSON Schema validation
pub use schema::{validate_schema_auto, validate_schema_str};

// Layout geometry evaluation
pub use telemetry::{evaluate_telemetry, TelemetryReport};

// Module error aliases for unambiguous caller matching
pub use engine::Error as EngineError;
pub use release::Error as ReleaseError;
pub use schema::Error as SchemaError;
pub use telemetry::Error as TelemetryError;
pub use workspace::Error as WorkspaceError;
