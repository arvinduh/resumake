//! Workspace and repository scaffolding for résumé projects.
//!
//! Owns generating starter `content.yaml`/`content.json` files,
//! `resumake.toml` manifests, `.gitignore`, `.gitattributes`, and GitHub
//! Actions workflows for continuous compilation and releases.

use std::fs;
use std::path;
use std::process;

use serde_json;
use serde_yaml;
use thiserror;

/// Bundled starter `content.yaml` scaffold.
const INIT_TEMPLATE: &str = include_str!("embedded/init_template.yaml");

/// Bundled starter `resumake.toml` manifest scaffold.
const INIT_MANIFEST: &str = include_str!("embedded/init_manifest.toml");

/// Bundled GitHub Actions CI workflow template.
const CI_WORKFLOW: &str = include_str!("embedded/workflows/ci.yml");

/// Bundled GitHub Actions Release workflow template.
const RELEASE_WORKFLOW: &str = include_str!("embedded/workflows/release.yml");

/// Canonical schema URL for content IDE completion.
pub const SCHEMA_URL: &str = "https://raw.githubusercontent.com/arvinduh/resumake/main/schemas/content.schema.json";

/// Errors originating from workspace scaffolding.
#[derive(Debug, thiserror::Error)]
pub enum Error {
  /// Target file already exists and overwriting was not requested.
  #[error("file '{}' already exists", path.display())]
  AlreadyExists {
    /// Conflicting path.
    path: path::PathBuf,
  },
  /// Template ejection failure.
  #[error(transparent)]
  Template(#[from] crate::templates::Error),
  /// Failed to parse starter template YAML when converting formats.
  #[error(transparent)]
  Yaml(#[from] serde_yaml::Error),
  /// Failed to serialize JSON starter content.
  #[error(transparent)]
  Json(#[from] serde_json::Error),
  /// Standard I/O error during file operations.
  #[error(transparent)]
  Io(#[from] std::io::Error),
}

/// Options configuring workspace initialization.
#[derive(Debug)]
#[expect(
  clippy::struct_excessive_bools,
  reason = "scaffolding configuration flags"
)]
pub struct InitOptions<'a> {
  /// Candidate full name.
  pub name: &'a str,
  /// Target directory to initialize in.
  pub target_dir: &'a path::Path,
  /// Content format (`yaml` or `json`) for starter content.
  pub format: crate::schema::ContentFormat,
  /// Whether to skip scaffolding `resumake.toml`.
  pub no_manifest: bool,
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
  /// Path to the scaffolded content file.
  pub content_path: path::PathBuf,
  /// Path to the scaffolded `resumake.toml` manifest, if written.
  pub manifest_path: Option<path::PathBuf>,
  /// All files created during initialization.
  pub created_files: Vec<path::PathBuf>,
  /// Whether `git init` was executed.
  pub git_initialized: bool,
}

/// Generates starter résumé content text in the requested format.
///
/// # Errors
/// Returns [`Error::Yaml`] or [`Error::Json`] if converting to JSON fails.
pub fn content_template(
  candidate_name: &str,
  format: crate::schema::ContentFormat,
) -> Result<String, Error> {
  let name = if candidate_name.trim().is_empty() {
    "Jane Doe"
  } else {
    candidate_name.trim()
  };

  let yaml = INIT_TEMPLATE
    .replace("CANDIDATE_NAME", name)
    .replace("RESUMAKE_SCHEMA_URL", SCHEMA_URL);

  match format {
    crate::schema::ContentFormat::Yaml => Ok(yaml),
    crate::schema::ContentFormat::Json => {
      let mut obj: serde_json::Map<String, serde_json::Value> =
        serde_yaml::from_str(&yaml)?;
      obj.insert(
        "$schema".to_string(),
        serde_json::Value::String(SCHEMA_URL.to_string()),
      );
      let mut out = serde_json::to_string_pretty(&obj)?;
      out.push('\n');
      Ok(out)
    }
  }
}

/// Generates starter `resumake.toml` text pinning template, telemetry, and
/// `main` document.
#[must_use]
pub fn manifest_template(
  template_name: &str,
  format: crate::schema::ContentFormat,
) -> String {
  let template = if template_name.trim().is_empty() {
    crate::templates::DEFAULT_TEMPLATE
  } else {
    template_name.trim()
  };
  let rendered = INIT_MANIFEST
    .replace("RESUMAKE_MANIFEST_SCHEMA_URL", crate::manifest::SCHEMA_URL)
    .replace("INIT_CONTENT_FILE", format.default_filename());

  if template == crate::templates::DEFAULT_TEMPLATE {
    rendered
  } else {
    rendered.replacen(
      &format!("template = \"{}\"", crate::templates::DEFAULT_TEMPLATE),
      &format!("template = \"{}\"", template.replace('"', "\\\"")),
      1,
    )
  }
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

  let content_file = options.target_dir.join(options.format.default_filename());
  if content_file.exists() && !options.force {
    return Err(Error::AlreadyExists { path: content_file });
  }

  let manifest_file = options.target_dir.join(crate::manifest::MANIFEST_FILE);
  if !options.no_manifest && manifest_file.exists() && !options.force {
    return Err(Error::AlreadyExists {
      path: manifest_file,
    });
  }

  let mut outcome = InitOutcome::default();

  let content_str = content_template(options.name, options.format)?;
  fs::write(&content_file, content_str)?;
  outcome.content_path.clone_from(&content_file);
  outcome.created_files.push(content_file);

  if !options.no_manifest {
    let manifest_str = manifest_template(options.template, options.format);
    fs::write(&manifest_file, manifest_str)?;
    outcome.manifest_path = Some(manifest_file.clone());
    outcome.created_files.push(manifest_file);
  }

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
