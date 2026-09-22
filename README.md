# Resumake (`rsmk`)

[![CI](https://github.com/arvinduh/resumake/actions/workflows/ci.yml/badge.svg)](https://github.com/arvinduh/resumake/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

> High-performance native Rust résumé compiler, in-process schema validator, and
> strict golden-ratio layout telemetry engine.

---

## Features

- **Blazing Fast**: Native Rust binary compiling single-page résumés in under
  100ms.
- **Zero Rust Required**: Prebuilt standalone binaries with instant 1-line
  installation.
- **In-Process Typst Engine**: Embedded compiler and modular templates—no
  external Typst CLI or font downloads required.
- **Strict Layout Telemetry**: Diagnostics measuring page count, vertical space
  fill percentage, and bullet wrapping.
- **In-Place Self-Updates**: Built-in `rsmk update` to automatically upgrade to
  the latest release.
- **Automated Résumé CI/CD**: Scaffolds GitHub Actions workflows to compile and
  release your PDF automatically on tag.
- **Accessible, Copyable PDFs**: Output is a tagged PDF (headings, paragraphs,
  lists), so text pasted into Word or Docs keeps its line breaks and parsers see
  the document's structure.
- **Adaptive Spacing**: Every gap derives from one `theme.leading` value, and
  spare page height is shared across section and entry gaps (capped by
  `theme.stretch`) so short résumés loosen instead of ending half-empty.

---

## Installation

### 1-Line Quick Install

#### macOS & Linux

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/arvinduh/resumake/releases/latest/download/resumake-installer.sh | sh
```

#### Windows (PowerShell)

```powershell
irm https://github.com/arvinduh/resumake/releases/latest/download/resumake-installer.ps1 | iex
```

Both installers download the release archive over HTTPS, verify checksums, and
configure your binary path (`~/.cargo/bin` or `%USERPROFILE%\.cargo\bin`).

#### Direct Prebuilt Binaries

Standalone prebuilt archives (`.zip` for Windows, `.tar.xz` for Linux and macOS)
are attached to every
[GitHub Release](https://github.com/arvinduh/resumake/releases/latest).

---

## Updating

To update your installed `rsmk` binary to the latest version at any time:

```bash
rsmk update
```

To verify if a new release is available without installing:

```bash
rsmk update --check
```

---

## Quickstart

### 1. Initialize a New Résumé Repository

```bash
mkdir my-resume && cd my-resume
rsmk init --name "Jane Doe"
```

This scaffolds:

- `content.yaml`: Configured with starter sections and JSON Schema header.
- `.gitignore`: Ignores generated PDF outputs and cache files.
- `.gitattributes`: Normalizes line endings (`* text=auto eol=lf`).
- `.github/workflows/`: CI verification and automated GitHub Release workflows.

### 2. Edit Your Content (`content.yaml`)

Resumake separates your career content from styling. Edit `content.yaml`:

```yaml
# yaml-language-server: $schema=https://github.com/arvinduh/resumake/releases/download/s1.0/resume.schema.json
meta:
  name: "Jane Doe"
  version: "1.0.0"
  role: "Senior Software Engineer"
  contact:
    - name: "jane@example.com"
      url: "mailto:jane@example.com"
    - name: "github.com/janedoe"
      url: "https://github.com/janedoe"
    - name: "San Francisco, CA"

sections:
  - title: "Experience"
    type: "experience"
    items:
      - company: "Acme Corp"
        role: "Senior Software Engineer"
        date: "2023 – Present"
        location: "San Francisco, CA"
        bullets:
          - "Architected real-time streaming pipeline reducing latency by 40%."
          - "Mentored 5 junior engineers and led technical design reviews."

  - title: "Education"
    type: "education"
    items:
      - institution: "University of California, Berkeley"
        degree: "B.S. in Computer Science"
        date: "2019 – 2023"

  - title: "Skills"
    type: "skills"
    items:
      - category: "Languages"
        skills: ["Rust", "Python", "TypeScript", "Go"]
      - category: "Infrastructure"
        skills: ["Kubernetes", "Docker", "AWS", "Terraform"]
```

### 3. Compile Your Résumé

```bash
rsmk build
```

Compiles your PDF (`janedoe_resume.pdf`) and displays layout telemetry metrics.

### 4. Dry-Run Verification

```bash
rsmk build --check
```

Validates YAML schema and evaluates layout constraints without writing a PDF to
disk.

### 5. Tag & Publish a Résumé Release

```bash
rsmk release
```

Runs 5 automated pre-flight assertions (clean working tree, upstream sync,
SemVer monotonicity, layout check) before creating and pushing git tag
`v<version>` to trigger the GitHub Actions release workflow.

---

## Layout Telemetry Diagnostics

Every compile checks golden-ratio layout geometry:

```txt
───────────────────────────────────────────────────────────────────────
 Candidate:       Jane Doe
 Output:          janedoe_resume.pdf
 Version:         1.0.0
 Page Count:      1 page(s)                                 [PASS 1/1]
 Vertical Fill:   95.2% (spare: 0.38 in)                     [OPTIMAL]
 Line Wraps:      0 wrapped items                            [0 WRAPS]
 Underfills:      0 items (<86%)                             [0 UNDER]
 Status:          SUCCESS (Strict 1-page layout verified)
───────────────────────────────────────────────────────────────────────
```

- **Page Count Guard**: Strictly fails if content spills over to page 2.
- **Vertical Fill Percentage**: Target between 90% and 98% for optimal white
  space balance. Measured at natural spacing, before spare height is shared out
  across the flexible gaps.
- **Line Wrap Detector**: Highlights bullet points that wrap only one or two
  orphan words onto a second line.

---

## CLI Command Surface

```bash
# Compilation & Telemetry
rsmk build [CONTENT]            # Compile PDF and evaluate layout geometry (defaults to content.yaml)
rsmk build -t, --template <TPL> # Built-in template name, local directory, or .typ file
rsmk build -o, --output <PDF>   # Custom output PDF path

# Validation
rsmk check [CONTENT]            # Verify schema & single-page layout geometry without writing PDF

# Project Scaffolding
rsmk init [DEST]                # Scaffold into current directory, target folder, or custom YAML file
rsmk init --name <NAME>         # Scaffold with specific candidate name
rsmk init --no-git              # Scaffold content.yaml only (skip git repo and workflows)
rsmk init --workflows           # Add or refresh workflow stubs (alias: --update)
rsmk init -f, --force           # Overwrite existing files

# Résumé Release
rsmk release [CONTENT]          # Pre-flight assertions + tag meta.version + push
rsmk release --dry-run          # Test pre-flight assertions without tagging
rsmk release -m <MESSAGE>       # Custom annotated tag message

# Template Management
rsmk template --list            # List built-in and local custom templates
rsmk template <NAME>            # Eject template tree to ./templates/<NAME>/
rsmk template <NAME> -f, --force# Overwrite existing ejected template directory

# Binary Self-Update
rsmk update                     # In-place update to the latest release
rsmk update --check             # Check if an update is available
rsmk update -f, --force         # Force reinstallation of latest release

# Schema (Tooling & Offline IDEs)
rsmk schema                     # Print canonical JSON Schema to stdout
rsmk schema -o, --output <PATH> # Export schema to a local file
```

---

## Development

Prerequisites: [Rust 1.80+](https://rustup.rs/)

```bash
# Run unit tests
cargo test --lib -q

# Run clippy linter
cargo clippy --all-targets -- -D warnings

# Build binary
cargo build --release
```

---

## License

Licensed under the [MIT License](LICENSE).
