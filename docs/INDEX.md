# Resumake Documentation Index

This document is the central sitemap for all documentation in this repository.
Agents and contributors should check this index before inspecting source code to
understand existing architecture, conventions, and design decisions.

---

## 1. Developer & Agent Onboarding

- **[AGENTS.md](../AGENTS.md)** — Universal agent entrypoint: repository layout,
  essential presubmit commands, 2-tier quality gate, conventions, and
  operational rules.
- **[.agents/orchestrate.md](../.agents/orchestrate.md)** — Comprehensive
  orchestration manual: worktree isolation, claim-then-verify protocol,
  maker-checker QA review, post-merge cleanup, and label-based issue state.
- **[.agents/start.md](../.agents/start.md)** — Copy-pasteable prompt templates
  for fresh orchestrator sessions, resumption, and QA reviewer passes.
- **[Contributing Guide](contributing.md)** — Local development environment
  setup, toolchain prerequisites, test suite, and template authoring
  instructions.

---

## 2. Product & Engine Architecture

- **[System Architecture](architecture.md)** — Native Rust engine pipeline,
  embedded Typst compilation, template registry, and key design decisions.
- **[YAML Schema Reference](schema-guide.md)** — Full specification of
  `ResumeDocument`, metadata directives, block types, and JSON Schema
  generation.
- **[Layout Telemetry Guide](telemetry-guide.md)** — Strict single-page
  geometry, `<pageinfo>` / `<bulletinfo>` metadata queries, fill percentages,
  and underfill/overflow guards.

---

## 3. User Guides & Operations

- **[Getting Started](getting-started.md)** — End-user tutorial: installation,
  initializing a resume workspace (`resumake init`), checking layout
  (`resumake check`), and compiling PDFs (`resumake build`).
- **[Release Procedure](release.md)** — Release workflows, version tagging,
  multi-platform binary builds, and changelog generation.
- **[Documentation Hub README](README.md)** — Documentation suite overview and
  table of contents.
