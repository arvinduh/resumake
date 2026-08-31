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

### 5. Tag and Push the Binary Release

You can cut the release using `rsmk release` with automated pre-flight checks:

```sh
rsmk release
```

`rsmk release` automatically runs pre-flight verifications before creating or pushing tags:
1. **Clean working tree**: verifies no uncommitted changes exist.
2. **Upstream sync**: verifies the branch tracks origin with 0 unpushed commits.
3. **Semver monotonicity**: checks `meta.version` is valid and strictly newer than existing git tags.
4. **Pre-flight check**: runs `rsmk build --check` in-memory.
5. **Atomic Tag & Push**: creates annotated tag `vX.Y.Z` and pushes to origin (bypassed on `--dry-run`).

Alternatively, manual tag and push:

```sh
git tag -a vX.Y.Z -m "vX.Y.Z"
git push origin vX.Y.Z
```

Pushing the `v*` tag triggers `.github/workflows/release.yml`, which:

1. Sets `make_latest: true` so `github.com/arvinduh/resumake/releases/latest`
   always points to the newest CLI binary release for update checkers.
2. Generates and publishes release notes from `CHANGELOG.md` via `git-cliff`.
3. Compiles and attaches the canonical `resume.schema.json` matching the new
   version.
4. Compiles optimized standalone binaries across Linux (x86_64), macOS (ARM64 &
   Intel), and Windows (x86_64) with SHA-256 checksums.

---

## Schema Releases (`s*`)

Schema releases publish JSON schema versions (e.g. `s1.0`, `s1.1`) independently
of CLI binary patches, matching Formality's release model:

### 1. Tag and Push Schema Release

```sh
git tag -a sX.Y -m "Schema sX.Y"
git push origin sX.Y
```

Pushing an `s*` tag triggers `.github/workflows/schema.yml`, which:

1. Sets `make_latest: false` so schema releases **never** displace the binary
   release from `/releases/latest` or confuse update checkers.
2. Generates and attaches `resume.schema.json` to the release.
3. Makes the schema available at the permanent URL:
   `https://github.com/arvinduh/resumake/releases/download/sX.Y/resume.schema.json`
