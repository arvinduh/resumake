# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-08-28

### Features

- _(core)_ Add modular Typst block layout engine and resume data models
  (f58fcbf)
- _(schema)_ Implement in-process YAML validation, schema export, and
  scaffolding (27a8052)
- _(engine)_ Implement Typst subprocess orchestration and component cache
  (a5ed5b7)
- _(telemetry)_ Implement single-page geometry evaluation and ANSI status UI
  (68b9535)
- _(cli)_ Implement CLI subcommands and end-to-end integration tests (59553d5)
- _(engine)_ Modularize template registry and harden schema validation (67eea06)
- _(ui)_ Migrate telemetry table to comfy-table (25d07d7)
- _(install)_ Add 1-line installers and cargo-binstall support (e78fcd5)
- _(githooks)_ Optimize pre-commit hook with tiered fast unit testing (2ee299f)

### Bug Fixes

- _(ci)_ Streamline test matrix, configure Pages enablement, and fix scaffold
  bullet wrap (5eb7d64)

### Documentation

- Add production README and Google Developer Style documentation suite (453082f)
- Update documentation suite for modular templates and schema validation
  (26b6497)
- _(release)_ Document release procedure and add quick install guides (99fd5b5)

### Refactor

- _(tooling)_ Remove standalone native configs in favor of singular
  formality.toml (78cf30a)

### Miscellaneous Tasks

- _(tooling)_ Configure formality, githooks, and CI workflows (3a6727a)
- Add MIT license (fa5eb7e)
- _(docs)_ Remove GitHub Pages deployment workflow and docs badge (2d8f2bf)

### Ci

- _(github)_ Add multi-platform CI matrix, docs deployment, and release
  workflows (22559a4)
- _(release)_ Automate changelog generation with git-cliff and target-matched
  assets (6956cbc)
