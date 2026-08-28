# Contributing to Resumake

[← Documentation Hub](README.md) • [Getting Started](getting-started.md) •
[Schema Reference](schema-guide.md) • [Layout Telemetry](telemetry-guide.md) •
[Architecture](architecture.md) • **Contributing**

---

Thank you for contributing to Resumake! We welcome bug reports, documentation
improvements, and feature contributions.

---

## 1. Code of Conduct

- Treat all contributors and users with respect.
- Focus on constructive code reviews and clear, concise discussions.

---

## 2. Setting Up Your Development Environment

### Toolchain Installation by OS

| Tool                   | Windows (`winget` / `cargo`)                                    | macOS (`brew`)                                                  | Linux (`apt` / `curl`)                                                                                                    |
| :--------------------- | :-------------------------------------------------------------- | :-------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------ |
| **Formality (`fml`)**  | `cargo install --git https://github.com/arvinduh/formality fml` | `cargo install --git https://github.com/arvinduh/formality fml` | `curl -sSL https://github.com/arvinduh/formality/releases/latest/download/fml-x86_64-unknown-linux-gnu.tar.gz \| tar -xz` |
| **Rust & Cargo**       | `winget install Rustlang.Rustup`                                | `brew install rustup-init && rustup-init`                       | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh`                                                         |
| **Typst Compiler**     | `winget install Typst.Typst`                                    | `brew install typst`                                            | `cargo install --locked typst-cli`                                                                                        |
| **Jujutsu (Optional)** | `winget install jj-vcs.jj`                                      | `brew install jj`                                               | `cargo install --locked jj-cli`                                                                                           |

Once `fml` is installed, run `fml doctor` or `fml install` to automatically
detect and configure all formatters and linters (Prettier, Markdownlint,
Rustfmt, Clippy, Taplo, Typstyle).

### Clone & Hook Setup

```bash
# Clone the repository
git clone https://github.com/<your-username>/resumake.git
cd resumake

# Enable pre-commit verification hooks
git config core.hooksPath .githooks
```

---

## 3. Local Verification Suite

Before submitting a Pull Request, verify that all local checks pass:

```bash
# 1. Auto-verify toolchains
fml doctor

# 2. Check native config synchronization
fml sync --check

# 3. Polyglot format verification (Rust, Typst, YAML, TOML, JSON, Markdown)
fml fmt --check

# 4. Strict polyglot linter (Clippy, Markdownlint, Typstyle)
fml lint

# 5. Comprehensive test suite
cargo test --all-targets

# 6. API Documentation build
cargo doc --no-deps
```

To automatically format all staged files or auto-fix linting issues before
committing:

```bash
fml fmt --staged
fml lint --fix
```

---

## 4. Development Workflow

1. **Create a Feature Branch:**

   ```bash
   git checkout -b feat/your-feature-name
   ```

2. **Write Focused, Modular Code:**
   - Keep functions pure where possible.
   - Add unit tests for new models, schema features, or telemetry rules.
   - Maintain zero compiler warnings under `-D warnings`.

3. **Follow Conventional Commits:**
   - `feat: add custom font family support`
   - `fix: handle edge-case margin overflow`
   - `docs: update telemetry documentation`
   - `ci: cache cargo dependencies in github actions`

4. **Push and Open a Pull Request:**

   ```bash
   git push -u origin feat/your-feature-name
   ```

---

## 5. Adding a New Template

Resumake's data model (`src/models.rs`) and rendering (the Typst files under
`src/embedded/templates/`) are fully decoupled: any template renders the same
`ResumeDocument` YAML, so adding a new visual layout never requires touching the
schema. To add one:

1. **Create the template tree** under `src/embedded/templates/<name>/`,
   mirroring the existing `classic/` layout:

   ```text
   src/embedded/templates/<name>/
     main.typ         # entry point — reads `content`, dispatches per section
     tokens.typ        # typography/spacing constants
     primitives.typ    # shared layout helpers (section(), bullets(), guard()...)
     blocks/*.typ       # one render function per section type
   ```

   You don't have to reimplement every block from scratch — a template is free
   to `#import` primitives or block renderers from `classic/` if its layout only
   needs to change `main.typ`'s composition (e.g. a two-column grid), and only
   fork the blocks whose visual treatment actually differs.

2. **Honor the telemetry contract.** `resumake build`/`check` measure layout by
   querying Typst metadata, not by parsing the PDF, so your `main.typ` must
   still:
   - Emit a `<pageinfo>` metadata tag with `pages`, `y`, `margin`, `page_w`,
     `page_h` (see the bottom of `classic/main.typ`).
   - Route bullet-like content through the `guard()` primitive so it emits
     `<bulletinfo>` tags (used for wrap/underfill detection).

   A template that skips this still compiles fine, but `resumake build`/`check`
   will report degraded or missing telemetry for it.

3. **Register it** in `src/engine.rs`: add an `EmbeddedTemplate` constant
   (following `CLASSIC_TEMPLATE`, with one `include_str!` per file) and append
   it to `TEMPLATE_REGISTRY`. That's the only Rust code that needs to change —
   `--template <name>` picks it up automatically, and `resolve_template` handles
   extraction to `.resumake/<name>/`.

4. **Add coverage:** a `resolve_template` extraction test in `src/engine.rs`
   (mirroring `test_resolve_template_extracts_modular_components`) and an
   end-to-end `resumake build --template <name>` case in `tests/cli.rs`.

See
[System Architecture § Key Design Decisions](architecture.md#4-key-design-decisions)
for the registry's design rationale.

---

## 6. Pull Request Checklist

Before marking your PR as ready for review:

- [ ] Configs are synchronized via `fml sync --check`.
- [ ] Code passes `fml fmt --check`.
- [ ] Code passes `fml lint`.
- [ ] All unit and integration tests pass via `cargo test --all-targets`.
- [ ] Documentation comments added for new public functions or structs.
- [ ] Commit history is clean and follows conventional commit format.
