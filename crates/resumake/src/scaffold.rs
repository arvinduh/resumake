//! Workspace and repository scaffolding for résumé projects.
//!
//! Owns generating starter `content.yaml` files, `.gitignore`, `.gitattributes`,
//! and GitHub Actions workflows for continuous compilation and releases.

use std::fs;
use std::path;
use std::process;

use thiserror;

/// Bundled starter `content.yaml` scaffold.
const INIT_TEMPLATE: &str = include_str!("embedded/init_template.yaml");

/// Bundled GitHub Actions CI workflow template.
const CI_WORKFLOW: &str = include_str!("embedded/workflows/ci.yml");

/// Bundled GitHub Actions Release workflow template.
const RELEASE_WORKFLOW: &str = include_str!("embedded/workflows/release.yml");

/// Canonical schema URL for IDE intellisense.
pub const SCHEMA_URL: &str =
  "https://raw.githubusercontent.com/arvinduh/resumake/main/schema.json";

/// Errors originating from workspace scaffolding.
#[derive(Debug, thiserror::Error)]
pub enum Error {
  /// Target file already exists and `--force` was not provided.
  #[error("file '{}' already exists; use --force to overwrite", path.display())]
  AlreadyExists {
    /// Conflicting path.
    path: path::PathBuf,
  },
  /// Target directory is not empty and `--force` was not provided.
  #[error("target directory '{}' is not empty; use --force to proceed", path.display())]
  DirectoryNotEmpty {
    /// Non-empty directory path.
    path: path::PathBuf,
  },
  /// Template ejection failure.
  #[error(transparent)]
  Template(#[from] crate::templates::Error),
  /// Standard I/O error during file operations.
  #[error(transparent)]
  Io(#[from] std::io::Error),
}

/// Options configuring workspace initialization.
#[derive(Debug, Clone)]
#[expect(
  clippy::struct_excessive_bools,
  reason = "scaffolding configuration flags"
)]
pub struct InitOptions<'a> {
  /// Candidate full name.
  pub name: &'a str,
  /// Target directory to initialize in.
  pub target_dir: &'a path::Path,
  /// Whether to overwrite existing files.
  pub force: bool,
  /// Whether to skip git repository initialization and git files.
  pub no_git: bool,
  /// Whether to skip GitHub Actions workflow creation.
  pub no_workflows: bool,
  /// Whether to eject the template into `templates/<name>.typ`.
  pub eject_template: bool,
  /// Template name to use or eject (e.g. "classic").
  pub template: &'a str,
}

/// Result summary of scaffolding a résumé repository.
#[derive(Debug, Default)]
pub struct InitOutcome {
  /// Path to the scaffolded content YAML file.
  pub content_path: path::PathBuf,
  /// All files created during initialization.
  pub created_files: Vec<path::PathBuf>,
  /// Whether `git init` was executed.
  pub git_initialized: bool,
}

/// Generates starter `content.yaml` text with candidate name and schema header.
#[must_use]
pub fn content_template(candidate_name: &str) -> String {
  let name = if candidate_name.trim().is_empty() {
    "Jane Doe"
  } else {
    candidate_name.trim()
  };

  INIT_TEMPLATE
    .replace("CANDIDATE_NAME", name)
    .replace("RESUMAKE_SCHEMA_URL", SCHEMA_URL)
}

/// Generates GitHub Actions CI workflow text with pinned CLI version.
#[must_use]
pub fn ci_workflow() -> String {
  CI_WORKFLOW.replace("RSMK_VERSION", env!("CARGO_PKG_VERSION"))
}

/// Generates GitHub Actions release workflow text with pinned CLI version.
#[must_use]
pub fn release_workflow() -> String {
  RELEASE_WORKFLOW.replace("RSMK_VERSION", env!("CARGO_PKG_VERSION"))
}

/// Scaffolds a new résumé project into `options.target_dir`.
///
/// # Errors
/// Returns [`Error::AlreadyExists`] if files exist without `force`, or
/// [`Error::Io`] on filesystem write failure.
pub fn init(options: &InitOptions<'_>) -> Result<InitOutcome, Error> {
  fs::create_dir_all(options.target_dir)?;

  let mut outcome = InitOutcome::default();
  let content_file = options.target_dir.join("content.yaml");

  if content_file.exists() && !options.force {
    return Err(Error::AlreadyExists { path: content_file });
  }

  let content_str = content_template(options.name);
  fs::write(&content_file, content_str)?;
  outcome.content_path.clone_from(&content_file);
  outcome.created_files.push(content_file);

  if options.eject_template {
    let tpl_file = crate::templates::eject(
      options.template,
      options.target_dir,
      options.force,
    )?;
    outcome.created_files.push(tpl_file);
  }

  if !options.no_workflows {
    let workflows_dir = options.target_dir.join(".github").join("workflows");
    fs::create_dir_all(&workflows_dir)?;

    let ci_file = workflows_dir.join("ci.yml");
    if options.force || !ci_file.exists() {
      fs::write(&ci_file, ci_workflow())?;
      outcome.created_files.push(ci_file);
    }

    let release_file = workflows_dir.join("release.yml");
    if options.force || !release_file.exists() {
      fs::write(&release_file, release_workflow())?;
      outcome.created_files.push(release_file);
    }
  }

  if !options.no_git {
    let gitignore_file = options.target_dir.join(".gitignore");
    if options.force || !gitignore_file.exists() {
      fs::write(&gitignore_file, "*.pdf\n.rsmk/\n")?;
      outcome.created_files.push(gitignore_file);
    }

    let gitattributes_file = options.target_dir.join(".gitattributes");
    if options.force || !gitattributes_file.exists() {
      fs::write(&gitattributes_file, "* text=auto eol=lf\n")?;
      outcome.created_files.push(gitattributes_file);
    }

    let git_dir = options.target_dir.join(".git");
    if !git_dir.exists() {
      let status = process::Command::new("git")
        .arg("init")
        .current_dir(options.target_dir)
        .status();
      if let Ok(s) = status
        && s.success()
      {
        outcome.git_initialized = true;
      }
    }
  }

  Ok(outcome)
}
