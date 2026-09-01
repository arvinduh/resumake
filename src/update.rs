//! `rsmk update` — replace the running binary in place via axoupdater.

use axoupdater::{AxoUpdater, ReleaseSource, ReleaseSourceType, Version};

/// Errors originating from the self-update process.
#[derive(thiserror::Error, Debug)]
pub enum UpdateError {
  /// AxoUpdater error.
  #[error("Self-update error: {0}")]
  AxoUpdater(Box<axoupdater::AxoupdateError>),
  /// Underlying I/O error.
  #[error("I/O error: {0}")]
  Io(#[from] std::io::Error),
}

impl From<axoupdater::AxoupdateError> for UpdateError {
  fn from(err: axoupdater::AxoupdateError) -> Self {
    Self::AxoUpdater(Box::new(err))
  }
}

/// Print `msg` unless `quiet` is set.
fn say(quiet: bool, msg: &str) {
  if !quiet {
    println!("{msg}");
  }
}

/// Queries the latest available version on GitHub Releases asynchronously using a single-threaded runtime.
fn query_latest_github_version(
  updater: &mut AxoUpdater,
) -> Result<Option<Version>, UpdateError> {
  let rt = tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()?;
  let latest = rt.block_on(updater.query_new_version())?.cloned();
  Ok(latest)
}

/// Entry point for the `update` subcommand.
///
/// # Errors
///
/// Returns an [`UpdateError`] if update checks or downloads fail.
pub fn run_update(
  check: bool,
  force: bool,
  quiet: bool,
) -> Result<(), UpdateError> {
  let mut updater = AxoUpdater::new_for("resumake");
  let has_receipt = updater.load_receipt().is_ok();

  if !has_receipt {
    updater.set_release_source(ReleaseSource {
      release_type: ReleaseSourceType::GitHub,
      owner: "arvinduh".to_string(),
      name: "resumake".to_string(),
      app_name: "resumake".to_string(),
    });
  }

  if force {
    updater.always_update(true);
  }

  let current_version_str = env!("CARGO_PKG_VERSION");
  let current_version = Version::parse(current_version_str).ok();

  if check {
    if has_receipt {
      if updater.is_update_needed_sync()? {
        say(
          quiet,
          "A newer rsmk is available. Run `rsmk update` to upgrade.",
        );
      } else {
        say(
          quiet,
          &format!("rsmk is up to date (v{current_version_str})."),
        );
      }
    } else if let Ok(Some(latest_ver)) =
      query_latest_github_version(&mut updater)
    {
      if let Some(ref cur) = current_version {
        if &latest_ver > cur {
          say(
            quiet,
            &format!(
              "A newer rsmk is available: v{latest_ver} (currently running v{cur}). Run `rsmk update` or re-run your installer to upgrade."
            ),
          );
        } else {
          say(
            quiet,
            &format!("rsmk is up to date (v{current_version_str})."),
          );
        }
      } else {
        say(
          quiet,
          &format!("Latest available rsmk release is v{latest_ver}."),
        );
      }
    } else {
      say(
        quiet,
        &format!("rsmk is up to date (v{current_version_str})."),
      );
    }
    return Ok(());
  }

  if has_receipt {
    if let Some(_res) = updater.run_sync()? {
      say(quiet, "rsmk was updated successfully!");
    } else {
      say(quiet, "rsmk is already up to date.");
    }
  } else {
    say(
      quiet,
      &format!(
        "No installation receipt found for rsmk (v{current_version_str}).\n\
        In-place binary replacement is supported for installations managed via the 1-line installer script.\n\n\
        To upgrade to the latest version:\n  \
        • Windows (PowerShell):  irm https://github.com/arvinduh/resumake/releases/latest/download/resumake-installer.ps1 | iex\n  \
        • Linux & macOS:        curl --proto '=https' --tlsv1.2 -LsSf https://github.com/arvinduh/resumake/releases/latest/download/resumake-installer.sh | sh\n  \
        • Windows (MSI):        https://github.com/arvinduh/resumake/releases/latest\n  \
        • Cargo:                cargo install --git https://github.com/arvinduh/resumake"
      ),
    );
  }

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_updater_initialization() {
    let updater = AxoUpdater::new_for("resumake");
    drop(updater);
  }

  #[test]
  fn test_update_error_display() {
    let io_err = UpdateError::Io(std::io::Error::new(
      std::io::ErrorKind::NotFound,
      "file not found",
    ));
    assert!(format!("{io_err}").contains("I/O error"));
  }

  #[test]
  fn test_run_update_check_quiet() {
    // In check mode with quiet = true, run_update succeeds without printing
    let res = run_update(true, false, true);
    assert!(res.is_ok());
  }

  #[test]
  fn test_run_update_fallback_quiet() {
    // In update mode without receipt, quiet = true succeeds gracefully
    let res = run_update(false, false, true);
    assert!(res.is_ok());
  }
}
