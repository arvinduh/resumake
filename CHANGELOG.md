# Changelog

All notable changes to this project will be documented in this file.

## [0.3.0] - 2026-08-31

### Features

- *(init)* Scaffold CI/Release workflows only alongside a git repo (#45) (654768b)
- *(cli)* Add `rsmk update` — self-update from GitHub releases (#57) (f444644)
- *(build)* Event-based --watch via the notify crate (Fixes #50) (#61) (1e6bfc1)

### Bug Fixes

- *(installers)* Make binary replacement atomic and update-safe (#55) (0b79c61)

### Performance

- *(engine)* [**breaking**] Run Typst as an embedded library instead of a subprocess (Fixes #46) (#65) (d25fc4b)

### Refactor

- *(deps)* Replace hand-rolled SHA-256 and unified-diff with sha2 + similar (#54) (98d0993)
- *(cli)* [**breaking**] Reduce to a 4-command surface; committed schema + drift test (#56) (0ff71c8)
- *(schema)* Publish resume.schema.json from CI only, never commit it (#58) (61539e9)
- *(release)* Use the semver crate for version parsing and comparison (Fixes #48) (#59) (401b03d)
- *(engine)* Embed the template registry with include_dir (#60) (97aaf9f)
- *(ui)* Consolidate terminal output on comfy-table (#62) (cad945d)
- *(git)* Use gix for read-only git queries in release/init (#63) (dfc39b1)
- Typed errors with thiserror/anyhow instead of Result<_, String> (Fixes #53) (#64) (d8e0894)

### Build

- Adopt cargo-dist for release artifacts and installers (Fixes #47) (#66) (6aa4a12)

## [0.2.0] - 2026-08-31

### Features

- *(agents)* Adopt formality agentic workflow, quality gates, and orchestrator process (#26) (5d2a5cd)
- *(install)* Support RESUMAKE_VERSION pin and verify .sha256 checksums (#16) (e8c98a5)
- *(schema)* Support meta.extra extensible fields in content.yaml (Fixes #29) (#31) (853ed92)
- *(cli)* Install as rsmk, keeping resumake as a secondary name (Fixes #19) (#32) (76ae6f0)
- *(cli)* Consolidate build, check, and watch into rsmk build [--check] [--watch] (Fixes #27) (#35) (2913ac5)
- *(template)* Add rsmk template list and eject subcommands (Fixes #28) (#36) (1fefbf6)
- *(ci)* Thin workflow stubs calling standard rsmk build commands (#37) (8c0fe68)
- *(release)* Add rsmk release with pre-flight checks (Fixes #20) (#38) (6d1d3ad)
- *(init)* Interactive rsmk init with gh integration, workflows, and provenance headers (Fixes #22) (#39) (4042bc7)
- *(init)* Add rsmk init --update to refresh workflows, and warn on version skew (Fixes #23) (#40) (bdfb20b)

### Bug Fixes

- *(engine)* Reject --content outside the project root with a clear error (#18) (e4415e8)
- *(init)* Scaffold uses Libertinus Serif embedded font (Fixes #24) (#30) (ace3939)

### Documentation

- *(schema)* Document s* releases as the canonical schema URL (#15) (a2d2200)
- Document 4-command surface, quickstart, template ejection, and release workflow (Fixes #25) (#41) (76ca33e)

### Styling

- *(docs)* Realign theme token table after font default rename (321b795)

### Testing

- *(cli)* Skip end-to-end test when typst is not on PATH (#17) (d9ec5da)

### Ci

- *(formality)* Upgrade to fml v0.2.1 and pin the version (431c324)
- *(release)* Create release once, cross-compile intel mac, gate dispatch, add timeouts (#14) (e9d955a)
- *(audit)* Replace deprecated rustsec/audit-check with prebuilt cargo-audit via install-action (#34) (89deedf)

## [0.1.1] - 2026-08-28

### Bug Fixes

- *(template)* Lead font fallback chain with Typst-embedded families (a3d8c67)

## [0.1.0] - 2026-08-28

### Features

- *(core)* Add modular Typst block layout engine and resume data models (f58fcbf)
- *(schema)* Implement in-process YAML validation, schema export, and scaffolding (27a8052)
- *(engine)* Implement Typst subprocess orchestration and component cache (a5ed5b7)
- *(telemetry)* Implement single-page geometry evaluation and ANSI status UI (68b9535)
- *(cli)* Implement CLI subcommands and end-to-end integration tests (59553d5)
- *(engine)* Modularize template registry and harden schema validation (67eea06)
- *(ui)* Migrate telemetry table to comfy-table (25d07d7)
- *(install)* Add 1-line installers and cargo-binstall support (e78fcd5)
- *(githooks)* Optimize pre-commit hook with tiered fast unit testing (2ee299f)

### Bug Fixes

- *(ci)* Streamline test matrix, configure Pages enablement, and fix scaffold bullet wrap (5eb7d64)

### Documentation

- Add production README and Google Developer Style documentation suite (453082f)
- Update documentation suite for modular templates and schema validation (26b6497)
- *(release)* Document release procedure and add quick install guides (99fd5b5)
- *(release)* Document schema release workflow and make_latest policies (7986dcd)

### Refactor

- *(tooling)* Remove standalone native configs in favor of singular formality.toml (78cf30a)

### Miscellaneous Tasks

- *(tooling)* Configure formality, githooks, and CI workflows (3a6727a)
- Add MIT license (fa5eb7e)
- *(docs)* Remove GitHub Pages deployment workflow and docs badge (2d8f2bf)
- *(schema)* Pin schema url to s1.0 release tag (1ad052e)

### Ci

- *(github)* Add multi-platform CI matrix, docs deployment, and release workflows (22559a4)
- *(release)* Automate changelog generation with git-cliff and target-matched assets (6956cbc)
- *(release)* Add schema workflow and configure make_latest policies (1d54358)


