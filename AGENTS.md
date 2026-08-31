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
- `src/engine.rs` — Core compilation pipeline, template extraction, and Typst
  invocation
- `src/init.rs` — Interactive project scaffolding, workflow provenance headers,
  and refresh
- `src/models.rs` — Canonical Resume data structures, metadata, and serde models
- `src/release.rs` — Automated pre-flight git assertions, SemVer 2.0
  monotonicity, and release tagging
- `src/schema.rs` — JSON Schema generator, validation logic, and drift
  verification
- `src/telemetry.rs` — Strict 1-page layout geometry calculations, overflow, and
  wrap checks
- `src/ui.rs` — Visual terminal diagnostics, table formatting, and color output
- `src/update.rs` — `rsmk update` self-update from GitHub Releases via the
  `self_update` crate, plus pure version-decision helpers
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
   - `Schema Drift`: `cargo test --lib schema` asserts the committed
     `resume.schema.json` still matches the models (regenerate with
     `cargo run --example generate-schema`).

## Conventions

- Commits: `type(scope): description (Fixes #issue)`, Conventional Commits
  style.
- Maintain telemetry contracts: all templates must emit `<pageinfo>` and route
  bullets through `<bulletinfo>`.
- Schema stability: `src/models.rs` is canonical. Never hand-edit
  `resume.schema.json`; regenerate it with
  `cargo run --example generate-schema`.
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
