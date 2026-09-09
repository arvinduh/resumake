//! Initialization logic for scaffolding new résumé projects, git repositories,
//! and GitHub Actions workflows.

use crate::schema;
use crate::utils::fs;
use crate::utils::git::{self, GitError};
use crate::utils::ui;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

/// Options configuring the workspace initialization process.
#[derive(Debug, Clone)]
pub struct InitOptions<'a> {
  /// Explicit candidate name override.
  pub name: Option<&'a str>,
  /// Destination path for the new content file.
  pub output: &'a Path,
  /// Whether to overwrite existing files.
  pub force: bool,
  /// Whether to skip git repository initialization and git config files.
  pub no_git: bool,
  /// Whether to skip GitHub Actions workflow scaffolding.
  pub no_workflows: bool,
  /// Whether to update GitHub Actions workflow files.
  pub update: bool,
  /// Suppress informational console output.
  pub quiet: bool,
}

/// Returns true when `path` should be treated as an explicit content-file
/// target rather than a directory to scaffold inside.
///
/// An existing directory is never a file; an existing regular file always is.
/// For a path that does not yet exist, only a `.yaml`/`.yml` extension marks it
/// as a file — bare names like `arvin_resume` are treated as directories.
fn looks_like_file(path: &Path) -> bool {
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
    Some("yaml") | Some("yml")
  )
}

/// Resolves the effective content-file path for `rsmk init` from the optional
/// positional destination and the optional `--output` flag (which are mutually
/// exclusive at the CLI layer).
///
/// - Neither given: `content.yaml` in the current directory.
/// - `--output <path>`: used verbatim.
/// - positional `<dest>`: a directory-like value scaffolds `<dest>/content.yaml`;
///   a `*.yaml` value is used verbatim as the file path.
pub fn resolve_init_output(
  dest: Option<&Path>,
  output: Option<&Path>,
) -> PathBuf {
  if let Some(output) = output {
    return output.to_path_buf();
  }
  match dest {
    None => PathBuf::from("content.yaml"),
    Some(dest) if looks_like_file(dest) => dest.to_path_buf(),
    Some(dest) => dest.join("content.yaml"),
  }
}

/// Returns true when `dir` exists and contains at least one entry.
fn dir_has_entries(dir: &Path) -> bool {
  match std::fs::read_dir(dir) {
    Ok(mut entries) => entries.next().is_some(),
    Err(_) => false,
  }
}

/// Resolves candidate name from CLI arguments or prompts interactively.
fn resolve_candidate_name(name_arg: Option<&str>) -> String {
  if let Some(name) = name_arg {
    let trimmed = name.trim();
    if !trimmed.is_empty() {
      return trimmed.to_string();
    }
  }

  if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
    use std::io::{self, Write};
    print!("Candidate name [Jane Doe]: ");
    let _ = io::stdout().flush();
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
      let trimmed = input.trim();
      if !trimmed.is_empty() {
        return trimmed.to_string();
      }
    }
  }

  "Jane Doe".to_string()
}

/// Errors originating from project initialization and workflow management.
#[derive(thiserror::Error, Debug)]
pub enum InitError {
  /// Destination file already exists and `--force` was not specified.
  #[error("File '{}' already exists. Use --force to overwrite.", path.display())]
  DestinationAlreadyExists {
    /// Path to the conflicting file.
    path: PathBuf,
  },
  /// Target directory is not empty and `--force` was not specified.
  #[error("Directory '{}' is not empty. Use --force to scaffold into it anyway.", path.display())]
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
  /// Git or GitHub CLI operation error.
  #[error(transparent)]
  Git(#[from] crate::utils::git::GitError),
  /// Underlying standard I/O error.
  #[error("I/O error: {0}")]
  Io(#[from] std::io::Error),
}

/// Ensures `.gitignore` exists and ignores `*.pdf` and `.resumake/`.
///
/// # Errors
///
/// Returns an [`InitError`] if reading or writing `.gitignore` fails.
fn ensure_gitignore(dir: &Path) -> Result<(), InitError> {
  let gitignore_path = dir.join(".gitignore");
  let mut existing = if gitignore_path.exists() {
    std::fs::read_to_string(&gitignore_path).map_err(|source| {
      InitError::FileRead {
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
      InitError::FileWrite {
        path: gitignore_path,
        source,
      }
    })?;
  }
  Ok(())
}

/// Ensures `.gitattributes` exists and contains `* text=auto eol=lf`.
///
/// # Errors
///
/// Returns an [`InitError`] if reading or writing `.gitattributes` fails.
fn ensure_gitattributes(dir: &Path) -> Result<(), InitError> {
  let gitattributes_path = dir.join(".gitattributes");
  let mut existing = if gitattributes_path.exists() {
    std::fs::read_to_string(&gitattributes_path).map_err(|source| {
      InitError::FileRead {
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
      InitError::FileWrite {
        path: gitattributes_path,
        source,
      }
    })?;
  }
  Ok(())
}

/// Scaffolds `.github/workflows/ci.yml` and `.github/workflows/release.yml` with provenance headers.
///
/// # Errors
///
/// Returns an [`InitError`] if creating directories or writing workflow files fails.
fn scaffold_workflows(dir: &Path, force: bool) -> Result<(), InitError> {
  let workflows_dir = dir.join(".github").join("workflows");
  std::fs::create_dir_all(&workflows_dir)?;

  let version = env!("CARGO_PKG_VERSION");

  let ci_path = workflows_dir.join("ci.yml");
  if !ci_path.exists() || force {
    let ci_content = schema::generate_ci_workflow(None);
    let ci_with_header = fs::stamp_provenance_header(&ci_content, version);
    std::fs::write(&ci_path, ci_with_header).map_err(|source| {
      InitError::FileWrite {
        path: ci_path.clone(),
        source,
      }
    })?;
  }

  let release_path = workflows_dir.join("release.yml");
  if !release_path.exists() || force {
    let release_content = schema::generate_release_workflow(None);
    let release_with_header =
      fs::stamp_provenance_header(&release_content, version);
    std::fs::write(&release_path, release_with_header).map_err(|source| {
      InitError::FileWrite {
        path: release_path,
        source,
      }
    })?;
  }

  Ok(())
}

/// Checks if GitHub CLI `gh` is installed and authenticated.
#[inline]
fn is_gh_authenticated(dir: &Path) -> bool {
  git::is_gh_authenticated(dir)
}

/// Handles interactive GitHub repository creation or prints remote setup guidance.
fn handle_github_remote(dir: &Path, quiet: bool) {
  if quiet {
    return;
  }

  let interactive =
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal();

  if interactive && is_gh_authenticated(dir) {
    use std::io::{self, Write};
    print!("\nCreate a GitHub repository and push? [y/N]: ");
    let _ = io::stdout().flush();
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
      let trimmed = input.trim().to_lowercase();
      if trimmed == "y" || trimmed == "yes" {
        match git::create_repo_and_push(dir) {
          Ok(()) => {
            ui::print_success(
              "GitHub repository created and pushed successfully.",
            );
            return;
          }
          Err(GitError::Spawn(e)) => {
            ui::print_error(&format!("Error running gh CLI: {e}"));
          }
          Err(_) => {
            ui::print_error("Failed to create GitHub repository via gh CLI.");
          }
        }
      }
    }
  }

  println!("\nNext steps to publish your résumé repository:");
  println!("  1. Track and commit your changes:");
  println!("     git add .");
  println!("     git commit -m \"feat: initial résumé scaffold\"");
  println!("  2. Push to GitHub:");
  println!("     gh repo create --source=. --push");
  println!("     # Or configure remote manually:");
  println!("     git remote add origin git@github.com:<username>/<repo>.git");
  println!("     git branch -M main");
  println!("     git push -u origin main");
}

/// Updates GitHub Actions workflow files to match the current binary version.
///
/// If a workflow file contains local modifications (SHA-256 mismatch with provenance header),
/// the update is skipped and a warning with a unified diff is displayed, unless `force` is true.
///
/// # Errors
///
/// Returns an [`InitError`] if creating directories or reading/writing workflow files fails.
fn update_workflows(
  dir: &Path,
  force: bool,
  quiet: bool,
) -> Result<(), InitError> {
  let workflows_dir = dir.join(".github").join("workflows");
  std::fs::create_dir_all(&workflows_dir)?;

  let version = env!("CARGO_PKG_VERSION");
  let workflow_specs: [(&str, String); 2] = [
    ("ci.yml", schema::generate_ci_workflow(None)),
    ("release.yml", schema::generate_release_workflow(None)),
  ];

  for (filename, raw_template) in workflow_specs {
    let path = workflows_dir.join(filename);
    let new_with_header = fs::stamp_provenance_header(&raw_template, version);

    if !path.exists() {
      std::fs::write(&path, &new_with_header).map_err(|source| {
        InitError::FileWrite {
          path: path.clone(),
          source,
        }
      })?;
      if !quiet {
        ui::print_success(&format!(
          "Created workflow '{}' pinned to rsmk v{version}",
          path.display()
        ));
      }
      continue;
    }

    let existing =
      std::fs::read_to_string(&path).map_err(|source| InitError::FileRead {
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
        InitError::FileWrite {
          path: path.clone(),
          source,
        }
      })?;
      if !quiet {
        let prefix = if !is_clean && force {
          "Force updated"
        } else {
          "Updated"
        };
        ui::print_success(&format!(
          "{prefix} workflow '{}' to rsmk v{version}",
          path.display()
        ));
      }
    } else {
      if !quiet {
        eprintln!(
          "warning: Workflow '{}' has local modifications; skipping update. Use --force to overwrite.",
          path.display()
        );
        let diff = fs::generate_unified_diff(
          &path.display().to_string(),
          &existing,
          &new_with_header,
        );
        eprintln!("{diff}");
      }
    }
  }

  Ok(())
}

/// Inspects `.github/workflows/` for pinned `version: "<VER>"` values and emits a warning
/// if any pinned version differs from `local_version`.
pub fn check_workflow_version_skew(repo_dir: &Path, local_version: &str) {
  let workflows_dir = repo_dir.join(".github").join("workflows");
  if !workflows_dir.is_dir() {
    return;
  }

  let entries = match std::fs::read_dir(&workflows_dir) {
    Ok(entries) => entries,
    Err(_) => return,
  };

  use std::collections::BTreeSet;
  let mut skew_versions = BTreeSet::new();

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

  for pinned in skew_versions {
    eprintln!(
      "warning: Repository workflows pin rsmk {pinned}, but your local binary is {local_version}.\n         Layout telemetry may measure geometry differently in CI.\n         Run `rsmk init --update` to synchronize your workflow pin."
    );
  }
}

/// Executes the full initialization command.
///
/// # Errors
///
/// Returns an [`InitError`] if destination exists without force, destination directory is not empty,
/// or file writing fails.
pub fn run_init(opts: InitOptions) -> Result<(), InitError> {
  let base_dir = opts
    .output
    .parent()
    .filter(|p| !p.as_os_str().is_empty())
    .unwrap_or(Path::new("."));

  if opts.update {
    return update_workflows(base_dir, opts.force, opts.quiet);
  }

  if opts.output.exists() && !opts.force {
    return Err(InitError::DestinationAlreadyExists {
      path: opts.output.to_path_buf(),
    });
  }

  // Validate the target directory up front so a rejected init leaves the
  // filesystem untouched (no directories created, no files written).
  if !opts.force && dir_has_entries(base_dir) {
    return Err(InitError::DestinationNotEmpty {
      path: base_dir.to_path_buf(),
    });
  }

  let candidate_name = resolve_candidate_name(opts.name);
  let template_content = schema::generate_init_template(&candidate_name);

  if !base_dir.exists() {
    std::fs::create_dir_all(base_dir)?;
  }

  std::fs::write(opts.output, template_content).map_err(|source| {
    InitError::FileWrite {
      path: opts.output.to_path_buf(),
      source,
    }
  })?;
  if !opts.quiet {
    ui::print_success(&format!(
      "Initialized new résumé content scaffold at '{}'",
      opts.output.display()
    ));
  }

  if !opts.no_git {
    if !git::is_inside_work_tree(base_dir) {
      match git::init_repo(base_dir) {
        Ok(()) => {
          if !opts.quiet {
            ui::print_success("Initialized Git repository");
          }
        }
        Err(e) => {
          if !opts.quiet {
            ui::print_info(&format!("Skipping 'git init': {e}"));
          }
        }
      }
    }
    ensure_gitignore(base_dir)?;
    ensure_gitattributes(base_dir)?;
    if !opts.quiet {
      ui::print_success("Created .gitignore and .gitattributes");
    }
  }

  // GitHub Actions workflows only make sense inside a repository, so they are
  // scaffolded only when a git repo is being set up. A workspace created with
  // --no-git can add them later with `rsmk init --update`.
  if opts.no_git {
    if !opts.no_workflows && !opts.quiet {
      ui::print_info(
        "Skipping GitHub Actions workflows (--no-git). Run `rsmk init --update` \
         from the project directory to add CI/Release workflows later.",
      );
    }
  } else if !opts.no_workflows {
    scaffold_workflows(base_dir, opts.force)?;
    if !opts.quiet {
      ui::print_success(
        "Created GitHub Actions workflows in .github/workflows/",
      );
    }
  }

  if !opts.no_git {
    handle_github_remote(base_dir, opts.quiet);
  }

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::TempDir;

  #[test]
  fn test_sha256_hex() {
    assert_eq!(
      fs::sha256_hex(b""),
      "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
      fs::sha256_hex(b"abc"),
      "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
      fs::sha256_hex(b"The quick brown fox jumps over the lazy dog"),
      "d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592"
    );
  }

  #[test]
  fn test_stamp_provenance_header() {
    let content = "name: CI\n";
    let stamped = fs::stamp_provenance_header(content, "0.1.0");
    assert!(stamped.contains("# Generated by rsmk 0.1.0."));
    assert!(stamped.contains("# rsmk:generated sha256="));
    assert!(stamped.ends_with("name: CI\n"));
  }

  #[test]
  fn test_ensure_gitignore_and_attributes() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    ensure_gitignore(dir).unwrap();
    let gitignore = std::fs::read_to_string(dir.join(".gitignore")).unwrap();
    assert!(gitignore.contains("*.pdf"));
    assert!(gitignore.contains(".resumake/"));

    // Running again does not duplicate lines
    ensure_gitignore(dir).unwrap();
    let gitignore2 = std::fs::read_to_string(dir.join(".gitignore")).unwrap();
    assert_eq!(gitignore, gitignore2);

    ensure_gitattributes(dir).unwrap();
    let gitattributes =
      std::fs::read_to_string(dir.join(".gitattributes")).unwrap();
    assert!(gitattributes.contains("* text=auto eol=lf"));

    // Running again does not duplicate
    ensure_gitattributes(dir).unwrap();
    let gitattributes2 =
      std::fs::read_to_string(dir.join(".gitattributes")).unwrap();
    assert_eq!(gitattributes, gitattributes2);
  }

  #[test]
  fn test_scaffold_workflows() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    scaffold_workflows(dir, false).unwrap();
    let ci_path = dir.join(".github").join("workflows").join("ci.yml");
    let release_path =
      dir.join(".github").join("workflows").join("release.yml");

    assert!(ci_path.exists());
    assert!(release_path.exists());

    let ci_content = std::fs::read_to_string(ci_path).unwrap();
    assert!(ci_content.contains("# Generated by rsmk"));
    assert!(ci_content.contains("# rsmk:generated sha256="));
    assert!(ci_content.contains("name: CI"));

    let rel_content = std::fs::read_to_string(release_path).unwrap();
    assert!(rel_content.contains("# Generated by rsmk"));
    assert!(rel_content.contains("# rsmk:generated sha256="));
    assert!(rel_content.contains("name: Release"));
  }

  #[test]
  fn test_extract_provenance_and_body() {
    let content = "name: CI\non: [push]\n";
    let stamped = fs::stamp_provenance_header(content, "0.1.0");
    let (hash, body) = fs::extract_provenance_and_body(&stamped).unwrap();
    assert_eq!(hash, fs::sha256_hex(content.as_bytes()));
    assert_eq!(body, content);

    // Test with CRLF
    let stamped_crlf = stamped.replace('\n', "\r\n");
    let (hash_crlf, body_crlf) =
      fs::extract_provenance_and_body(&stamped_crlf).unwrap();
    assert_eq!(hash_crlf, fs::sha256_hex(content.as_bytes()));
    assert_eq!(body_crlf.replace("\r\n", "\n"), content);

    // Test missing header
    assert_eq!(fs::extract_provenance_and_body("name: CI\n"), None);
  }

  #[test]
  fn test_generate_unified_diff() {
    let old_text = "line1\nline2\nline3\n";
    let new_text = "line1\nline2_modified\nline3\nline4\n";
    let diff = fs::generate_unified_diff("test.yml", old_text, new_text);
    assert!(diff.contains("--- test.yml (current)"));
    assert!(diff.contains("+++ test.yml (target)"));
    assert!(diff.contains("-line2"));
    assert!(diff.contains("+line2_modified"));
    assert!(diff.contains("+line4"));
  }

  #[test]
  fn test_update_workflows_clean_and_modified_and_force() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();
    let workflows_dir = dir.join(".github").join("workflows");
    std::fs::create_dir_all(&workflows_dir).unwrap();

    let ci_path = workflows_dir.join("ci.yml");
    let release_path = workflows_dir.join("release.yml");

    // 1. Scaffold initial workflows with version 0.0.1
    let old_ci_body = schema::generate_ci_workflow(Some("0.0.1"));
    let old_ci_stamped = fs::stamp_provenance_header(&old_ci_body, "0.0.1");
    std::fs::write(&ci_path, old_ci_stamped).unwrap();

    let old_release_body = schema::generate_release_workflow(Some("0.0.1"));
    let old_release_stamped =
      fs::stamp_provenance_header(&old_release_body, "0.0.1");
    std::fs::write(&release_path, old_release_stamped).unwrap();

    // 2. Clean update should update both to current version
    update_workflows(dir, false, true).unwrap();

    let current_version = env!("CARGO_PKG_VERSION");
    let updated_ci = std::fs::read_to_string(&ci_path).unwrap();
    assert!(
      updated_ci.contains(&format!("Generated by rsmk {current_version}"))
    );
    assert!(updated_ci.contains(&format!("version: \"{current_version}\"")));

    let updated_release = std::fs::read_to_string(&release_path).unwrap();
    assert!(
      updated_release.contains(&format!("Generated by rsmk {current_version}"))
    );
    assert!(
      updated_release.contains(&format!("version: \"{current_version}\""))
    );

    // 3. User modifies CI workflow
    let modified_ci =
      format!("{updated_ci}\n      - run: echo 'custom step'\n");
    std::fs::write(&ci_path, &modified_ci).unwrap();

    // Update without force should skip CI and keep user modification
    update_workflows(dir, false, true).unwrap();
    let ci_after_skip = std::fs::read_to_string(&ci_path).unwrap();
    assert_eq!(ci_after_skip, modified_ci);

    // Update with force should overwrite CI
    update_workflows(dir, true, true).unwrap();
    let ci_after_force = std::fs::read_to_string(&ci_path).unwrap();
    assert!(!ci_after_force.contains("custom step"));
    assert!(
      ci_after_force.contains(&format!("Generated by rsmk {current_version}"))
    );
  }

  #[test]
  fn test_run_init_update_does_not_touch_content_yaml() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();
    let content_file = dir.join("content.yaml");
    std::fs::write(&content_file, "custom content").unwrap();

    let workflows_dir = dir.join(".github").join("workflows");
    std::fs::create_dir_all(&workflows_dir).unwrap();
    let ci_path = workflows_dir.join("ci.yml");
    let old_ci = fs::stamp_provenance_header(
      &schema::generate_ci_workflow(Some("0.0.1")),
      "0.0.1",
    );
    std::fs::write(&ci_path, old_ci).unwrap();

    run_init(InitOptions {
      name: None,
      output: &content_file,
      force: false,
      no_git: true,
      no_workflows: false,
      update: true,
      quiet: true,
    })
    .unwrap();

    // Verify content.yaml untouched
    assert_eq!(
      std::fs::read_to_string(&content_file).unwrap(),
      "custom content"
    );

    // Verify CI workflow updated
    let updated_ci = std::fs::read_to_string(&ci_path).unwrap();
    let current_version = env!("CARGO_PKG_VERSION");
    assert!(
      updated_ci.contains(&format!("Generated by rsmk {current_version}"))
    );
  }

  #[test]
  fn test_is_inside_git_repo() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    assert!(!git::is_inside_work_tree(dir));

    git::init_repo(dir).unwrap();
    assert!(git::is_inside_work_tree(dir));

    let subdir = dir.join("sub").join("nested");
    std::fs::create_dir_all(&subdir).unwrap();
    assert!(git::is_inside_work_tree(&subdir));

    let non_existent = dir.join("does_not_exist");
    assert!(!git::is_inside_work_tree(&non_existent));
  }
}
