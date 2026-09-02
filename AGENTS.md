# AGENTS.md

resumake: Modular, golden-ratio résumé compiler and layout telemetry engine.

## Commands

```bash
cargo test --lib -q
cargo clippy --all-targets -- -D warnings
cargo run -q -- build
```

Activate the staged pre-commit hook:

```bash
git config core.hooksPath .githooks
```

## Layout

- `src/cli.rs` — CLI subcommand options, arguments, and clap parser
- `src/commands/` — CLI subcommand handlers (`build.rs`, `init.rs`,
  `release.rs`, `template.rs`, `update.rs`)
- `src/engine/` — In-process Typst engine orchestration, World implementation,
  and template registry
- `src/error.rs` — Crate-level error umbrella, Result alias, and classification
  helpers
- `src/models.rs` — Canonical Resume data structures, metadata, and serde models
- `src/schema.rs` — JSON Schema generator, validation logic, and drift
  verification
- `src/telemetry.rs` — Strict 1-page layout geometry calculations, overflow, and
  wrap checks
- `src/utils/` — Shared cross-cutting utilities (`git.rs`, `ui.rs`, `fs.rs`)
- `src/embedded/` — Built-in templates (`classic/`), workflow templates, and
  initialization assets
- `docs/` — Architecture documentation, guides, and ADRs (see `docs/INDEX.md`)
- `tests/` — CLI integration and end-to-end test suites

## Progressive 2-Tier Quality Gate

1. **Tier 1 (Local pre-commit)**: `.githooks/pre-commit` (activated via
   `git config core.hooksPath .githooks`) runs `fml fmt --staged` / `fml lint`
   (or falls back to `cargo fmt --check` / `cargo clippy`) and fast unit tests
   (`cargo test --lib -q`) before committing.
2. **Tier 2 (Parallel CI checks)**: `.github/workflows/ci.yml` and `schema.yml`
   run:
   - `Polyglot Lint & Format`: `fml fmt --check` and `fml lint`.
   - `Security Audit`: `cargo audit` against Rust advisory database.
   - `Multi-OS Test Matrix`: Full test suite across Linux and Windows.

## Conventions

- Commits: `type(scope): description (Fixes #issue)`, Conventional Commits
  style.
- Granular, atomic commits: every commit must be super well-scoped and represent
  exactly one logical change. While PRs can be larger, they should be composed
  of multiple clean, atomic commits.
- Maintain telemetry contracts: all templates must emit `<pageinfo>` and route
  bullets through `<bulletinfo>`.
- Schema stability: `src/models.rs` is canonical. `resume.schema.json` is never
  committed — the release workflows generate it from the models with
  `cargo run --example generate-schema` and publish it as a release asset.
- Modern module architecture: use `mod_name.rs` (and `mod_name/` for submodules)
  — never `mod.rs` files.
- Subsystem boundaries: keep CLI commands in `src/commands/`, compilation in
  `src/engine/`, shared plumbing in `src/utils/`, and domain models in
  `src/models.rs`, `src/schema.rs`, `src/telemetry.rs`.
- Colocated domain errors: each subsystem defines its own domain errors,
  aggregated into the crate umbrella `ResumakeError` in `src/error.rs` with the
  `Result<T>` alias and classification helpers.
- Absolute import hierarchy: use absolute paths (`crate::...`) for all internal
  crate imports.
- Zero internal re-exporting: never use `pub use crate::...` within internal
  submodules. Only `src/lib.rs` exports the public API surface.
- Visibility boundaries: keep internal engine, CLI, and scaffolding logic
  cleanly scoped at the module declaration level in `src/lib.rs`.
- In-process Typst engine: never invoke external `typst` CLI binaries;
  compilation and telemetry are strictly evaluated in-process via Typst crates.
- CI monitoring: use `gh pr checks <PR> --watch` (or `gh run watch`) instead of
  manual polling loops to stream live status and reactively block until
  pass/fail.
- Release boundaries: `rsmk release` is strictly an end-user command for résumé
  repositories containing `content.yaml`. Resumake crate releases are cut by
  pushing a `v*` tag directly to `main` (driven by `cargo-dist`).
- Always run the freshly built binary (`cargo run -q -- ...`), never a stale
  global `resumake`/`rsmk` on PATH.

## Always

- Run `cargo test --lib -q && cargo clippy --all-targets -- -D warnings` before
  any commit.
- Check `docs/INDEX.md` before reading source to understand structure or
  conventions already documented there.

## Ask first

- Anything touching branch protection or CI required-status-check names.
- Version bumps or release workflow triggers (owned by release tooling).
- Breaking changes to CLI subcommand semantics or `ResumeDocument` schema
  fields.

## Never

- Commit directly to `main`.
- Break single-page layout telemetry guarantees or remove metadata queries.

Default to dispatching worker subagents in isolated worktrees, not editing
source directly — see `.agents/orchestrate.md` §8 for the narrow, enumerated
exceptions. That file also covers worktrees, the maker-checker QA gate, dispatch
order, and design-phase/applied-feature rules.
