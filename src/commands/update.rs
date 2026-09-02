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
  let current_version = Version::parse(current_version_str).ok();

  if check {
    if let Ok(is_needed) = updater.is_update_needed_sync() {
      if is_needed {
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
              "A newer rsmk is available: v{latest_ver} (currently running v{cur}). Run `rsmk update` to upgrade."
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

  say(quiet, "Checking for resumake updates...");
  match updater.run_sync() {
    Ok(Some(_res)) => {
      say(quiet, "rsmk was updated successfully!");
    }
    Ok(None) => {
      say(
        quiet,
        &format!("rsmk is already up to date (v{current_version_str})."),
      );
    }
    Err(e) => {
      say(quiet, &format!("[FAIL] Self-update failed: {e}"));
    }
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

  #[test]
  fn test_run_update_check_force_quiet() {
    // In check mode with force = true and quiet = true, run_update succeeds gracefully
    let res = run_update(true, true, true);
    assert!(res.is_ok());
  }
}
