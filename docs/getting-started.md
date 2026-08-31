# Getting Started with Resumake

[← Documentation Hub](README.md) • **Getting Started** •
[Schema Reference](schema-guide.md) • [Layout Telemetry](telemetry-guide.md) •
[Architecture](architecture.md) • [Contributing](contributing.md)

---

This tutorial guides you through installing Resumake, scaffolding a structured
résumé project, customizing your content, and compiling your first single-page
PDF with layout telemetry.

---

## 1. Prerequisites

Resumake is distributed as a standalone, zero-dependency native binary.

- **Rust Toolchain (for source build):** Rust 1.75+ (`cargo`)
- **Prebuilt Binary (alternative):** Download from the
  [GitHub Releases](https://github.com/arvinduh/resumake/releases) page for
  Windows, macOS, or Linux.

---

## 2. Installation

### From Source (Cargo)

```bash
# Clone the repository
git clone https://github.com/arvinduh/resumake.git
cd resumake

# Install binary to ~/.cargo/bin
cargo install --path .
```

Verify the installation:

```bash
rsmk --help
```

---

## 3. Project Walkthrough

### Step 1: Scaffold a Project

Create a new directory and scaffold the initial configuration and template:

```bash
mkdir my-resume
cd my-resume
rsmk init --name "Jane Doe"
```

This generates `content.yaml` populated with standard sections (Education,
Technical Skills, Experience, and Projects).

---

### Step 2: Compile to PDF

Compile your résumé to a high-fidelity PDF:

```bash
rsmk build
```

Resumake compiles the Typst template and performs real-time layout telemetry
analysis, outputting a terminal summary:

```text
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

### Picking a Template

`rsmk build`/`check`/`watch` render `content.yaml` through a named, built-in
layout — the same YAML works under any of them, since the content model has no
idea which template is drawing it. Today the registry only ships `classic` (the
layout above), which is also the default, so this is a no-op unless you pass it
explicitly:

```bash
rsmk build --template classic
```

More built-in layouts (e.g. a two-column sidebar résumé) can be added to the
registry over time without any changes to your `content.yaml` — see
[System Architecture](architecture.md#4-key-design-decisions). If you'd rather
supply your own Typst file entirely, bypassing the registry, use
`--source <path.typ>` instead.

---

### Step 3: Fast Dry-Run Validation

If you want to validate your YAML schema and check layout geometry without
writing a PDF to disk, use the `check` command:

```bash
rsmk check
```

---

### Step 4: Live Auto-Recompilation (Watch Mode)

Enable real-time recompilation whenever you modify `content.yaml`:

```bash
rsmk watch
```

Keep your PDF viewer open side-by-side to see instant visual updates in under
100 milliseconds.

---

## 4. Next Steps

- **[YAML Schema Reference](schema-guide.md):** Customize sections, contact
  links, and theme tokens.
- **[Layout Telemetry Guide](telemetry-guide.md):** Understand the math behind
  single-page geometry optimization.
