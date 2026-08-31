# Release Procedure

This document describes how a versioned release of `resumake` is cut and
published.

---

## Overview

Releases are cut from `main` and are driven by
[Conventional Commits](https://www.conventionalcommits.org/). GitHub Releases
automatically derives structured release notes directly from merged pull
requests and conventional commit history on tag push.

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

Update `version` in `Cargo.toml` and commit:

```sh
git add Cargo.toml Cargo.lock
git commit -m "chore(release): bump version to vX.Y.Z"
```

### 3. Tag and Push the Binary Release

You can cut the release using `rsmk release` with automated pre-flight checks:

```sh
rsmk release
```

`rsmk release` automatically runs pre-flight verifications before creating or
pushing tags:

1. **Clean working tree**: verifies no uncommitted changes exist.
2. **Upstream sync**: verifies the branch tracks origin with 0 unpushed commits.
3. **Semver monotonicity**: checks `meta.version` is valid and strictly newer
   than existing git tags.
4. **Pre-flight check**: runs `rsmk build --check` in-memory.
5. **Atomic Tag & Push**: creates annotated tag `vX.Y.Z` and pushes to origin
   (bypassed on `--dry-run`).

Alternatively, manual tag and push:

```sh
git tag -a vX.Y.Z -m "vX.Y.Z"
git push origin vX.Y.Z
```

Pushing the `v*` tag triggers `.github/workflows/release.yml` (managed by
`cargo-dist`), which:

1. Plans and orchestrates multi-platform compilation across Linux (x86_64),
   macOS (ARM64 & Intel), and Windows (x86_64).
2. Generates standalone archives, checksums, and clean 1-line installers
   (`resumake-installer.sh`, `resumake-installer.ps1`).
3. Compiles the canonical JSON Schema (`cargo run --example generate-schema`)
   and attaches `resume.schema.json` as a release asset.
4. Creates the GitHub Release with generated release notes, manifests, and
   binaries.

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
2. Attaches the `resume.schema.json` committed at the repo root (kept current by
   the schema drift test) to the release — it checks out the tag and uploads the
   file, with no Rust build.
3. Makes the schema available at the permanent URL:
   `https://github.com/arvinduh/resumake/releases/download/sX.Y/resume.schema.json`
