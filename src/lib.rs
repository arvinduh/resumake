//! Resumake core library.
//!
//! Provides the in-process Typst engine compiler, résumé data structures,
//! layout geometry telemetry evaluator, JSON schema validator, and core
//! utilities.

/// In-process Typst engine orchestration and template registry.
pub mod engine;
/// Canonical résumé data structures and Serde models.
pub mod models;
/// JSON Schema generator and validation logic.
pub mod schema;
/// 1-page layout geometry calculations and telemetry evaluation.
pub mod telemetry;
/// Cross-cutting shared utilities (filesystem, git).
pub mod utils;
