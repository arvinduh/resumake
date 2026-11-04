# Resumake (`rsmk`)

[![CI](https://github.com/arvinduh/resumake/actions/workflows/ci.yml/badge.svg)](https://github.com/arvinduh/resumake/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

`rsmk` compiles a résumé written in YAML to a one-page PDF, and checks the
layout as it goes: page count, how full the page is, and which bullets wrap.

## Features

- **One binary**: Typst is compiled in, with its fonts, so there is no Typst
  CLI or font download to set up.
- **Layout telemetry**: every build reports page count and vertical fill, and
  flags bullets that wrap a word or two onto a second line.
- **Schema validation**: `content.yaml` is checked against a JSON Schema that
  your editor can use for completion.
- **Accessible PDFs**: output is a tagged PDF, so pasted text keeps its line
  breaks and parsers see headings and lists.
- **Adaptive spacing**: every gap derives from `theme.leading`, and spare page
  height is shared across the gaps (capped by `theme.stretch`), so a short
  résumé loosens instead of ending half-empty.
- **Résumé CI**: `rsmk init` scaffolds GitHub Actions workflows that check
  every push and publish the PDF when you push a tag.

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

`rsmk update` replaces the running `rsmk` with the latest release, and
`rsmk update --check` only reports whether one exists.

## Quickstart

```sh
mkdir my-resume && cd my-resume
rsmk init --name "Jane Doe"
```

This writes:

- `content.yaml`: starter sections, with a schema header for your editor.
- `.github/workflows/`: a CI check and a release workflow.
- `.gitignore` and `.gitattributes`, and runs `git init`.

Edit `content.yaml`. A minimal one looks like this:

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/arvinduh/resumake/main/schema.json
meta:
  name: "Jane Doe"
  version: "1.0.0"
  title: "Software Engineer"
  contact:
    - name: "jane@example.com"
      link: "mailto:jane@example.com"
    - name: "github.com/janedoe"
      link: "https://github.com/janedoe"

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

Then build it:

```sh
rsmk build   # writes jane_doe_resume.pdf
rsmk check   # validates and measures the layout without writing a PDF
```

```text
built jane_doe_resume.pdf (1 page, 42.8% fill)
```

`build` and `check` fail when the résumé runs past one page or a bullet wraps
onto a second line: each page's fill is listed, and a `note:` line names each
wrapped bullet. `--max-pages` raises the page limit.

To publish, tag the résumé repository. The scaffolded release workflow builds
the PDF and attaches it to a GitHub release:

```sh
git tag 1.0.0 && git push origin 1.0.0
```

## Commands

```text
rsmk build [CONTENT]       Compile to PDF and report layout (default: content.yaml)
  -t, --template <TPL>     Built-in template name or a .typ file
  -o, --output <PDF>       Output path (default: <name>_resume.pdf)
  --schema <FILE>          Validate against a custom JSON Schema
  --font-path <DIR>        Extra font directory (./fonts is picked up on its own)
  --max-pages <N>          Most pages allowed (default 1)
rsmk check [CONTENT]       Same checks as build, without writing a PDF

rsmk init [DEST]           Scaffold a résumé repository (default: .)
  -n, --name <NAME>        Candidate name
  -e, --eject              Also copy the template to templates/<name>.typ
  --no-git                 Skip git init, .gitignore and .gitattributes
  --no-workflows           Skip the GitHub Actions workflows
  -f, --force              Overwrite existing files

rsmk template list         List built-in templates
rsmk template eject <NAME> Copy a template to templates/<NAME>.typ to edit it

rsmk update                Install the latest release
  --check                  Only report whether a newer release exists
rsmk schema [-o FILE]      Print the JSON Schema for content.yaml

-q, --quiet                Print only errors (any command)
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
