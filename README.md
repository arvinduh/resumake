# Resumake

[![CI](https://github.com/arvinduh/resumake/actions/workflows/ci.yml/badge.svg)](https://github.com/arvinduh/resumake/actions/workflows/ci.yml)
[![Docs](https://github.com/arvinduh/resumake/actions/workflows/docs.yml/badge.svg)](https://arvinduh.github.io/resumake/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

> High-performance native Rust résumé compiler, in-process schema validator, and
> layout telemetry engine.

---

## Features

- **Blazing Fast**: Native Rust binary compiling single-page résumés in under
  100ms.
- **In-Process Schema Validation**: Validates YAML schemas before spawning any
  compiler process.
- **Strict Layout Telemetry**: Zero-emoji terminal badges measuring page count,
  vertical space fill percentage, and bullet wrapping.
- **Zero-Dependency Engine**: Built-in modular Typst engine and design tokens
  embedded directly in the binary.
- **Live Watch Mode**: Real-time document recompilation on file change
  (`resumake watch`).

---

## Installation

### 1-Line Quick Install

#### Linux & macOS

```bash
curl -fsSL https://raw.githubusercontent.com/arvinduh/resumake/main/install.sh | sh
```

#### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/arvinduh/resumake/main/install.ps1 | iex
```

### Package Managers / Cargo

#### Fast install via `cargo-binstall` (zero compilation)

```bash
cargo binstall resumake
```

#### Build from source via `cargo`

```bash
cargo install --path .
# or from the git repository:
cargo install --git https://github.com/arvinduh/resumake
```

### Direct Prebuilt Binaries

Prebuilt standalone binaries are attached to every
[GitHub Release](https://github.com/arvinduh/resumake/releases/latest).

---

## Quickstart

### 1. Scaffold a New Résumé

```bash
resumake init --name "Jane Doe"
```

### 3. Compile to PDF with Layout Telemetry

```bash
resumake build
```

---

## Telemetry Terminal Output

```txt
────────────────────────────────────────────────────────────────
 Candidate:       Jane Doe
 Output:          janedoe_resume.pdf
 Version:         1.0.0
────────────────────────────────────────────────────────────────
 Page Count:      1 page(s)                             [PASS 1/1]
 Vertical Fill:   95.2% (spare: 0.38 in)                 [OPTIMAL]
 Line Wraps:      0 wrapped items                        [0 WRAPS]
 Underfills:      0 items (<86%)                         [0 UNDER]
────────────────────────────────────────────────────────────────
 Status: SUCCESS (Strict 1-page layout verified)
────────────────────────────────────────────────────────────────
```

---

## CLI Reference

```bash
resumake build              # Compile resume to PDF and evaluate layout
resumake check               # Dry-run validation without writing a PDF
resumake watch               # Real-time auto-recompile on file change
resumake schema              # Print canonical JSON Schema (Draft-07) to stdout
resumake schema --export out.json  # ...or write it to a file
resumake init --name <NAME>  # Scaffold new content.yaml with directives
```

`build`, `check`, and `watch` also accept:

```bash
resumake build --template <name>   # Pick a built-in layout (default: classic)
resumake build --source <path.typ> # Bypass the registry with your own Typst file
```

The same `content.yaml` renders under any registered template — see
[System Architecture](docs/architecture.md#4-key-design-decisions) for how the
template registry works and [Contributing](docs/contributing.md) for how to add
a new one.

---

## Documentation Hub

Explore the complete documentation in the [`docs/`](docs/README.md) directory:

| Guide                                                 | Description                                                                        |
| :---------------------------------------------------- | :--------------------------------------------------------------------------------- |
| **[Getting Started Guide](docs/getting-started.md)**  | Step-by-step tutorial from installation to your first single-page PDF.             |
| **[YAML Schema Reference](docs/schema-guide.md)**     | Full specification for layout directives, tokens, metadata, and block sections.    |
| **[Layout Telemetry Guide](docs/telemetry-guide.md)** | Deep dive into page count verification, vertical fill percentage, and wrap checks. |
| **[System Architecture](docs/architecture.md)**       | Modular design, embedded Typst orchestrator, and compiler pipeline.                |
| **[Contributing Guide](docs/contributing.md)**        | Development environment setup, coding conventions, testing, and PR checklist.      |
| **[Release Procedure](docs/release.md)**              | Version tagging, git-cliff changelogs, and binary distribution workflow.           |

---

## Contributing

We welcome contributions from the community!

1. Fork the repository and create your branch from `main`:

   ```bash
   git checkout -b feat/my-improvement
   ```

2. Ensure all formatting, linter, and test checks pass:

   ```bash
   fml sync --check
   fml fmt --check
   fml lint
   cargo test --all-targets
   cargo doc --no-deps
   ```

3. Open a Pull Request on GitHub. For comprehensive contribution guidelines, see
   [docs/contributing.md](docs/contributing.md).

---

## Release Process

Releases are driven by Conventional Commits and automated via GitHub Actions:

1. Update `version` in `Cargo.toml`.
2. Generate changelog with `git-cliff` and commit.
3. Tag the release commit: `git tag -a vX.Y.Z -m "vX.Y.Z"`.
4. Push the tag: `git push origin vX.Y.Z`.
5. GitHub Actions automatically compiles cross-platform binaries (Linux x86_64,
   macOS ARM64/Intel, Windows x86_64), generates SHA-256 checksums, publishes
   `resume.schema.json`, and attaches release notes via `git-cliff`.

For the complete maintainer walkthrough, see [docs/release.md](docs/release.md).

---

## License

MIT License. Copyright (c) 2026 Resumake Authors.
