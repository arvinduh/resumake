# Layout Telemetry & Geometry Guide

[← Documentation Hub](README.md) • [Getting Started](getting-started.md) •
[Schema Reference](schema-guide.md) • **Layout Telemetry** •
[Architecture](architecture.md) • [Contributing](contributing.md)

---

One of Resumake's signature capabilities is **Layout Telemetry**. Rather than
relying on guesswork to fit a résumé onto a single page, Resumake
programmatically measures physical layout geometry and provides strict feedback.

---

## 1. Why Layout Telemetry Matters

Recruiters and hiring managers spend an average of 6–8 seconds reviewing a
résumé. Common formatting errors include:

1. **Accidental Multi-Page Spilling:** A single line overflowing onto a second
   page.
2. **Poor Vertical Utilization:** Large awkward white gaps at the bottom (below
   80% fill).
3. **Orphaned Words (Bad Line Wraps):** Bullet points wrapping onto a new line
   with only 1–3 trailing words, wasting valuable vertical real estate.

Resumake evaluates these factors automatically on every build.

---

## 2. Telemetry Metrics

### A. Page Count Enforcement (`[PASS 1/1]`)

- Resumake queries the compiled document's `<pageinfo>` metadata for the final
  page count.
- If page count is greater than 1, `rsmk build`/`check` exits with a
  non-zero status and a failure message, preventing accidental multi-page
  exports.

### B. Vertical Fill Percentage (`[OPTIMAL]`)

Measures the ratio of used vertical content height to total printable page
height:

$$\text{Fill \%} = \frac{\text{Used Height}}{\text{Available Height}} \times 100$$

| Fill Range        | Status Badge | Meaning                                  |
| :---------------- | :----------- | :--------------------------------------- |
| **90.0% – 99.0%** | `[OPTIMAL]`  | Perfect page balance.                    |
| **> 99.0%**       | `[OVERFLOW]` | Content is at or beyond the page edge.   |
| **< 90.0%**       | `[ROOM]`     | Spare space remains; consider expanding. |

### C. Bullet Wrap Detection (`[0 WRAPS]`)

Analyzes bullet point text width against the column width, per bullet:

- A bullet with fill greater than 100% wrapped onto a second line and is
  reported as a **wrap**.
- A bullet with fill under 86% is reported as an **underfill** — likely too
  short to read as a complete accomplishment statement.
- Only items probed with `kind: "bullet"` participate in wrap/underfill
  detection; label rows rendered via `line-item` (skills, education metadata,
  references) are excluded by design.

---

## 3. Sample Terminal Telemetry Output

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

## 4. How to Optimize Your Layout

If your telemetry reports spare room or a wrap warning:

1. **Adjust Margins:** Modify `meta.theme.margin` (e.g. from `"0.5in"` to
   `"0.45in"`) in `content.yaml`.
2. **Tune Typography:** Change `meta.theme.font_size` (e.g. `"11.5pt"` to
   `"11pt"`).
3. **Rephrase Bullets:** Tighten phrasing on bullets flagged with line wraps to
   reclaim 1–2 vertical lines.
