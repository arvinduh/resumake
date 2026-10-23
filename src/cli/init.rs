//! Initialization logic for scaffolding new résumé projects, git repositories,
//! and `GitHub` Actions workflows.

use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use resumake::schema;
use resumake::utils::{fs, git};

use crate::cli::ui;

/// Available CLI arguments for `rsmk init`.
#[derive(Debug, Clone, PartialEq, Eq, clap::Args)]
#[expect(clippy::struct_excessive_bools, reason = "CLI argument flags")]
pub struct InitArgs {
  /// Destination for the scaffold: a directory (creates it and writes
  /// `content.yaml` inside) or an explicit `*.yaml` file path. Defaults to
  /// `content.yaml` in the current directory.
  #[arg(value_name = "DEST")]
  pub dest: Option<PathBuf>,

  /// Candidate display name
  #[arg(short, long)]
  pub name: Option<String>,

  /// Overwrite destination file if it already exists
  #[arg(short, long)]
  pub force: bool,

  /// Skip initializing a git repository and git config files (also skips
  /// `GitHub` Actions workflows, which require a repo; add them later with
  /// `rsmk init --workflows`)
  #[arg(long)]
  pub no_git: bool,

  /// Skip generating `GitHub` Actions CI and Release workflows
  #[arg(long)]
  pub no_workflows: bool,

  /// Create or refresh `GitHub` Actions workflows without modifying
  /// content.yaml
  #[arg(long, visible_alias = "update")]
  pub workflows: bool,
}

impl InitArgs {
  /// Runs the init command.
  ///
  /// # Errors
  ///
  /// Returns an [`Error`] if initialization or workflow generation fails.
  pub fn run(self, quiet: bool) -> Result<(), Error> {
    let resolved_output = resolve_init_output(self.dest.as_deref());
    run_init(InitOptions {
      name: self.name.as_deref(),
      output: &resolved_output,
      force: self.force,
      no_git: self.no_git,
      no_workflows: self.no_workflows,
      workflows: self.workflows,
      quiet,
    })
  }
}

/// Options configuring the workspace initialization process.
#[derive(Debug, Clone, Copy)]
#[expect(clippy::struct_excessive_bools, reason = "CLI argument flags")]
pub(crate) struct InitOptions<'a> {
  /// Explicit candidate name override.
  pub name: Option<&'a str>,
  /// Destination path for the new content file.
  pub output: &'a Path,
  /// Whether to overwrite existing files.
  pub force: bool,
  /// Whether to skip git repository initialization and git config files.
  pub no_git: bool,
  /// Whether to skip `GitHub` Actions workflow scaffolding.
  pub no_workflows: bool,
  /// Whether to create or update `GitHub` Actions workflow files.
  pub workflows: bool,
  /// Suppress informational console output.
  pub quiet: bool,
}

/// Returns true when `path` should be treated as an explicit content-file
/// target rather than a directory to scaffold inside.
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
    Some("yaml" | "yml")
  )
}

/// Resolves the effective content-file path for `rsmk init` from the optional
/// positional destination.
pub(crate) fn resolve_init_output(dest: Option<&Path>) -> PathBuf {
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
pub enum Error {
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
  /// Git operation error.
  #[error(transparent)]
  Git(#[from] git::Error),
  /// Underlying standard I/O error.
  #[error("I/O error: {0}")]
  Io(#[from] std::io::Error),
}

/// Ensures `.gitignore` exists and ignores `*.pdf` and `.resumake/`.
fn ensure_gitignore(dir: &Path) -> Result<(), Error> {
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
  }
  Ok(())
}

/// Ensures `.gitattributes` exists and contains `* text=auto eol=lf`.
fn ensure_gitattributes(dir: &Path) -> Result<(), Error> {
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
  }
  Ok(())
}

/// Scaffolds `.github/workflows/ci.yml` and `.github/workflows/release.yml`
/// with provenance headers.
fn scaffold_workflows(dir: &Path, force: bool) -> Result<(), Error> {
  let workflows_dir = dir.join(".github").join("workflows");
  std::fs::create_dir_all(&workflows_dir)?;

  let version = env!("CARGO_PKG_VERSION");

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
  }

  let release_path = workflows_dir.join("release.yml");
  if !release_path.exists() || force {
    let release_content = schema::generate_release_workflow(None);
    let release_with_header =
      fs::stamp_provenance_header(&release_content, version);
    std::fs::write(&release_path, release_with_header).map_err(|source| {
      Error::FileWrite {
        path: release_path,
        source,
      }
    })?;
  }

  Ok(())
}

/// Prints remote setup guidance for newly initialized repositories.
fn print_next_steps(quiet: bool) {
  if quiet {
    return;
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

/// Updates `GitHub` Actions workflow files to match the current binary version.
fn update_workflows(dir: &Path, force: bool, quiet: bool) -> Result<(), Error> {
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
        Error::FileWrite {
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
    } else if !quiet {
      eprintln!(
        "warning: Workflow '{}' has local modifications; skipping update. Use \
         --force to overwrite.",
        path.display()
      );
    }
  }

  Ok(())
}

/// Inspects `.github/workflows/` for pinned `version: "<VER>"` values and emits
/// a warning if any pinned version differs from `local_version`.
pub(crate) fn check_workflow_version_skew(
  repo_dir: &Path,
  local_version: &str,
) {
  let workflows_dir = repo_dir.join(".github").join("workflows");
  if !workflows_dir.is_dir() {
    return;
  }

  let Ok(entries) = std::fs::read_dir(&workflows_dir) else {
    return;
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

  for pinned in skew_versions {
    eprintln!(
      "warning: Repository workflows pin rsmk {pinned}, but your local binary \
       is {local_version}.\n         Layout telemetry may measure geometry \
       differently in CI.\n         Run `rsmk init --workflows` to \
       synchronize your workflow pin."
    );
  }
}

/// Executes the full initialization command.
pub(crate) fn run_init(opts: InitOptions) -> Result<(), Error> {
  let base_dir = opts
    .output
    .parent()
    .filter(|p| !p.as_os_str().is_empty())
    .unwrap_or(Path::new("."));

  if opts.workflows {
    return update_workflows(base_dir, opts.force, opts.quiet);
  }

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

  let candidate_name = resolve_candidate_name(opts.name);
  let template_content = schema::generate_init_template(&candidate_name);

  if !base_dir.exists() {
    std::fs::create_dir_all(base_dir)?;
  }

  std::fs::write(opts.output, template_content).map_err(|source| {
    Error::FileWrite {
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

  if opts.no_git {
    if !opts.no_workflows && !opts.quiet {
      ui::print_info(
        "Skipping GitHub Actions workflows (--no-git). Run `rsmk init \
         --workflows` from the project directory to add CI/Release workflows \
         later.",
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
    print_next_steps(opts.quiet);
  }

  Ok(())
}
