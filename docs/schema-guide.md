# YAML Schema Reference Guide

[← Documentation Hub](README.md) • [Getting Started](getting-started.md) • **Schema Reference** • [Layout Telemetry](telemetry-guide.md) • [Architecture](architecture.md) • [Contributing](contributing.md)

---

Resumake uses a strictly validated YAML schema to define résumé structure, layout directives, design tokens, and career blocks. This document details every configurable property.

---

## 1. Document Structure Overview

A complete `content.yaml` file consists of four primary root sections:

```yaml
version: "1.0.0"

directives:
  # Global layout, margin, and typography rules

tokens:
  # Color palette, font choices, and spacing scales

metadata:
  # Candidate contact info, title, and social links

blocks:
  # Section data (experience, education, projects, skills, etc.)
```

---

## 2. Directives (`directives`)

Directives govern the global layout behavior and section ordering.

| Field | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `order` | `array[string]` | `["experience", "education", ...]` | Explicit order of sections on the résumé. |
| `paper` | `string` | `"us-letter"` | Page dimension: `"us-letter"` (8.5x11in) or `"a4"`. |
| `margin_x` | `string` | `"0.5in"` | Left and right page margins. |
| `margin_y` | `string` | `"0.45in"` | Top and bottom page margins. |
| `font_size` | `string` | `"9.5pt"` | Base body typography size. |
| `heading_size`| `string` | `"11.5pt"` | Section header font size. |
| `title_size` | `string` | `"20pt"` | Candidate name heading size. |
| `line_height`| `string` | `"0.65em"` | Paragraph leading / vertical spacing. |

---

## 3. Design Tokens (`tokens`)

Tokens customize the visual aesthetics of the generated PDF:

```yaml
tokens:
  primary_color: "#1e3a8a"    # Section headings and accent lines
  secondary_color: "#475569"  # Subtitles and dates
  text_color: "#0f172a"       # Body copy
  link_color: "#2563eb"       # Hyperlinks
  font_family: "Liberation Sans"
```

---

## 4. Metadata (`metadata`)

Metadata defines personal and contact details rendered at the top of the résumé:

```yaml
metadata:
  name: "Jane Doe"
  title: "Principal Systems Engineer"
  email: "janedoe@example.com"
  phone: "+1 (555) 019-2834"
  location: "San Francisco, CA"
  website: "https://janedoe.dev"
  github: "https://github.com/janedoe"
  linkedin: "https://linkedin.com/in/janedoe"
```

---

## 5. Blocks (`blocks`)

Blocks hold the content for individual sections. Resumake supports six modular block types:

### A. Experience (`blocks.experience`)
```yaml
experience:
  - company: "Anthropic"
    role: "Staff Infrastructure Engineer"
    location: "San Francisco, CA"
    start_date: "2023"
    end_date: "Present"
    highlights:
      - "Architected low-latency distributed cluster scheduler handling 50k+ nodes."
      - "Reduced GPU synchronization overhead by 34% through zero-copy ring buffers."
```

### B. Education (`blocks.education`)
```yaml
education:
  - institution: "University of California, Berkeley"
    degree: "B.S. in Electrical Engineering and Computer Sciences"
    location: "Berkeley, CA"
    start_date: "2016"
    end_date: "2020"
    gpa: "3.92"
```

### C. Projects (`blocks.projects`)
```yaml
projects:
  - name: "Resumake"
    url: "https://github.com/arvinduh/resumake"
    role: "Author & Lead Developer"
    highlights:
      - "Engineered headless Rust compiler generating single-page PDFs in <100ms."
```

### D. Skills (`blocks.skills`)
```yaml
skills:
  - category: "Languages"
    items: ["Rust", "Go", "C++", "Python", "SQL"]
  - category: "Infrastructure"
    items: ["Kubernetes", "Linux eBPF", "Docker", "Terraform"]
```

---

## 6. Exporting Canonical JSON Schema

You can export the official JSON Schema (Draft 2020-12) for IDE auto-completion and linting:

```bash
resumake schema --export
```

This generates `schema/resume.schema.json`, which can be referenced in VSCode or JetBrains YAML plugins.
