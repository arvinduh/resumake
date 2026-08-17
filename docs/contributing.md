# Contributing to Resumake

[← Documentation Hub](README.md) • [Getting Started](getting-started.md) • [Schema Reference](schema-guide.md) • [Layout Telemetry](telemetry-guide.md) • [Architecture](architecture.md) • **Contributing**

---

Thank you for contributing to Resumake! We welcome bug reports, documentation improvements, and feature contributions.

---

## 1. Code of Conduct

* Treat all contributors and users with respect.
* Focus on constructive code reviews and clear, concise discussions.

---

## 2. Setting Up Your Development Environment

### Toolchain Installation by OS

| Tool | Windows (`winget` / `cargo`) | macOS (`brew`) | Linux (`apt` / `curl`) |
| :--- | :--- | :--- | :--- |
| **Rust & Cargo** | `winget install Rustlang.Rustup` | `brew install rustup-init && rustup-init` | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| **Typst Compiler** | `winget install Typst.Typst` | `brew install typst` | `cargo install --locked typst-cli` |
| **Markdown Linter** | `winget install DavidAnson.markdownlint-cli2` | `brew install markdownlint-cli2` | `npm install -g markdownlint-cli2` |
| **Jujutsu (Optional)** | `winget install jj-vcs.jj` | `brew install jj` | `cargo install --locked jj-cli` |

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
# 1. Format verification
cargo fmt --check

# 2. Strict linter (zero warnings)
cargo clippy --all-targets -- -D warnings

# 3. Comprehensive test suite
cargo test --all-targets

# 4. API Documentation build
cargo doc --no-deps
```

---

## 4. Development Workflow

1. **Create a Feature Branch:**
   ```bash
   git checkout -b feat/your-feature-name
   ```

2. **Write Focused, Modular Code:**
   * Keep functions pure where possible.
   * Add unit tests for new models, schema features, or telemetry rules.
   * Maintain zero compiler warnings under `-D warnings`.

3. **Follow Conventional Commits:**
   * `feat: add custom font family support`
   * `fix: handle edge-case margin overflow`
   * `docs: update telemetry documentation`
   * `ci: cache cargo dependencies in github actions`

4. **Push and Open a Pull Request:**
   ```bash
   git push -u origin feat/your-feature-name
   ```

---

## 5. Pull Request Checklist

Before marking your PR as ready for review:
- [ ] Code passes `cargo fmt --check`.
- [ ] Code passes `cargo clippy --all-targets -- -D warnings`.
- [ ] All 20+ unit and integration tests pass via `cargo test --all-targets`.
- [ ] Documentation comments added for new public functions or structs.
- [ ] Commit history is clean and follows conventional commit format.
