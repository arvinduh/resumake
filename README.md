# Resumake (`rsmk`)

[![CI](https://github.com/arvinduh/resumake/actions/workflows/ci.yml/badge.svg)](https://github.com/arvinduh/resumake/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

`rsmk` compiles a résumé written in YAML or JSON to a PDF using Typst, and
checks layout telemetry as it builds: page count, final-page fill, line wraps,
orphan trailing lines, and font availability.

## Features

- **One binary**: Typst is compiled in, with embedded fonts, so there is no
  Typst CLI or external toolchain to install.
- **Project manifest (`resumake.toml`)**: Configure shared `[build]` defaults,
  `[telemetry]` gates, and multiple named `[docs.<name>]` tracks (`publish`
  defaults to `false` so drafts never ship accidentally).
- **Layout telemetry**: Every check and build reports page fill percentages,
  spare vertical height, wrapped bullets, trailing orphan lines, and missing
  requested fonts (`strict_fonts`).
- **Multi-format + IDE IntelliSense**: Author résumés in YAML (`.yaml`, `.yml`)
  or JSON (`.json`), backed by Draft-07 JSON Schemas
  (`schemas/content.schema.json` and `schemas/manifest.schema.json`).
- **Accessible PDFs**: Output is a tagged PDF, so pasted text preserves line
  breaks and parsers see headings and lists.
- **Adaptive spacing**: Every gap derives from `theme.leading`, and spare page
  height is shared across gaps (capped by `theme.stretch`), so shorter résumés
  breathe evenly.
- **Résumé CI**: `rsmk init` scaffolds GitHub Actions workflows that run `rsmk
  check` on every push and stage built PDFs into `.rsmk/dist/*.pdf` on tag
  releases.

## Install

On Linux (x64) and macOS (Apple Silicon):

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/arvinduh/resumake/releases/latest/download/rsmk-installer.sh | sh
```

On Windows (x64):

```powershell
powershell -c "irm https://github.com/arvinduh/resumake/releases/latest/download/rsmk-installer.ps1 | iex"
```

The installers verify checksums and put `rsmk` in `~/.local/bin`, or
`~/.cargo/bin` if that is where your binaries already live. Archives for each
platform are also attached to every
[release](https://github.com/arvinduh/resumake/releases/latest).

`rsmk update` replaces the running `rsmk` with the latest release, and `rsmk
update --check` only reports whether one exists.

## Quickstart

```sh
mkdir my-resume && cd my-resume
rsmk init --name "Jane Doe"
```

This writes:

- `resumake.toml`: build defaults, telemetry rules, and `[docs.main]` target.
- `content.yaml` (or `content.json` with `--format json`): starter sections with
  IDE schema IntelliSense.
- `.github/workflows/`: CI (`rsmk check`) and release workflows.
- `.gitignore` and `.gitattributes`, and runs `git init`.

Edit `content.yaml`. Résumé documents separate `meta`, `theme`, and `sections`:

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/arvinduh/resumake/main/schemas/content.schema.json
meta:
  name: "Jane Doe"
  title: "Software Engineer"
  contact:
    - name: "jane@example.com"
      link: "mailto:jane@example.com"
    - name: "github.com/janedoe"
      link: "https://github.com/janedoe"

theme:
  font_family: "Libertinus Serif"
  paper_size: "us-letter"
  margin: "0.5in"

sections:
  - title: "Experience"
    type: "experience"
    items:
      - org: "Acme Corp"
        location: "San Francisco, CA"
        roles:
          - title: "Software Engineer"
            dates: "2023 - Present"
            bullets:
              - "Built a streaming pipeline that cut latency by 40%."

  - title: "Skills"
    type: "skills"
    items:
      Languages: "Rust, Python, TypeScript, Go"
```

Manage build settings and tracks in `resumake.toml`:

```toml
#:schema https://raw.githubusercontent.com/arvinduh/resumake/main/schemas/manifest.schema.json

[build]
template = "classic"

[telemetry]
max_pages = 1
max_bullet_lines = 1
strict_fonts = true
# min_pages = 1
# min_fill = 75.0
# orphan_lines = "ignore" # "ignore" | "warn" | "error"

[docs.main]
content = "content.yaml"
publish = true
```

Then verify and build:

```sh
rsmk build # builds all publish = true docs (e.g. jane_doe_resume.pdf)
rsmk check # checks all [docs] targets without writing PDFs
```

```text
[main] built jane_doe_resume.pdf (1 page, 42.8% fill)
```

`build` compiles every `[docs.<name>]` entry with `publish = true` (or a named
target such as `rsmk build ml`, `--all`, or standalone `content.{yaml,yml,json}`
when no manifest exists). `check` validates all `[docs]` entries, including
drafts (`publish = false`).

To publish, tag the résumé repository. The scaffolded release workflow builds
published targets into `.rsmk/dist` and attaches them to a GitHub Release:

```sh
git tag 1.0.0 && git push origin 1.0.0
```

## Commands

```text
rsmk build [TARGET]           Compile doc name, content path, or all publish=true docs
  -t, --template <TPL>        Built-in template name, dir, or .typ file
  -o, --output <PDF>          Output path (single target only)
  --out-dir <DIR>             Output directory (overrides [build].out_dir)
  --all                       Build all [docs] regardless of publish
  --schema <FILE>             Custom content JSON Schema
  --font-path <DIR>           Extra font directory (./fonts auto-detected)
  --min-pages <N>             Minimum pages required (default 1)
  --max-pages <N>             Maximum pages allowed (default 1)
  --max-bullet-lines <N>      Maximum lines per bullet/row (default 1)
  --min-fill <PCT>            Minimum final-page fill percentage
  --strict-fonts <BOOL>       Fail when requested font_family is missing (true/false)
  --orphan-lines <POLICY>     Short tail wraps: ignore | warn | error
rsmk check [TARGET]           Validate schema and layout telemetry without PDFs

rsmk init [DEST]              Scaffold a résumé repository (default: .)
  -n, --name <NAME>           Candidate name
  --format <FMT>              Starter content format: yaml | json (default yaml)
  --no-manifest               Skip generating resumake.toml
  -e, --eject                 Copy template to templates/<name>.typ
  --no-git                    Skip git init, .gitignore, and .gitattributes
  --no-workflows              Skip GitHub Actions workflows
  -f, --force                 Overwrite existing files

rsmk template list            List built-in templates
rsmk template eject <NAME>    Copy a template to templates/<NAME>.typ

rsmk update                   Install the latest release
  --check                     Only check whether a newer release exists
rsmk schema                   Print canonical JSON Schema
  --kind <KIND>               Schema kind: content | manifest (default content)
  -o, --output <FILE>         Write schema JSON to FILE

-q, --quiet                   Print only errors (any command)
```

## Development

The toolchain is pinned in `rust-toolchain.toml`. CI runs:

```sh
fml fmt --check
fml lint
cargo clippy --all-targets -- -D warnings
cargo test
```

[`fml`](https://github.com/arvinduh/formality) wraps rustfmt, clippy and the
formatters for the repo's other files.

## License

Licensed under the [MIT License](LICENSE).
