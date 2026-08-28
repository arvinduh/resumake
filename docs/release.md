# Release Procedure

This document describes how a versioned release of `resumake` is cut and
published.

---

## Overview

Releases are cut from `main` and are driven by
[Conventional Commits](https://www.conventionalcommits.org/). Every commit
merged to `main` should follow the `<type>(<scope>): <description>` format
already used throughout this repository's history (see `git log`). This lets
[git-cliff](https://git-cliff.org/) derive a structured changelog directly from
commit messages.

---

## Prerequisites

- [`git-cliff`](https://git-cliff.org/) installed locally for previewing
  changelog output:

  ```sh
  cargo install git-cliff --locked
  ```

  _(CI does not require a local install — the release workflow runs it via the
  `orhun/git-cliff-action` GitHub Action.)_

- Push access to `main` and permission to push git tags (`v*`).

---

## Step-by-Step Procedure

### 1. Confirm `main` is Releasable

Ensure all formatters, linters, tests, and documentation builds pass:

```sh
fml fmt --check
fml lint
cargo test --all-targets
cargo doc --no-deps
```

### 2. Bump the Version

Update `version` in `Cargo.toml`. Follow semver based on changes since the last
tag:

- `fix(...)` $\rightarrow$ patch bump (`0.1.0` $\rightarrow$ `0.1.1`)
- `feat(...)` $\rightarrow$ minor bump (`0.1.0` $\rightarrow$ `0.2.0`)
- `feat(...)!` or `BREAKING CHANGE` $\rightarrow$ major bump

### 3. Preview the Changelog

Preview what `git-cliff` will generate for the new version:

```sh
git cliff --unreleased --tag vX.Y.Z
```

### 4. Generate and Commit the Changelog

```sh
git cliff --tag vX.Y.Z --output CHANGELOG.md
git add Cargo.toml CHANGELOG.md
git commit -m "chore(release): bump version to vX.Y.Z"
git push origin main
```

### 5. Tag and Push the Release

```sh
git tag -a vX.Y.Z -m "vX.Y.Z"
git push origin vX.Y.Z
```

Pushing the tag triggers `.github/workflows/release.yml`, which:

1. Generates and publishes release notes from `CHANGELOG.md` via `git-cliff`.
2. Compiles and attaches the canonical `resume.schema.json` matching the new
   version.
3. Compiles optimized standalone binaries across Linux (x86_64), macOS (ARM64 &
   Intel), and Windows (x86_64) with SHA-256 checksums.
