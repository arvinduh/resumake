# Release Procedure

This document explains how releases work in the Resumake ecosystem, clearly
distinguishing between **developer releases** of the `resumake` compiler binary
itself and **end-user releases** of personal résumé repositories.

---

## 1. Developer Releases (`resumake` CLI & Crate)

Resumake CLI releases are cut directly from `main` and automated via
`cargo-dist` in `.github/workflows/release.yml`.

> [!IMPORTANT] The CLI command `rsmk release` is strictly for **end users** in a
> résumé workspace containing `content.yaml`. Developers cutting a release of
> `resumake` itself must **never** invoke `rsmk release` in the compiler
> repository.

### Step-by-Step Developer Release

1. **Verify Presubmit**: Ensure all linters, formatting checks, and tests pass:

   ```sh
   fml fmt --check
   fml lint
   cargo test --all-targets
   cargo doc --no-deps
   ```

2. **Verify Cargo-Dist Plan**: Confirm that `cargo-dist` builds clean binary
   targets and installers:

   ```sh
   dist plan
   ```

3. **Bump Version**: Update `version` in `Cargo.toml` and synchronize
   `Cargo.lock`:

   ```sh
   git add Cargo.toml Cargo.lock
   git commit -m "chore(release): bump version to vX.Y.Z"
   ```

4. **Merge to `main`**: Open a pull request, wait for all CI checks to turn
   green, and merge to `main`.

5. **Tag and Push**: Create an annotated semver tag on `main` and push to
   GitHub:

   ```sh
   git checkout main
   git pull origin main
   git tag -a vX.Y.Z -m "Release vX.Y.Z"
   git push origin vX.Y.Z
   ```

6. **Automated CI Release**: Pushing the `v*` tag triggers
   `.github/workflows/release.yml`, which:
   - Compiles cross-platform binaries across Linux (x86_64), macOS (ARM64 &
     x86_64), and Windows (x86_64).
   - Generates standalone compressed archives and fast 1-line shell/PowerShell
     installers (`resumake-installer.sh`, `resumake-installer.ps1`).
   - Generates the canonical JSON Schema (`cargo run --example generate-schema`)
     and attaches `resume.schema.json` as a release asset.
   - Publishes the GitHub Release with automated changelog notes derived from
     merged pull requests.

---

## 2. User Résumé Releases (`rsmk release`)

End users who maintain their personal résumé using Resumake (scaffolded via
`rsmk init`) use `rsmk release` to cut versions of their résumé PDF.

### What `rsmk release` Does

When an end user runs `rsmk release` in their résumé repository:

1. **Schema Version Extraction**: Reads `meta.version` from their
   `content.yaml`.
2. **Workflow Drift Check**: Warns if the repo's `.github/workflows/` are out of
   date with the installed CLI version.
3. **Clean Tree Check**: Verifies there are no uncommitted changes.
4. **Upstream Sync Check**: Verifies the local branch is up to date with
   `origin`.
5. **SemVer Monotonicity**: Verifies `meta.version` is strictly greater than all
   prior git tags in the repository.
6. **Strict 1-Page Layout Validation**: Runs an in-memory layout check
   (`rsmk build --check`) to guarantee single-page geometry before publishing.
7. **Atomic Tag & Push**: Creates tag `vX.Y.Z` and pushes it to GitHub,
   triggering their personal GitHub Actions workflow to build and attach their
   compiled PDF to a GitHub Release.

```sh
# Dry run pre-flight check without creating tags
rsmk release --dry-run

# Cut and publish resume release
rsmk release
```

---

## 3. Schema Releases (`s*`)

Schema releases publish JSON schema versions (e.g. `s1.0`, `s1.1`) independently
of CLI binary patches:

```sh
git tag -a sX.Y -m "Schema sX.Y"
git push origin sX.Y
```

Pushing an `s*` tag triggers `.github/workflows/schema.yml`, which:

1. Sets `make_latest: false` so schema releases never displace binary releases.
2. Uploads `resume.schema.json` as a release asset.
3. Makes the schema available at the permanent URL:
   `https://github.com/arvinduh/resumake/releases/download/sX.Y/resume.schema.json`
