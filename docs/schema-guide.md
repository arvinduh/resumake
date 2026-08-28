# YAML Schema Reference Guide

[← Documentation Hub](README.md) • [Getting Started](getting-started.md) •
**Schema Reference** • [Layout Telemetry](telemetry-guide.md) •
[Architecture](architecture.md) • [Contributing](contributing.md)

---

Resumake uses a strictly validated YAML schema to define résumé structure,
contact metadata, theme tokens, and modular content sections. This document
details every configurable property as actually implemented in
[`src/models.rs`](../src/models.rs) and enforced by
[`resumake schema`](../src/schema.rs).

---

## 1. Document Structure Overview

A `content.yaml` file has exactly two root keys: `meta` and `sections`.

```yaml
meta:
  # Candidate identity, contact links, ATS keywords, and theme tokens

sections:
  # Ordered list of modular résumé sections (education, experience, ...)
```

There is no separate `directives`, `tokens`, or `blocks` root key — theme tokens
live under `meta.theme`, and every résumé section (experience, education,
skills, and so on) is one entry in the `sections` list.

Unrecognized fields are rejected, not silently ignored: every field listed below
is closed to typos and renames by the schema itself
(`"additionalProperties": false`), so `resumake check`/`build` fails with the
offending field name instead of quietly dropping its value. This applies to
`meta` and to a section's strongly-typed shorthand keys (`education`,
`experience`, ...); the generic `items:` form used alongside an explicit `type:`
is intentionally open-ended, so a typo inside `items:` is not caught.

---

## 2. Metadata (`meta`)

| Field         | Type            | Required | Description                                             |
| :------------ | :-------------- | :------- | :------------------------------------------------------ |
| `name`        | `string`        | Yes      | Full display name of the candidate.                     |
| `version`     | `string`        | Yes      | Semantic milestone version, e.g. `"1.0.0"`.             |
| `title`       | `string`        | No       | Headline shown under the name, e.g. `"Staff Engineer"`. |
| `description` | `string`        | No       | Short bio or document summary.                          |
| `badge`       | `string`        | No       | Availability / clearance / work-auth badge.             |
| `keywords`    | `array[string]` | No       | ATS keywords embedded in PDF metadata.                  |
| `contact`     | `array`         | Yes      | Contact items — see below.                              |
| `theme`       | `object`        | No       | Visual styling tokens — see below.                      |

Each `contact` entry is either a plain string or a structured object:

```yaml
meta:
  contact:
    - "(555) 019-2834"
    - name: "janedoe@example.com"
      link: "mailto:janedoe@example.com"
    - name: "linkedin.com/in/janedoe"
      link: "https://linkedin.com/in/janedoe"
```

`name` accepts the aliases `label` and `value`; `link` accepts the alias `url`.

---

## 3. Theme Tokens (`meta.theme`)

All fields are optional and fall back to sensible defaults.

| Field            | Type     | Default             | Description                      |
| :--------------- | :------- | :------------------ | :------------------------------- |
| `font_family`    | `string` | `"Linux Libertine"` | Body font family.                |
| `font_size`      | `string` | `"11.5pt"`          | Base body typography size.       |
| `name_size`      | `string` | `"25pt"`            | Candidate name heading size.     |
| `section_size`   | `string` | `"13pt"`            | Section header font size.        |
| `org_size`       | `string` | `"12pt"`            | Organization / institution size. |
| `rule_thickness` | `string` | `"0.5pt"`           | Section divider rule thickness.  |
| `paper_size`     | `string` | `"us-letter"`       | `"us-letter"` or `"a4"`.         |
| `margin`         | `string` | `"0.5in"`           | Uniform page margin.             |
| `ink_color`      | `string` | `"#111111"`         | Primary text color (hex).        |
| `accent_color`   | `string` | `"#2a2a2a"`         | Rule / accent color (hex).       |
| `muted_color`    | `string` | `"#444444"`         | Secondary metadata color (hex).  |

```yaml
meta:
  theme:
    font_family: "Linux Libertine"
    paper_size: "us-letter"
    margin: "0.5in"
    accent_color: "#2a2a2a"
```

---

## 4. Sections (`sections`)

Each entry in `sections` needs a `title`. The renderer picks a block type either
from an explicit `type` field or from the presence of a matching shorthand key
(`education`, `experience`, `skills`, and so on). Content can be supplied either
under that shorthand key or under a generic `items` key alongside `type`; both
forms below are equivalent:

```yaml
- title: "Education"
  type: "education"
  items: [...]

- title: "Education"
  education: [...]
```

### A. Education (`type: "education"`)

Accepts a single object or a list of objects.

```yaml
- title: "Education"
  type: "education"
  items:
    - institution: "University Name"
      location: "City, State"
      degree: "B.S. in Computer Science"
      gpa: "3.90 / 4.00"
      dates: "Sep 2018 - Jun 2022"
      honors: "Dean's Honor List"
      coursework: "Data Structures, Algorithms, Operating Systems"
```

`institution` accepts the aliases `school`/`university`; `dates` accepts `date`.
`honors` and `coursework` accept either a comma-separated string or a YAML list.
`thesis` accepts a string or `{title, advisor}`.

### B. Skills (`type: "skills"`)

Accepts either a category → skills dictionary or a list of category objects.

```yaml
- title: "Technical Skills"
  type: "skills"
  items:
    Languages: "Rust, Python, C++, TypeScript, SQL, Bash"
    Frameworks & Tools: "Git, Docker, Kubernetes, GitHub Actions"
```

### C. Experience (`type: "experience"`)

Each organization holds one or more `roles` (for promotion ladders), or uses the
single-role shorthand fields (`title`, `dates`, `bullets`) directly.

```yaml
- title: "Experience"
  type: "experience"
  items:
    - org: "Organization Name"
      location: "City, State"
      roles:
        - title: "Software Engineer"
          dates: "Jul 2022 - Present"
          bullets:
            - "Designed an automated data validation pipeline."
```

`org` accepts the aliases `company`/`organization`; a role's `title` accepts
`role`/`position`, and its `dates` accepts `date`.

### D. Projects (`type: "projects"`)

```yaml
- title: "Projects"
  type: "projects"
  items:
    - name: "Open Source Project Name"
      stack: "Rust / WebAssembly"
      date: "Jan 2023"
      link: "https://github.com/janedoe/project"
      bullets:
        - "Built a high-performance CLI for validating data schemas."
```

### E. Other block types

| `type`                                 | Item fields                                         | Notes                                             |
| :------------------------------------- | :-------------------------------------------------- | :------------------------------------------------ |
| `publications`                         | `title`, `authors`, `venue`, `year`, `url`, `notes` | `venue` aliases `journal`/`conference`.           |
| `certifications`, `awards`, `speaking` | `name`/`title`, `issuer`/`event`, `date`, ...       | All three render via the shared split-line block. |
| `references` / `columns`               | `name`, `role`, `org`, `contact`                    | Renders as a balanced 2-column grid.              |
| `lines`                                | free-form `key: value` map                          | Renders as bolded label rows.                     |
| `bullets`                              | list of strings                                     | Plain bullet list, no heading row.                |
| `text`                                 | a single string                                     | Freeform prose paragraph.                         |

---

## 5. Exporting the Canonical JSON Schema

Print the live schema — derived directly from the Rust types, so it can never
drift from this document — to stdout:

```bash
resumake schema
```

Or write it to a file:

```bash
resumake schema --export resume.schema.json
```

This is not a file you need to generate yourself for day-to-day use: the
`# yaml-language-server: $schema=...` comment `resumake init` writes at the top
of `content.yaml` already points at the versioned copy published with each
GitHub Release, so VS Code and JetBrains YAML plugins pick it up automatically.
`resumake schema --export` is for local tooling that wants a schema file
directly (custom linters, a different editor plugin, CI in a downstream
project).

---

## 6. Schema Stability & Versioning

The schema is derived at compile time from `src/models.rs`; it is not a
separately-versioned artifact and not committed to the repository, so it can
never silently drift from what the running binary actually accepts (see
[System Architecture](architecture.md) for why). Two things follow from that:

- **Compatibility tracks the `resumake` crate version, via semver.** PATCH and
  MINOR releases only ever add optional fields or aliases — they never rename or
  remove one required by an earlier release in the same major line. Only a MAJOR
  version bump may break an existing `content.yaml`. In practice: if
  `resumake check` passes on one release in a major line, it will keep passing
  on later releases in that same major line.
- **A schema published under one release stays exactly as it was.** Each GitHub
  Release publishes its own `resume.schema.json` as a release asset
  (`.github/workflows/release.yml`), and `resumake init` points a freshly
  scaffolded file at that exact tagged copy — not a branch-tracking URL that
  would silently re-validate old files against a newer schema the moment `main`
  changes.

If you're pinning a `resumake` version for a team or CI pipeline, pin the same
major version everyone's editors resolve `$schema` against, and treat a major
version bump as the point to re-read this guide.
