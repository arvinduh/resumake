# AGENTS.md

resumake is a modular résumé compiler and layout telemetry engine.

## Toolchain

`rust-toolchain.toml` pins stable `1.98.1`, and CI installs it from there.
resumake must build on stable.

## Compatibility

Until 1.0.0 nothing keeps backwards compatibility, CLI flags and schema keys
included: no shims, deprecated aliases or migration paths. Removing something
removes it outright.

## CI and merging

- `.github/workflows/ci.yml` runs `fml fmt --check`, `fml lint`, clippy, and the
  tests.
- PRs are squash-merged.

## Releases

Bare `x.y.z` tags, never `vx.y.z`.

## Ask first

- Version bumps and release tags.

## Never

- Commit directly to `main`.
