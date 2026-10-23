//! Handlers for `rsmk release`.

use std::path::{Path, PathBuf};

use colored::Colorize;
use resumake::utils::git;
use resumake::{engine, release, schema, workspace};

use crate::cli::Error;

/// Available CLI arguments for `rsmk release`.
#[derive(Debug, Clone, PartialEq, Eq, clap::Args)]
pub struct ReleaseArgs {
  /// Path to content YAML file
  #[arg(value_name = "CONTENT", default_value = "content.yaml")]
  pub content: PathBuf,

  /// Optional release message for the annotated git tag
  #[arg(short, long)]
  pub message: Option<String>,

  /// Dry-run mode: run all pre-flight checks without creating or pushing a git
  /// tag
  #[arg(long)]
  pub dry_run: bool,

  /// Skip compilation and telemetry pre-flight check (rsmk check)
  #[arg(long)]
  pub skip_build: bool,
}

impl ReleaseArgs {
  /// Runs the release command.
  ///
  /// # Errors
  ///
  /// Returns an [`Error`] if pre-flight checks, tag creation, or tag push
  /// fails.
  pub fn run(self, quiet: bool) -> Result<(), Error> {
    let repo_dir = if self.content.is_file() {
      self.content.parent().unwrap_or(Path::new("."))
    } else {
      Path::new(".")
    };
    let repo_dir = if repo_dir.as_os_str().is_empty() {
      Path::new(".")
    } else {
      repo_dir
    };

    let raw_version = schema::load_content_version(&self.content)?;
    let target_ver = release::parse_version(&raw_version)?;

    let skews = workspace::check_workflow_version_skew(
      repo_dir,
      env!("CARGO_PKG_VERSION"),
    );
    for pinned in skews {
      eprintln!(
        "warning: Repository workflows pin rsmk {pinned}, but your local \
         binary is {}.\n         Layout telemetry may measure geometry \
         differently in CI.\n         Run `rsmk init --workflows` to \
         synchronize your workflow pin.",
        env!("CARGO_PKG_VERSION")
      );
    }

    if !quiet {
      println!("Résumé Release v{target_ver}\n");
    }

    git::check_working_tree_clean(repo_dir)?;
    if !quiet {
      println!("  {} working tree clean", "✓".green());
    }

    git::check_upstream_synced(repo_dir)?;
    if !quiet {
      println!(
        "  {} upstream branch synced (nothing unpushed)",
        "✓".green()
      );
    }

    let latest_tag = release::check_semver_monotonicity(&target_ver, repo_dir)?;
    if !quiet {
      if let Some(prev) = latest_tag {
        println!("  {} v{target_ver} is new, ahead of v{prev}", "✓".green());
      } else {
        println!("  {} v{target_ver} is new", "✓".green());
      }
    }

    if !self.skip_build {
      engine::verify_content(
        &self.content,
        engine::templates::DEFAULT_TEMPLATE,
        None,
        None,
      )?;
      if !quiet {
        println!("  {} pre-flight check passed (rsmk check)", "✓".green());
      }
    } else if !quiet {
      println!("  {} pre-flight check skipped (--skip-build)", "✓".green());
    }

    if self.dry_run {
      return Ok(());
    }

    let tag_name =
      release::execute_release(repo_dir, &target_ver, self.message.as_deref())?;

    if !quiet {
      println!("\n  {} created tag {tag_name}", "✓".green());
      println!("  {} pushed tag to origin", "✓".green());

      let remote_url = git::get_remote_origin_url(repo_dir)
        .unwrap_or_else(|_| "https://github.com/arvinduh/resumake".to_string());
      let actions_url = release::derive_actions_url(&remote_url);
      println!("    Release workflow triggered: {actions_url}");
    }

    Ok(())
  }
}
