//! Resumake core library.

#![deny(dead_code, unused_imports, unused_variables)]
#![warn(missing_docs)]

/// CLI options, subcommand argument definitions, and Clap parser.
pub mod cli;
/// CLI subcommand implementations and dispatcher.
pub mod commands;
/// In-process Typst engine orchestration and template registry.
pub mod engine;
/// Crate-level error types, result aliases, and classification helpers.
pub mod error;
/// Canonical résumé data structures and Serde models.
pub mod models;
/// JSON Schema generator and validation logic.
pub mod schema;
/// 1-page layout geometry calculations and telemetry evaluation.
pub mod telemetry;
/// Cross-cutting shared utilities (filesystem, git, terminal UI).
pub mod utils;

pub use crate::commands::{init, release, update};
pub use crate::error::{Result, ResumakeError, WatchError};
pub use crate::utils::ui;
