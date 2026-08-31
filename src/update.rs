//! `rsmk update` — replace the running binary in place from GitHub Releases.
//!
//! The heavy lifting (release discovery, download, `.sha256` verification,
//! archive extraction, and the atomic self-replace) is delegated entirely to
//! the [`self_update`] crate. Only the pure decision logic lives here so it can
//! be unit-tested without touching the network.

use self_update::backends::github;
use self_update::version::cmp_versions;
use std::cmp::Ordering;

/// GitHub repository owner that publishes `rsmk` releases.
const REPO_OWNER: &str = "arvinduh";
/// GitHub repository name that publishes `rsmk` releases.
const REPO_NAME: &str = "resumake";
/// Name of the executable inside each release archive.
const BIN_NAME: &str = "rsmk";
/// Sentinel version handed to `self_update` to force a same-version reinstall.
const FORCE_SENTINEL_VERSION: &str = "0.0.0";

/// What `rsmk update` should do given the current and latest release versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateAction {
  /// The running binary already matches (or is newer than) the latest release.
  UpToDate,
  /// A newer release exists and should be installed.
  Update,
  /// Already current, but `--force` asks for a reinstall anyway.
  ForcedReinstall,
}

/// Map a release target triple to the asset published for it, or `None` when no
/// prebuilt `rsmk` binary exists for that platform.
///
/// Keyed on the four triples the release workflow actually builds; call
/// [`normalize_host_target`] first to fold ABI-compatible host triples onto one
/// of these.
pub fn asset_name_for_target(target: &str) -> Option<&'static str> {
  match target {
    "x86_64-unknown-linux-gnu" => {
      Some("resumake-x86_64-unknown-linux-gnu.tar.gz")
    }
    "aarch64-apple-darwin" => Some("resumake-aarch64-apple-darwin.tar.gz"),
    "x86_64-apple-darwin" => Some("resumake-x86_64-apple-darwin.tar.gz"),
    "x86_64-pc-windows-msvc" => Some("resumake-x86_64-pc-windows-msvc.zip"),
    _ => None,
  }
}

/// Fold the running binary's target triple onto the triple whose release asset
/// can replace it. On x86-64 Windows the `-gnu` and `-gnullvm` toolchains share
/// the MSVC ABI, so the published `-msvc` archive runs there unchanged;
/// everything else maps to itself.
///
/// Consequence: a user who self-built with the `x86_64-pc-windows-gnu`
/// toolchain is moved onto the published `-msvc` `.zip` build the first
/// time they run `rsmk update`. `rsmk` links no C runtime dependencies
/// dynamically, so the swap is harmless, and intentional: there is no
/// `-gnu` release asset to update from.
pub fn normalize_host_target(host: &str) -> &str {
  match host {
    "x86_64-pc-windows-gnu" | "x86_64-pc-windows-gnullvm" => {
      "x86_64-pc-windows-msvc"
    }
    other => other,
  }
}

/// Decide what to do from a comparison of the current and latest semver
/// strings. Pure and network-free so the matrix can be unit-tested.
///
/// A genuine upgrade (`current < latest`) always wins over the `force` flag;
/// `force` only upgrades the "nothing to do" outcome into a reinstall.
pub fn update_action(
  current: &str,
  latest: &str,
  force: bool,
) -> Result<UpdateAction, String> {
  let ordering = cmp_versions(current, latest).map_err(|e| e.to_string())?;
  Ok(match ordering {
    Ordering::Less => UpdateAction::Update,
    _ if force => UpdateAction::ForcedReinstall,
    _ => UpdateAction::UpToDate,
  })
}

/// Print `msg` unless `quiet` is set.
fn say(quiet: bool, msg: &str) {
  if !quiet {
    println!("{msg}");
  }
}

/// Build a configured `self_update` GitHub updater for the current platform.
fn build_updater(
  current_version: &str,
  target: &str,
  asset: &str,
  quiet: bool,
) -> Result<github::Update, String> {
  github::Update::configure()
    .repo_owner(REPO_OWNER)
    .repo_name(REPO_NAME)
    .bin_name(BIN_NAME)
    .target(target)
    .current_version(current_version)
    // Release assets ship a `<asset>.sha256` sidecar; have self_update fetch
    // and enforce it before replacing the binary.
    .checksum_from_asset(format!("{asset}.sha256"))
    .show_output(!quiet)
    .show_download_progress(!quiet)
    // Never block on stdin — this may run unattended.
    .no_confirm(true)
    .build()
    .map_err(|e| e.to_string())
}

/// Entry point for the `update` subcommand.
pub fn run_update(check: bool, force: bool, quiet: bool) -> Result<(), String> {
  let current = env!("CARGO_PKG_VERSION");
  let target = normalize_host_target(self_update::get_target());
  let asset = asset_name_for_target(target).ok_or_else(|| {
    format!("no prebuilt rsmk binary for {target}; build from source")
  })?;

  let updater = build_updater(current, target, asset, quiet)?;
  let latest_releases =
    updater.get_latest_release().map_err(|e| e.to_string())?;
  let latest_release = latest_releases
    .latest()
    .ok_or_else(|| "no rsmk releases published yet".to_string())?;
  let latest = latest_release.version().trim_start_matches('v');

  let action = update_action(current, latest, force)?;

  if check {
    match action {
      UpdateAction::Update => say(
        quiet,
        &format!(
          "A newer rsmk is available: v{latest} (current v{current}). \
           Run `rsmk update` to upgrade."
        ),
      ),
      UpdateAction::UpToDate | UpdateAction::ForcedReinstall => {
        say(quiet, &format!("rsmk is up to date (v{current})"));
      }
    }
    return Ok(());
  }

  match action {
    UpdateAction::UpToDate => {
      say(quiet, &format!("rsmk is already up to date (v{current})"));
      Ok(())
    }
    UpdateAction::Update => {
      updater.update().map_err(|e| e.to_string())?;
      say(quiet, &format!("Updated rsmk v{current} -> v{latest}"));
      Ok(())
    }
    UpdateAction::ForcedReinstall => {
      // self_update skips the replace when versions match, so rebuild with a
      // sentinel current version to force it to reinstall the latest release.
      let forced = build_updater(FORCE_SENTINEL_VERSION, target, asset, quiet)?;
      forced.update().map_err(|e| e.to_string())?;
      say(quiet, &format!("Updated rsmk v{current} -> v{latest}"));
      Ok(())
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn asset_name_covers_every_release_target() {
    assert_eq!(
      asset_name_for_target("x86_64-unknown-linux-gnu"),
      Some("resumake-x86_64-unknown-linux-gnu.tar.gz")
    );
    assert_eq!(
      asset_name_for_target("aarch64-apple-darwin"),
      Some("resumake-aarch64-apple-darwin.tar.gz")
    );
    assert_eq!(
      asset_name_for_target("x86_64-apple-darwin"),
      Some("resumake-x86_64-apple-darwin.tar.gz")
    );
    assert_eq!(
      asset_name_for_target("x86_64-pc-windows-msvc"),
      Some("resumake-x86_64-pc-windows-msvc.zip")
    );
  }

  #[test]
  fn asset_name_is_none_for_unsupported_target() {
    assert_eq!(asset_name_for_target("riscv64gc-unknown-linux-gnu"), None);
    assert_eq!(asset_name_for_target("aarch64-pc-windows-msvc"), None);
  }

  #[test]
  fn windows_gnu_hosts_fold_onto_the_msvc_asset() {
    for host in [
      "x86_64-pc-windows-gnu",
      "x86_64-pc-windows-gnullvm",
      "x86_64-pc-windows-msvc",
    ] {
      assert_eq!(
        asset_name_for_target(normalize_host_target(host)),
        Some("resumake-x86_64-pc-windows-msvc.zip")
      );
    }
    assert_eq!(
      normalize_host_target("x86_64-unknown-linux-gnu"),
      "x86_64-unknown-linux-gnu"
    );
  }

  #[test]
  fn update_action_installs_when_newer_exists() {
    assert_eq!(
      update_action("0.1.0", "0.2.0", false).unwrap(),
      UpdateAction::Update
    );
    // A real upgrade takes priority over the force label.
    assert_eq!(
      update_action("0.1.0", "0.2.0", true).unwrap(),
      UpdateAction::Update
    );
  }

  #[test]
  fn update_action_is_up_to_date_when_equal() {
    assert_eq!(
      update_action("0.2.0", "0.2.0", false).unwrap(),
      UpdateAction::UpToDate
    );
    assert_eq!(
      update_action("0.2.0", "0.2.0", true).unwrap(),
      UpdateAction::ForcedReinstall
    );
  }

  #[test]
  fn update_action_is_up_to_date_when_current_is_newer() {
    assert_eq!(
      update_action("0.3.0", "0.2.0", false).unwrap(),
      UpdateAction::UpToDate
    );
    assert_eq!(
      update_action("0.3.0", "0.2.0", true).unwrap(),
      UpdateAction::ForcedReinstall
    );
  }

  #[test]
  fn update_action_rejects_garbage_versions() {
    assert!(update_action("not-a-version", "0.2.0", false).is_err());
    assert!(update_action("0.2.0", "not-a-version", false).is_err());
  }
}
