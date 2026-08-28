# Resumake Documentation Hub

Welcome to the **Resumake** documentation suite. This hub provides comprehensive
guides, schema specifications, architecture deep dives, and contribution
instructions for the Resumake résumé compiler.

---

## Documentation Sitemap

| Guide                                            | Description                                                                    | Target Audience          |
| :----------------------------------------------- | :----------------------------------------------------------------------------- | :----------------------- |
| **[Getting Started](getting-started.md)**        | Step-by-step tutorial from installation to your first compiled PDF.            | New Users                |
| **[YAML Schema Reference](schema-guide.md)**     | Complete specification of all directives, metadata, and block sections.        | Résumé Authors           |
| **[Layout Telemetry Guide](telemetry-guide.md)** | Learn how strict 1-page geometry, fill percentage, and wrap checks work.       | Authors & Designers      |
| **[System Architecture](architecture.md)**       | Deep dive into the native Rust engine, embedded Typst compiler, and pipeline.  | Developers & Integrators |
| **[Contributing Guide](contributing.md)**        | Code standards, local test suite, pre-commit setup, and Pull Request workflow. | Contributors             |

---

## Quick Navigation

```text
resumake/
├── README.md                 # Project Overview & Quickstart
└── docs/
    ├── README.md             # Documentation Hub (You are here)
    ├── getting-started.md    # Installation & First Build Tutorial
    ├── schema-guide.md       # Full Schema Directives & Blocks
    ├── telemetry-guide.md    # Layout Geometry & Optimization
    ├── architecture.md       # Rust Engine Architecture
    └── contributing.md       # Contribution & PR Workflow
```

---

## About Resumake

Resumake is a high-performance native Rust résumé compiler designed to solve the
common pitfalls of résumé formatting:

1. **Zero Layout Regressions:** Automatically detects overflow and page spilling
   before you submit applications.
2. **Modular Typst Engine:** Compiles pixel-perfect PDFs in under 100
   milliseconds without external dependencies.
3. **Structured Content:** Separates your career data (`content.yaml`) from
   presentation styling (`main.typ` & design tokens).

---

[← Return to Repository Home](../README.md)
