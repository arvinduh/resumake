//! Resumake core résumé compiler and layout telemetry engine.
//!
//! Provides in-process Typst document compilation, Draft-07 JSON Schema
//! generation and validation, and layout telemetry analysis.

#![deny(clippy::print_stdout, clippy::print_stderr)]

pub mod manifest;
pub mod render;
pub mod scaffold;
pub mod schema;
pub mod telemetry;
pub mod templates;

pub use render::{compile, export_pdf};
pub use scaffold::init;
pub use schema::{ContentFormat, generate_schema, validate, validate_file};
pub use telemetry::{TelemetryPolicy, TelemetryReport, evaluate};
pub use templates::{eject, list, resolve};
