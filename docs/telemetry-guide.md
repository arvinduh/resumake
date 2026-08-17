# Layout Telemetry & Geometry Guide

[← Documentation Hub](README.md) • [Getting Started](getting-started.md) • [Schema Reference](schema-guide.md) • **Layout Telemetry** • [Architecture](architecture.md) • [Contributing](contributing.md)

---

One of Resumake's signature capabilities is **Layout Telemetry**. Rather than relying on guesswork to fit a résumé onto a single page, Resumake programmatically measures physical layout geometry and provides strict feedback.

---

## 1. Why Layout Telemetry Matters

Recruiters and hiring managers spend an average of 6–8 seconds reviewing a résumé. Common formatting errors include:
1. **Accidental Multi-Page Spilling:** A single line overflowing onto a second page.
2. **Poor Vertical Utilization:** Large awkward white gaps at the bottom (<80% fill).
3. **Orphaned Words (Bad Line Wraps):** Bullet points wrapping onto a new line with only 1–3 trailing words, wasting valuable vertical real estate.

Resumake evaluates these factors automatically on every build.

---

## 2. Telemetry Metrics

### A. Page Count Enforcement (`[PASS 1/1]`)
* Resumake parses the compiled PDF document structure.
* If page count $> 1$, the build triggers a `LAYOUT_OVERFLOW` error and halts with a failure code, preventing accidental multi-page exports.

### B. Vertical Fill Percentage (`[OPTIMAL]`)
Measures the ratio of used vertical content height to total printable page height.

$$\text{Fill Percentage} = \frac{\text{Used Height}}{\text{Available Page Height}} \times 100\%$$

| Fill Range | Status Badge | Meaning |
| :--- | :--- | :--- |
| **90.0% – 98.0%** | `[OPTIMAL]` | Perfect page balance. Clean margins without overcrowding. |
| **80.0% – 89.9%** | `[ACCEPTABLE]` | Good layout, slight spare space at the bottom. |
| **< 80.0%** | `[UNDERFILL]` | Under-utilized space; consider adding highlights or expanding details. |
| **> 98.5%** | `[TIGHT]` | Approaching risk of overflowing onto page 2. |

### C. Bullet Wrap Detection (`[0 WRAPS]`)
Analyzes bullet point text lengths against the column width:
* Detects lines that wrap into a second line with only 1–4 trailing words (typographic "widows/orphans").
* Helps you rephrase bullet points to be punchy single-liners or full two-liners.

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

If your telemetry reports an `[UNDERFILL]` or `[TIGHT]` warning:
1. **Adjust Margins:** Modify `margin_y` (e.g. from `0.45in` to `0.40in` or `0.50in`) in `content.yaml`.
2. **Tune Typography:** Change `font_size` (e.g. `9.5pt` $\leftrightarrow$ `10pt`) or `line_height` (e.g. `0.65em` $\leftrightarrow$ `0.70em`).
3. **Rephrase Bullets:** Tighten phrasing on bullets flagged with line wraps to reclaim 1–2 vertical lines.
