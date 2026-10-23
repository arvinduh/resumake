//! `rsmk update` — replace the running binary in place via axoupdater.

use axoupdater::{AxoUpdater, ReleaseSource, ReleaseSourceType, Version};

use crate::cli::ui;

/// Errors originating from the self-update process.
#[derive(thiserror::Error, Debug)]
pub enum Error {
  /// `AxoUpdater` error.
  #[error("Self-update error: {0}")]
  AxoUpdater(Box<axoupdater::AxoupdateError>),
  /// Underlying I/O error.
  #[error("I/O error: {0}")]
  Io(#[from] std::io::Error),
}

impl From<axoupdater::AxoupdateError> for Error {
  fn from(err: axoupdater::AxoupdateError) -> Self {
    Self::AxoUpdater(Box::new(err))
  }
}

/// Available CLI arguments for `rsmk update`.
#[derive(Debug, Clone, PartialEq, Eq, clap::Args)]
pub struct UpdateArgs {
  /// Report whether a newer release exists without installing it
  #[arg(long)]
  pub check: bool,

  /// Reinstall even if the current version is already latest
  #[arg(short, long)]
  pub force: bool,
}

impl UpdateArgs {
  /// Runs the update command.
  ///
  /// # Errors
  ///
  /// Returns an [`Error`] if update checks or downloads fail.
  pub fn run(self, quiet: bool) -> Result<(), Error> {
    run_update(self.check, self.force, quiet)
  }
}

/// Entry point for the `update` subcommand logic.
///
/// # Errors
///
/// Returns an [`Error`] if update checks or downloads fail.
pub(crate) fn run_update(
  check: bool,
  force: bool,
  quiet: bool,
) -> Result<(), Error> {
  let mut updater = AxoUpdater::new_for("resumake");
  if updater.load_receipt().is_err() {
    let current_exe = std::env::current_exe()?;
    let install_dir = current_exe
      .parent()
      .unwrap_or_else(|| std::path::Path::new("."));
    updater.set_release_source(ReleaseSource {
      release_type: ReleaseSourceType::GitHub,
      owner: "arvinduh".to_string(),
      name: "resumake".to_string(),
      app_name: "resumake".to_string(),
    });
    updater.set_install_dir(install_dir.to_str().unwrap_or("."));
    if let Ok(ver) = Version::parse(env!("CARGO_PKG_VERSION")) {
      let _ = updater.set_current_version(ver);
    }
  }

  if force {
    updater.always_update(true);
  }

  let current_version_str = env!("CARGO_PKG_VERSION");

  if check {
    match updater.is_update_needed_sync() {
      Ok(true) => {
        if !quiet {
          ui::print_info(
            "A newer rsmk is available. Run `rsmk update` to upgrade.",
          );
        }
      }
      Ok(false) => {
        if !quiet {
          ui::print_success(&format!(
            "rsmk is up to date (v{current_version_str})."
          ));
        }
      }
      Err(e) => {
        if !quiet {
          ui::print_error(&format!("Could not check for updates: {e}"));
        }
      }
    }
    return Ok(());
  }

  if !quiet {
    ui::print_info("Checking for resumake updates...");
  }
  match updater.run_sync() {
    Ok(Some(_res)) => {
      if !quiet {
        ui::print_success("rsmk was updated successfully!");
      }
    }
    Ok(None) => {
      if !quiet {
        ui::print_success(&format!(
          "rsmk is already up to date (v{current_version_str})."
        ));
      }
    }
    Err(e) => {
      if !quiet {
        ui::print_error(&format!("Self-update failed: {e}"));
      }
    }
  }

  Ok(())
}
