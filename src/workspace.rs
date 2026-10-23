//! Workspace scaffolding, template generation, and workflow lifecycle
//! management.

use std::path::{Path, PathBuf};

use crate::schema;
use crate::utils::{fs, git};

/// Errors originating from project initialization and workflow management.
#[derive(Debug, thiserror::Error)]
pub enum Error {
  /// Destination file already exists and `--force` was not specified.
  #[error("File '{}' already exists. Use --force to overwrite.", path.display())]
  DestinationAlreadyExists {
    /// Path to the conflicting file.
    path: PathBuf,
  },
  /// Target directory is not empty and `--force` was not specified.
  #[error(
    "Directory '{}' is not empty. Use --force to scaffold into it anyway.",
    path.display()
  )]
  DestinationNotEmpty {
    /// Path to the non-empty directory.
    path: PathBuf,
  },
  /// Failed to read a file during initialization.
  #[error("Failed to read file '{}': {source}", path.display())]
  FileRead {
    /// Path to the target file.
    path: PathBuf,
    /// Underlying I/O error.
    #[source]
    source: std::io::Error,
  },
  /// Failed to write a file during initialization.
  #[error("Failed to write file '{}': {source}", path.display())]
  FileWrite {
    /// Path to the target file.
    path: PathBuf,
    /// Underlying I/O error.
    #[source]
    source: std::io::Error,
  },
  /// Git operation error.
  #[error(transparent)]
  Git(#[from] git::Error),
  /// Underlying standard I/O error.
  #[error("I/O error: {0}")]
  Io(#[from] std::io::Error),
}

/// Options configuring workspace initialization.
#[derive(Debug, Clone)]
pub struct InitOptions<'a> {
  /// Candidate display name.
  pub name: &'a str,
  /// Destination path for the new content file.
  pub output: &'a Path,
  /// Whether to overwrite existing files.
  pub force: bool,
  /// Whether to skip git repository initialization and git config files.
  pub no_git: bool,
  /// Whether to skip `GitHub` Actions workflow scaffolding.
  pub no_workflows: bool,
}

/// The result of a workspace initialization operation.
#[derive(Debug, Default)]
pub struct InitOutcome {
  /// Path to the created content YAML file.
  pub content_path: PathBuf,
  /// Files created during initialization.
  pub created_files: Vec<PathBuf>,
  /// Whether a git repository was initialized.
  pub git_initialized: bool,
  /// Whether `GitHub` Actions workflows were scaffolded.
  pub workflows_created: bool,
}

/// Status of an individual workflow file update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowUpdateStatus {
  /// Workflow file was newly created.
  Created,
  /// Workflow file had a clean provenance hash and was updated.
  Updated,
  /// Workflow file had local modifications but was updated due to force flag.
  ForceUpdated,
  /// Workflow file had local modifications and was skipped.
  SkippedLocalModifications,
}

/// The outcome of updating an individual workflow file.
#[derive(Debug, Clone)]
pub struct WorkflowUpdateOutcome {
  /// Path to the workflow file.
  pub path: PathBuf,
  /// Status of the update operation.
  pub status: WorkflowUpdateStatus,
}

/// Returns true when `path` should be treated as an explicit content-file
/// target rather than a directory to scaffold inside.
#[must_use]
pub fn looks_like_file(path: &Path) -> bool {
  if path.is_dir() {
    return false;
  }
  if path.is_file() {
    return true;
  }
  matches!(
    path
      .extension()
      .and_then(|e| e.to_str())
      .map(str::to_ascii_lowercase)
      .as_deref(),
    Some("yaml" | "yml")
  )
}

/// Resolves the effective content-file path for workspace initialization.
#[must_use]
pub fn resolve_init_output(dest: Option<&Path>) -> PathBuf {
  match dest {
    None => PathBuf::from("content.yaml"),
    Some(dest) if looks_like_file(dest) => dest.to_path_buf(),
    Some(dest) => dest.join("content.yaml"),
  }
}

/// Returns true when `dir` exists and contains at least one entry.
#[must_use]
pub fn dir_has_entries(dir: &Path) -> bool {
  match std::fs::read_dir(dir) {
    Ok(mut entries) => entries.next().is_some(),
    Err(_) => false,
  }
}

/// Ensures `.gitignore` exists and ignores `*.pdf` and `.resumake/`.
///
/// # Errors
/// Returns [`Error::FileRead`] or [`Error::FileWrite`] if I/O fails.
pub fn ensure_gitignore(dir: &Path) -> Result<bool, Error> {
  let gitignore_path = dir.join(".gitignore");
  let mut existing = if gitignore_path.exists() {
    std::fs::read_to_string(&gitignore_path).map_err(|source| {
      Error::FileRead {
        path: gitignore_path.clone(),
        source,
      }
    })?
  } else {
    String::new()
  };

  let has_pdf = existing.lines().any(|l| l.trim() == "*.pdf");
  let has_resumake = existing
    .lines()
    .any(|l| l.trim() == ".resumake/" || l.trim() == ".resumake");

  let mut modified = false;

  if !has_pdf {
    if !existing.is_empty() && !existing.ends_with('\n') {
      existing.push('\n');
    }
    existing.push_str("*.pdf\n");
    modified = true;
  }
  if !has_resumake {
    if !existing.is_empty() && !existing.ends_with('\n') {
      existing.push('\n');
    }
    existing.push_str(".resumake/\n");
    modified = true;
  }

  if modified || !gitignore_path.exists() {
    std::fs::write(&gitignore_path, existing).map_err(|source| {
      Error::FileWrite {
        path: gitignore_path,
        source,
      }
    })?;
    return Ok(true);
  }
  Ok(false)
}

/// Ensures `.gitattributes` exists and contains `* text=auto eol=lf`.
///
/// # Errors
/// Returns [`Error::FileRead`] or [`Error::FileWrite`] if I/O fails.
pub fn ensure_gitattributes(dir: &Path) -> Result<bool, Error> {
  let gitattributes_path = dir.join(".gitattributes");
  let mut existing = if gitattributes_path.exists() {
    std::fs::read_to_string(&gitattributes_path).map_err(|source| {
      Error::FileRead {
        path: gitattributes_path.clone(),
        source,
      }
    })?
  } else {
    String::new()
  };

  let has_text_attr =
    existing.lines().any(|l| l.trim() == "* text=auto eol=lf");
  let mut modified = false;

  if !has_text_attr {
    if !existing.is_empty() && !existing.ends_with('\n') {
      existing.push('\n');
    }
    existing.push_str("* text=auto eol=lf\n");
    modified = true;
  }

  if modified || !gitattributes_path.exists() {
    std::fs::write(&gitattributes_path, existing).map_err(|source| {
      Error::FileWrite {
        path: gitattributes_path,
        source,
      }
    })?;
    return Ok(true);
  }
  Ok(false)
}

/// Scaffolds `.github/workflows/ci.yml` and `.github/workflows/release.yml`
/// with provenance headers.
///
/// # Errors
/// Returns [`Error`] if directory creation or file writing fails.
pub fn scaffold_workflows(
  dir: &Path,
  force: bool,
) -> Result<Vec<PathBuf>, Error> {
  let workflows_dir = dir.join(".github").join("workflows");
  std::fs::create_dir_all(&workflows_dir)?;

  let version = env!("CARGO_PKG_VERSION");
  let mut created = Vec::new();

  let ci_path = workflows_dir.join("ci.yml");
  if !ci_path.exists() || force {
    let ci_content = schema::generate_ci_workflow(None);
    let ci_with_header = fs::stamp_provenance_header(&ci_content, version);
    std::fs::write(&ci_path, ci_with_header).map_err(|source| {
      Error::FileWrite {
        path: ci_path.clone(),
        source,
      }
    })?;
    created.push(ci_path);
  }

  let release_path = workflows_dir.join("release.yml");
  if !release_path.exists() || force {
    let release_content = schema::generate_release_workflow(None);
    let release_with_header =
      fs::stamp_provenance_header(&release_content, version);
    std::fs::write(&release_path, release_with_header).map_err(|source| {
      Error::FileWrite {
        path: release_path.clone(),
        source,
      }
    })?;
    created.push(release_path);
  }

  Ok(created)
}

/// Updates `GitHub` Actions workflow files to match the current binary version.
///
/// # Errors
/// Returns [`Error`] if reading or writing workflow files fails.
pub fn update_workflows(
  dir: &Path,
  force: bool,
) -> Result<Vec<WorkflowUpdateOutcome>, Error> {
  let workflows_dir = dir.join(".github").join("workflows");
  std::fs::create_dir_all(&workflows_dir)?;

  let version = env!("CARGO_PKG_VERSION");
  let workflow_specs: [(&str, String); 2] = [
    ("ci.yml", schema::generate_ci_workflow(None)),
    ("release.yml", schema::generate_release_workflow(None)),
  ];

  let mut outcomes = Vec::new();

  for (filename, raw_template) in workflow_specs {
    let path = workflows_dir.join(filename);
    let new_with_header = fs::stamp_provenance_header(&raw_template, version);

    if !path.exists() {
      std::fs::write(&path, &new_with_header).map_err(|source| {
        Error::FileWrite {
          path: path.clone(),
          source,
        }
      })?;
      outcomes.push(WorkflowUpdateOutcome {
        path,
        status: WorkflowUpdateStatus::Created,
      });
      continue;
    }

    let existing =
      std::fs::read_to_string(&path).map_err(|source| Error::FileRead {
        path: path.clone(),
        source,
      })?;

    let is_clean = match fs::extract_provenance_and_body(&existing) {
      Some((recorded_hash, body)) => {
        let actual_hash = fs::sha256_hex(body.as_bytes());
        let actual_hash_normalized =
          fs::sha256_hex(body.replace("\r\n", "\n").as_bytes());
        actual_hash.eq_ignore_ascii_case(recorded_hash)
          || actual_hash_normalized.eq_ignore_ascii_case(recorded_hash)
      }
      None => false,
    };

    if is_clean || force {
      std::fs::write(&path, &new_with_header).map_err(|source| {
        Error::FileWrite {
          path: path.clone(),
          source,
        }
      })?;
      outcomes.push(WorkflowUpdateOutcome {
        path,
        status: if !is_clean && force {
          WorkflowUpdateStatus::ForceUpdated
        } else {
          WorkflowUpdateStatus::Updated
        },
      });
    } else {
      outcomes.push(WorkflowUpdateOutcome {
        path,
        status: WorkflowUpdateStatus::SkippedLocalModifications,
      });
    }
  }

  Ok(outcomes)
}

/// Inspects `.github/workflows/` for pinned `version: "<VER>"` values that
/// differ from `local_version`.
#[must_use]
pub fn check_workflow_version_skew(
  repo_dir: &Path,
  local_version: &str,
) -> Vec<String> {
  let workflows_dir = repo_dir.join(".github").join("workflows");
  if !workflows_dir.is_dir() {
    return Vec::new();
  }

  let Ok(entries) = std::fs::read_dir(&workflows_dir) else {
    return Vec::new();
  };

  let mut skew_versions = std::collections::BTreeSet::new();

  for entry in entries.flatten() {
    let path = entry.path();
    if let Some(ext) = path.extension() {
      if ext == "yml" || ext == "yaml" {
        if let Ok(content) = std::fs::read_to_string(&path) {
          for line in content.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("version:") {
              let val = rest.trim().trim_matches('"').trim_matches('\'').trim();
              if !val.is_empty()
                && val != "RSMK_VERSION"
                && val != local_version
              {
                skew_versions.insert(val.to_string());
              }
            }
          }
        }
      }
    }
  }

  skew_versions.into_iter().collect()
}

/// Initializes a new résumé workspace.
///
/// # Errors
/// Returns [`Error`] if target file already exists without `force`, target
/// directory is not empty without `force`, or file writing fails.
pub fn init(opts: &InitOptions<'_>) -> Result<InitOutcome, Error> {
  let base_dir = opts
    .output
    .parent()
    .filter(|p| !p.as_os_str().is_empty())
    .unwrap_or(Path::new("."));

  if opts.output.exists() && !opts.force {
    return Err(Error::DestinationAlreadyExists {
      path: opts.output.to_path_buf(),
    });
  }

  if !opts.force && dir_has_entries(base_dir) {
    return Err(Error::DestinationNotEmpty {
      path: base_dir.to_path_buf(),
    });
  }

  let template_content = schema::generate_init_template(opts.name);

  if !base_dir.exists() {
    std::fs::create_dir_all(base_dir)?;
  }

  std::fs::write(opts.output, template_content).map_err(|source| {
    Error::FileWrite {
      path: opts.output.to_path_buf(),
      source,
    }
  })?;

  let mut outcome = InitOutcome {
    content_path: opts.output.to_path_buf(),
    created_files: vec![opts.output.to_path_buf()],
    git_initialized: false,
    workflows_created: false,
  };

  if !opts.no_git {
    if !git::is_inside_work_tree(base_dir) && git::init_repo(base_dir).is_ok() {
      outcome.git_initialized = true;
    }
    if ensure_gitignore(base_dir)? {
      outcome.created_files.push(base_dir.join(".gitignore"));
    }
    if ensure_gitattributes(base_dir)? {
      outcome.created_files.push(base_dir.join(".gitattributes"));
    }
  }

  if !opts.no_git && !opts.no_workflows {
    let workflow_files = scaffold_workflows(base_dir, opts.force)?;
    outcome.created_files.extend(workflow_files);
    outcome.workflows_created = true;
  }

  Ok(outcome)
}

#[cfg(test)]
mod tests {
  use tempfile::TempDir;

  use super::*;

  #[test]
  fn test_looks_like_file() {
    assert!(looks_like_file(Path::new("content.yaml")));
    assert!(looks_like_file(Path::new("my_resume.yml")));
    assert!(!looks_like_file(Path::new("subfolder")));
  }

  #[test]
  fn test_resolve_init_output() {
    assert_eq!(resolve_init_output(None), PathBuf::from("content.yaml"));
    assert_eq!(
      resolve_init_output(Some(Path::new("custom.yaml"))),
      PathBuf::from("custom.yaml")
    );
    assert_eq!(
      resolve_init_output(Some(Path::new("my_dir"))),
      PathBuf::from("my_dir").join("content.yaml")
    );
  }

  #[test]
  fn test_init_scaffold_lifecycle() {
    let temp = TempDir::new().unwrap();
    let content_path = temp.path().join("content.yaml");

    let opts = InitOptions {
      name: "Alice Vance",
      output: &content_path,
      force: false,
      no_git: true,
      no_workflows: true,
    };

    let outcome = init(&opts).unwrap();
    assert_eq!(outcome.content_path, content_path);
    assert!(content_path.exists());
    let text = std::fs::read_to_string(&content_path).unwrap();
    assert!(text.contains("Alice Vance"));

    // Collision rejection without force
    let err = init(&opts).unwrap_err();
    assert!(matches!(err, Error::DestinationAlreadyExists { .. }));

    // Overwrite succeeds with force
    let force_opts = InitOptions {
      force: true,
      ..opts
    };
    assert!(init(&force_opts).is_ok());
  }
}
