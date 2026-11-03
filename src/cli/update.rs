//! `rsmk update`: replace the running binary with the latest release.
//!
//! Owns checking that the binary can be replaced where it is and driving
//! `axoupdater`, which runs the release's own installer into the binary's
//! directory.

use std::env;
use std::ffi;
use std::fs;
use std::io;
use std::path;

use axoupdater;
use clap;
use thiserror;

use crate::cli;

/// The cargo-dist app name, which names the binary and installers.
const APP: &str = "rsmk";

/// GitHub token for lifting anonymous API rate limits.
const TOKEN: &str = "RSMK_GITHUB_TOKEN";

/// Why `rsmk update` stopped.
#[derive(Debug, thiserror::Error)]
pub enum Error {
  /// Binary name does not match expected app name.
  #[error(
    "{} is not named {APP}{}, so the installer would not replace it",
    path.display(),
    env::consts::EXE_SUFFIX
  )]
  NotNamedRsmk {
    /// The running binary.
    path: path::PathBuf,
  },
  /// The binary's directory is not writable.
  #[error("cannot write to {}: {source}", dir.display())]
  Unwritable {
    /// Target directory.
    dir: path::PathBuf,
    /// Underlying error.
    source: io::Error,
  },
  /// Failed to locate running executable.
  #[error("cannot locate the running binary: {0}")]
  Exe(io::Error),
  /// Checking or installing update failed.
  #[error(transparent)]
  Update(#[from] Box<axoupdater::AxoupdateError>),
}

/// Available CLI arguments for `rsmk update`.
#[derive(clap::Args)]
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
  /// Returns [`cli::Error`] if checking or installing the release fails.
  #[expect(clippy::print_stdout, reason = "CLI output")]
  pub fn run(self, quiet: bool) -> Result<(), cli::Error> {
    let current = env!("CARGO_PKG_VERSION");
    let exe = running_exe().map_err(Error::Exe)?;
    let dir = exe.parent().unwrap_or_else(|| path::Path::new("."));
    let dir_utf8 = dir.to_str().ok_or_else(|| {
      Error::Exe(io::Error::new(
        io::ErrorKind::InvalidData,
        "path is not UTF-8",
      ))
    })?;

    let mut updater = axoupdater::AxoUpdater::new_for(APP);
    if let Ok(token) = env::var(TOKEN) {
      updater.set_github_token(&token);
    }

    let parsed_version = axoupdater::Version::parse(current).map_err(|e| {
      Error::Exe(io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
    })?;

    updater
      .set_release_source(axoupdater::ReleaseSource {
        release_type: axoupdater::ReleaseSourceType::GitHub,
        owner: "arvinduh".to_string(),
        name: "resumake".to_string(),
        app_name: APP.to_string(),
      })
      .set_install_dir(dir_utf8)
      .set_current_version(parsed_version)
      .map_err(Box::new)
      .map_err(Error::from)?;

    if self.check {
      let needed = updater
        .is_update_needed_sync()
        .map_err(Box::new)
        .map_err(Error::from)?;
      if needed {
        println!("a newer release of rsmk is available (current: {current})");
      } else if !quiet {
        println!("rsmk {current} is the latest release");
      }
      return Ok(());
    }

    if !self.force
      && !updater
        .is_update_needed_sync()
        .map_err(Box::new)
        .map_err(Error::from)?
    {
      if !quiet {
        println!("rsmk {current} is already up to date");
      }
      return Ok(());
    }

    check_replaceable(&exe, dir)?;
    let result = updater.run_sync().map_err(Box::new).map_err(Error::from)?;
    let latest =
      result.map_or_else(|| current.to_string(), |r| r.new_version.to_string());

    if !quiet {
      println!("updated rsmk from {current} to {latest}");
    }

    Ok(())
  }
}

/// The running binary's path.
fn running_exe() -> io::Result<path::PathBuf> {
  let exe = env::current_exe()?;
  if cfg!(windows) {
    Ok(exe)
  } else {
    exe.canonicalize()
  }
}

/// Verifies `exe` is named `rsmk` and `dir` is writable.
fn check_replaceable(exe: &path::Path, dir: &path::Path) -> Result<(), Error> {
  let name = format!("{APP}{}", env::consts::EXE_SUFFIX);
  let named =
    exe
      .file_name()
      .and_then(ffi::OsStr::to_str)
      .is_some_and(|actual| {
        actual == name || (cfg!(windows) && actual.eq_ignore_ascii_case(&name))
      });

  if !named {
    return Err(Error::NotNamedRsmk {
      path: exe.to_path_buf(),
    });
  }

  let probe = dir.join(".rsmk-write-test");
  fs::write(&probe, b"")
    .and_then(|()| fs::remove_file(&probe))
    .map_err(|source| Error::Unwritable {
      dir: dir.to_path_buf(),
      source,
    })?;

  Ok(())
}
