//! `rsmk update` — replace the running binary in place via axoupdater.

use crate::utils::ui;
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

/// How the install receipt relates to the running binary.
#[derive(Debug, PartialEq, Eq)]
enum Receipt {
  /// A receipt exists and describes this executable.
  Matches,
  /// No receipt could be loaded.
  Missing,
  /// A receipt exists but describes a different install location.
  Stale,
}

/// Builds an updater for the running binary.
///
/// axoupdater only trusts a receipt for the executable it describes, and
/// when they disagree it answers "no update needed" without querying
/// GitHub at all. A receipt left behind by another install (or written by
/// a build under `target/`) would then pin `rsmk update` to "up to date"
/// forever, so a stale receipt is treated like a missing one: the release
/// source, install directory and version come from this binary instead.
fn build_updater() -> Result<(AxoUpdater, Receipt), UpdateError> {
  let mut updater = AxoUpdater::new_for("resumake");
  let receipt = if updater.load_receipt().is_err() {
    Receipt::Missing
  } else if updater.check_receipt_is_for_this_executable()? {
    return Ok((updater, Receipt::Matches));
  } else {
    Receipt::Stale
  };

  let mut updater = AxoUpdater::new_for("resumake");
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
  Ok((updater, receipt))
}

/// Explains failures a user can act on. A permission error almost always
/// means the binary sits in an admin-only directory (e.g. an MSI install
/// under Program Files), which a per-user self-update cannot write to.
fn failure_hint(err: &UpdateError) -> Option<String> {
  let io = match err {
    UpdateError::Io(io) => io,
    UpdateError::AxoUpdater(e) => match e.as_ref() {
      axoupdater::AxoupdateError::Io(io) => io,
      _ => return None,
    },
  };
  if io.kind() != std::io::ErrorKind::PermissionDenied {
    return None;
  }
  let exe = std::env::current_exe()
    .map(|p| p.display().to_string())
    .unwrap_or_else(|_| "the rsmk binary".to_string());
  Some(format!(
    "No permission to replace {exe}. If it is in an admin-only folder \
     (such as Program Files), uninstall that copy and reinstall per-user \
     with the installer from the README, or rerun from an elevated shell."
  ))
}

/// Entry point for the `update` subcommand.
///
/// # Errors
///
/// Returns an [`UpdateError`] if update checks or downloads fail.
pub(crate) fn run_update(
  check: bool,
  force: bool,
  quiet: bool,
) -> Result<(), UpdateError> {
  let (mut updater, receipt) = build_updater()?;
  if receipt == Receipt::Stale && !quiet {
    ui::print_info(
      "Ignoring an install receipt that belongs to a different rsmk \
       install; checking GitHub releases for this binary directly.",
    );
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
        let err = UpdateError::from(e);
        if let Some(hint) = failure_hint(&err) {
          ui::print_info(&hint);
        }
      }
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
  fn test_failure_hint_explains_permission_denied() {
    let denied =
      || std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
    let wrapped = UpdateError::AxoUpdater(Box::new(
      axoupdater::AxoupdateError::Io(denied()),
    ));
    let hint = failure_hint(&wrapped).expect("permission errors get a hint");
    assert!(hint.contains("No permission to replace"));
    assert!(failure_hint(&UpdateError::Io(denied())).is_some());
  }

  #[test]
  fn test_failure_hint_ignores_other_errors() {
    let missing =
      UpdateError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "x"));
    assert!(failure_hint(&missing).is_none());
  }

  #[test]
  fn test_run_update_check_quiet() {
    // In check mode with quiet = true, run_update succeeds without printing
    let res = run_update(true, false, true);
    assert!(res.is_ok());
  }

  #[test]
  fn test_run_update_check_force_quiet() {
    // In check mode with force = true and quiet = true, run_update succeeds gracefully
    let res = run_update(true, true, true);
    assert!(res.is_ok());
  }
}
