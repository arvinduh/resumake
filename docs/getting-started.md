# Getting Started with Resumake

[← Documentation Hub](README.md) • **Getting Started** •
[Schema Reference](schema-guide.md) • [Layout Telemetry](telemetry-guide.md) •
[Architecture](architecture.md) • [Contributing](contributing.md)

---

This tutorial guides you through installing Resumake, scaffolding a structured
résumé project, customizing your content, verifying layout geometry, and cutting
releases.

---

## 1. Prerequisites

Resumake is distributed as a standalone, zero-dependency native binary.

- **Rust Toolchain (for source build):** Rust 1.75+ (`cargo`)
- **Prebuilt Binary (alternative):** Download from the
  [GitHub Releases](https://github.com/arvinduh/resumake/releases) page for
  Windows, macOS, or Linux.

---

## 2. Installation

### 1-Line Quick Install

```bash
# Linux & macOS
curl -fsSL https://raw.githubusercontent.com/arvinduh/resumake/main/installers/install.sh | sh
```

```powershell
# Windows (PowerShell)
irm https://raw.githubusercontent.com/arvinduh/resumake/main/installers/install.ps1 | iex
```

Verify the installation:

```bash
rsmk --help
```

### Updating rsmk

Once installed, upgrade in place from the latest GitHub release:

```bash
rsmk update             # download and install the latest release
rsmk update --check      # only report whether a newer release exists
rsmk update --force      # reinstall even if already on the latest version
```

`rsmk update` is the supported upgrade path: it fetches the release archive over
HTTPS, verifies it against the published `.sha256` checksum, and atomically
replaces the running binary. Re-running the install script (or the PowerShell
`irm … | iex` line on Windows) still works too.

---

## 3. Project Walkthrough

### Step 1: Scaffold a Project

Create a new directory and run the initialization wizard:

```bash
mkdir my-resume && cd my-resume
rsmk init --name "Jane Doe"
```

This scaffolds:

- `content.yaml` populated with standard sections (Education, Experience,
  Projects, Skills) and configured with the `Libertinus Serif` font.
- `.gitignore` (ignoring compiled PDFs and internal `.resumake/` caches).
- `.gitattributes` (`* text=auto eol=lf`).
- `.github/workflows/ci.yml` & `release.yml` with SHA-256 provenance headers.
- Git repository initialization (and optional GitHub repository creation via
  `gh`).

Pass `--no-git` for a content-only scaffold: no repository, and no CI/Release
workflows (they require a repo). Add the workflows later from the project
directory with `rsmk init --update`.

---

### Step 2: Live Recompilation & Geometry Feedback

Start the live watcher loop while editing `content.yaml`:

```bash
rsmk build --watch
```

Keep your PDF viewer open side-by-side. Every time you save `content.yaml`,
Resumake re-renders the document and evaluates single-page geometry in <100ms:

```text
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

---

### Step 3: Fast Dry-Run Validation

To validate YAML schema structure and check layout geometry in CI or pre-commit
without writing a PDF to disk:

```bash
rsmk build --check
```

---

### Step 4: Customizing Templates

To inspect or customize the underlying Typst template:

```bash
# List available templates
rsmk template list

# Eject the bundled classic template into your local workspace
rsmk template eject classic
```

This extracts `main.typ`, `tokens.typ`, `primitives.typ`, and `blocks/*.typ`
into `./templates/classic/`. You can compile using your customized local
template:

```bash
rsmk build --template ./templates/classic/main.typ
```

---

### Step 5: Cutting Releases

When you are ready to publish a new version of your résumé:

```bash
rsmk release
```

`rsmk release` performs 5 automated pre-flight checks:

1. Working tree is clean (no uncommitted changes).
2. Upstream branch is synced (no unpushed commits).
3. `meta.version` in `content.yaml` is valid SemVer and strictly newer than
   existing git tags.
4. Pre-flight schema validation and layout check passes.
5. Atomically creates tag `v<version>` and pushes to `origin`.

Pushing the tag triggers your repository's GitHub Actions release workflow to
compile and attach the PDF to a new GitHub Release.

---

## 4. Next Steps

- **[YAML Schema Reference](schema-guide.md):** Customize sections, contact
  links, and theme tokens.
- **[Layout Telemetry Guide](telemetry-guide.md):** Understand the math behind
  single-page geometry optimization.
- **[System Architecture](architecture.md):** Deep dive into the compiler and
  pipeline design.
