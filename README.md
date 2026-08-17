# Resumake

[![CI](https://github.com/arvinduh/resumake/actions/workflows/ci.yml/badge.svg)](https://github.com/arvinduh/resumake/actions/workflows/ci.yml)
[![Docs](https://github.com/arvinduh/resumake/actions/workflows/docs.yml/badge.svg)](https://arvinduh.github.io/resumake/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

> High-performance native Rust résumé compiler, in-process schema validator, and layout telemetry engine.

---

## Features

- **Blazing Fast**: Native Rust binary compiling single-page résumés in under 100ms.
- **In-Process Schema Validation**: Validates YAML schemas before spawning any compiler process.
- **Strict Layout Telemetry**: Zero-emoji terminal badges measuring page count, vertical space fill percentage, and bullet wrapping.
- **Zero-Dependency Engine**: Built-in modular Typst engine and design tokens embedded directly in the binary.
- **Live Watch Mode**: Real-time document recompilation on file change (`resumake watch`).

---

## Quickstart

### 1. Installation

```bash
cargo install --path .
```

### 2. Scaffold a New Résumé

```bash
resumake init --name "Jane Doe"
```

### 3. Compile to PDF with Layout Telemetry

```bash
resumake build
```

---

## Telemetry Terminal Output

```
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
resumake check              # Dry-run validation without writing a PDF
resumake watch              # Real-time auto-recompile on file change
resumake schema --export    # Export canonical JSON Schema (Draft 2020-12)
resumake init --name <NAME> # Scaffold new content.yaml with directives
```

---

## Documentation Hub

Explore the complete documentation in the [`docs/`](docs/README.md) directory:

| Guide | Description |
| :--- | :--- |
| **[Getting Started Guide](docs/getting-started.md)** | Step-by-step tutorial from installation to your first single-page PDF. |
| **[YAML Schema Reference](docs/schema-guide.md)** | Full specification for layout directives, tokens, metadata, and block sections. |
| **[Layout Telemetry Guide](docs/telemetry-guide.md)** | Deep dive into page count verification, vertical fill percentage, and wrap checks. |
| **[System Architecture](docs/architecture.md)** | Modular design, embedded Typst orchestrator, and compiler pipeline. |
| **[Contributing Guide](docs/contributing.md)** | Development environment setup, coding conventions, testing, and PR checklist. |

---

## Contributing

We welcome contributions from the community!

1. Fork the repository and create your branch from `main`:
   ```bash
   git checkout -b feat/my-improvement
   ```
2. Ensure all formatting, linter, and test checks pass:
   ```bash
   cargo fmt --check
   cargo clippy --all-targets -- -D warnings
   cargo test --all-targets
   cargo doc --no-deps
   ```
3. Open a Pull Request on GitHub. For comprehensive contribution guidelines, see [docs/contributing.md](docs/contributing.md).

---

## Release Process

Releases are automated via GitHub Actions:
1. Update `version` in `Cargo.toml`.
2. Tag the release commit: `git tag vX.Y.Z`.
3. Push the tag: `git push origin vX.Y.Z`.
4. GitHub Actions automatically compiles cross-platform binaries (Linux x86_64, macOS ARM64/Intel, Windows x86_64), generates SHA-256 checksums, and publishes the GitHub Release.

---

## License

MIT License. Copyright (c) 2026 Resumake Authors.
