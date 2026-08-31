# Resumake (`rsmk`)

[![CI](https://github.com/arvinduh/resumake/actions/workflows/ci.yml/badge.svg)](https://github.com/arvinduh/resumake/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

> High-performance native Rust résumé compiler, in-process schema validator, and
> strict golden-ratio layout telemetry engine.

---

## Features

- **Blazing Fast**: Native Rust binary compiling single-page résumés in under
  100ms.
- **In-Process Schema Validation**: Validates YAML schemas before spawning any
  compiler process.
- **Strict Layout Telemetry**: Zero-emoji terminal diagnostics measuring page
  count, vertical space fill percentage, and bullet wrapping.
- **Standalone 4-Command Surface**: Complete lifecycle support via `build`,
  `init`, `release`, and `template`.
- **Zero-Dependency Engine**: Built-in modular Typst engine and design tokens
  embedded directly in the binary.
- **CI/CD Automation**: Embedded GitHub Actions workflows with automatic SHA-256
  provenance tracking and release management.

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

Both installers download the release archive over HTTPS, verify it against the
published `.sha256` checksum, and refuse to install on a mismatch.

#### Environment variables

| Variable               | Default   | Effect                                                                                                                 |
| :--------------------- | :-------- | :--------------------------------------------------------------------------------------------------------------------- |
| `RESUMAKE_VERSION`     | `latest`  | Pin a specific release, e.g. `RESUMAKE_VERSION=0.1.1` (a leading `v` is accepted). `latest` tracks the newest release. |
| `RESUMAKE_INSTALL_DIR` | see below | Directory to install the `rsmk` binary into (`~/.local/bin` on Linux/macOS, `~/bin` on Windows).                       |

```bash
# Linux & macOS — install a pinned version
curl -fsSL https://raw.githubusercontent.com/arvinduh/resumake/main/install.sh | RESUMAKE_VERSION=0.1.1 sh
```

```powershell
# Windows — install a pinned version
$env:RESUMAKE_VERSION = '0.1.1'; irm https://raw.githubusercontent.com/arvinduh/resumake/main/install.ps1 | iex
```

### Direct Prebuilt Binaries

Prebuilt standalone binaries are attached to every
[GitHub Release](https://github.com/arvinduh/resumake/releases/latest).

---

## Quickstart

### 1. Initialize a New Résumé Repository

```bash
mkdir my-resume && cd my-resume
rsmk init --name "Jane Doe"
```

This scaffolds:

- `content.yaml` configured with `Libertinus Serif` font.
- `.gitignore` (ignoring compiled PDFs and cache artifacts).
- `.gitattributes` (`* text=auto eol=lf`).
- `.github/workflows/ci.yml` & `release.yml` with SHA-256 provenance headers.

### 2. Live Recompilation & Geometry Feedback

```bash
rsmk build --watch
```

Keep your PDF viewer open side-by-side. Every time you save `content.yaml`,
Resumake re-renders the document and evaluates single-page geometry in <100ms.

### 3. Dry-Run Verification

```bash
rsmk build --check
```

Evaluates schema validity and layout constraints without writing a PDF to disk.

### 4. Tag & Cut a Release

```bash
rsmk release
```

Runs 5 automated pre-flight assertions (clean working tree, upstream sync,
SemVer monotonicity, in-memory layout check) before creating and pushing git tag
`v<version>` to trigger the GitHub Actions release workflow.

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

## CLI Command Surface

```bash
# Core Compilation & Telemetry
rsmk build                      # Compile PDF and evaluate layout geometry
rsmk build -c, --check          # Dry-run validation (0 files written)
rsmk build -w, --watch          # Live file-watcher loop on YAML / Typst change
rsmk build -t, --template <TPL> # Pick a template ('classic' or path to main.typ)

# Project Scaffolding & Lifecycle
rsmk init                       # Interactive wizard: git + workflows + gh setup
rsmk init <DEST>                # Scaffold into a directory (created if needed)
rsmk init --name <NAME>         # Scaffold with specific candidate name
rsmk init -u, --update          # Refresh workflow stubs using SHA-256 provenance
rsmk init -f, --force           # Overwrite existing files; scaffold into a non-empty dir

# Release Engine
rsmk release                    # Pre-flight assertions + tag meta.version + push
rsmk release --dry-run          # Test pre-flight assertions without tagging
rsmk release -m <MESSAGE>       # Custom annotated tag message

# Template Management
rsmk template list              # List built-in and local custom templates
rsmk template eject classic     # Extract template tree to ./templates/classic/

# Schema Tools
rsmk schema                     # Print canonical JSON Schema (Draft-07) to stdout
rsmk schema --export out.json   # Export JSON Schema to a file
```

---

## Documentation Hub

Explore the complete documentation in the [`docs/`](docs/README.md) directory:

| Guide                                                 | Description                                                                    |
| :---------------------------------------------------- | :----------------------------------------------------------------------------- |
| **[Documentation Index](docs/INDEX.md)**              | Central sitemap for all documentation, specs, and orchestration files.         |
| **[Getting Started](docs/getting-started.md)**        | Step-by-step tutorial from installation to cutting your first release.         |
| **[YAML Schema Reference](docs/schema-guide.md)**     | Complete specification of all directives, metadata, and block sections.        |
| **[Layout Telemetry Guide](docs/telemetry-guide.md)** | Learn how strict 1-page geometry, fill percentage, and wrap checks work.       |
| **[System Architecture](docs/architecture.md)**       | Deep dive into the native Rust engine, embedded Typst compiler, and pipeline.  |
| **[Release Procedure](docs/release.md)**              | Version tagging, git-cliff changelogs, and binary distribution workflow.       |
| **[Contributing Guide](docs/contributing.md)**        | Code standards, local test suite, pre-commit setup, and Pull Request workflow. |

---

## License

Licensed under the [MIT License](LICENSE).
