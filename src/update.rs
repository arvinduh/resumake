//! `rsmk update` — replace the running binary in place via axoupdater.

use axoupdater::AxoUpdater;

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

/// Entry point for the `update` subcommand.
///
/// # Errors
///
/// Returns an [`UpdateError`] if update checks or downloads fail.
pub fn run_update(
  check: bool,
  _force: bool,
  quiet: bool,
) -> Result<(), UpdateError> {
  let mut updater = AxoUpdater::new_for("resumake");
  updater.load_receipt()?;
  if check {
    if updater.is_update_needed_sync()? {
      say(
        quiet,
        "A newer rsmk is available. Run `rsmk update` to upgrade.",
      );
    } else {
      say(quiet, "rsmk is up to date");
    }
    return Ok(());
  }

  if let Some(_res) = updater.run_sync()? {
    say(quiet, "rsmk was updated successfully!");
  } else {
    say(quiet, "rsmk is already up to date.");
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
}
