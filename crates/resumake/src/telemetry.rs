//! Document layout telemetry, page overflow analysis, and bullet wrapping.
//!
//! Owns extracting layout geometry probes (`<pageinfo>`, `<bulletinfo>`)
//! from compiled Typst documents and calculating page fill percentages.

use schemars;
use serde;
use serde_json;
use thiserror;
use typst::foundations;
use typst::introspection;
use typst::utils;
use typst_layout;

/// Errors originating from layout telemetry evaluation.
#[derive(Debug, thiserror::Error)]
pub enum Error {
  /// Deserialization failure for probed telemetry JSON.
  #[error("failed to deserialize telemetry probe: {0}")]
  Deserialization(#[from] serde_json::Error),
}

/// Page geometry metrics probed from `<pageinfo>`.
#[derive(serde::Deserialize)]
struct PageInfo {
  /// Total pages in document.
  #[serde(default = "default_pages")]
  pages: usize,
  /// Margin size in points.
  #[serde(default = "default_margin")]
  margin: f64,
  /// Page height in points.
  #[serde(default = "default_height")]
  page_h: f64,
  /// Y cursor coordinate at document end in points.
  #[serde(default)]
  y: f64,
}

/// Default page count fallback.
const fn default_pages() -> usize {
  1
}

/// Default page margin fallback in points.
const fn default_margin() -> f64 {
  36.0
}

/// Default page height fallback in points.
const fn default_height() -> f64 {
  792.0
}

impl Default for PageInfo {
  fn default() -> Self {
    Self {
      pages: default_pages(),
      margin: default_margin(),
      page_h: default_height(),
      y: 0.0,
    }
  }
}

/// Default trailing-line fill threshold (percentage) below which a wrapped
/// bullet is considered an orphan tail wrap.
pub const ORPHAN_TAIL_PCT: f64 = 20.0;

/// Text item probe queried from `<bulletinfo>`.
#[derive(Debug, serde::Deserialize)]
pub struct BulletInfo {
  /// Content item kind (e.g. "bullet").
  #[serde(default)]
  pub kind: String,
  /// Measured single-line width fill percentage.
  #[serde(default)]
  pub fill: f64,
  /// Explicit wrap flag emitted by templates.
  #[serde(default)]
  pub wrapped: Option<bool>,
  /// Total rendered line count for this item.
  #[serde(default)]
  pub lines: Option<usize>,
  /// Rendered final-line width fill percentage for wrapped items.
  #[serde(default)]
  pub tail_fill: Option<f64>,
  /// Text content of the bullet item.
  #[serde(default)]
  pub text: String,
}

impl BulletInfo {
  /// Returns `true` when this item wrapped past a single line.
  #[must_use]
  pub fn is_wrapped(&self) -> bool {
    self
      .wrapped
      .unwrap_or_else(|| self.lines.is_some_and(|l| l > 1) || self.fill > 100.0)
  }

  /// Rendered line count for this item (falls back to wrap status when omitted
  /// by custom templates).
  #[must_use]
  pub fn line_count(&self) -> usize {
    self
      .lines
      .unwrap_or_else(|| if self.is_wrapped() { 2 } else { 1 })
  }

  /// Returns `true` when a wrapped bullet ends with a short trailing line under
  /// [`ORPHAN_TAIL_PCT`].
  #[must_use]
  pub fn is_orphan_tail(&self) -> bool {
    self.kind == "bullet"
      && self.is_wrapped()
      && self
        .tail_fill
        .is_some_and(|pct| pct > 0.0 && pct < ORPHAN_TAIL_PCT)
  }

  /// Calculates where a wrapped line breaks onto a second line.
  #[must_use]
  pub fn wrap_boundary(&self) -> Option<WrapBoundary<'_>> {
    calculate_wrap_boundary(&self.text, self.fill)
  }
}

/// Calculated character and text boundaries for a wrapped line.
#[derive(Debug)]
pub struct WrapBoundary<'a> {
  /// Character index where line 1 ends.
  pub fit_char_idx: usize,
  /// Text overflowing onto the second line.
  pub spill_text: &'a str,
}

/// Calculates where a wrapped line breaks onto a second line.
#[must_use]
pub fn calculate_wrap_boundary(
  text: &str,
  fill: f64,
) -> Option<WrapBoundary<'_>> {
  if fill <= 100.0 || text.is_empty() {
    return None;
  }

  let total_chars = text.chars().count();
  if total_chars == 0 {
    return None;
  }

  let fit_ratio = (100.0 / fill).clamp(0.0, 1.0);
  #[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "bounded char count"
  )]
  let estimated_char_cutoff = (total_chars as f64 * fit_ratio).floor() as usize;

  let mut byte_cutoff = text
    .char_indices()
    .nth(estimated_char_cutoff)
    .map_or(text.len(), |(idx, _)| idx);

  if let Some(ws_idx) = text[..byte_cutoff].rfind(char::is_whitespace)
    && ws_idx > 0
  {
    byte_cutoff = ws_idx;
  }

  let fit_text = text[..byte_cutoff].trim_end();
  let spill_text = text[byte_cutoff..].trim_start();
  let fit_char_idx = fit_text.chars().count();

  Some(WrapBoundary {
    fit_char_idx,
    spill_text,
  })
}

/// Comprehensive layout evaluation report.
#[derive(Debug)]
pub struct TelemetryReport {
  /// Total number of pages.
  pub page_count: usize,
  /// Fill percentage for each page.
  pub page_fills: Vec<f64>,
  /// Final page fill percentage.
  pub fill_pct: f64,
  /// Spare vertical space on final page in inches.
  pub spare_in: f64,
  /// List of items that wrapped across lines.
  pub line_wraps: Vec<BulletInfo>,
  /// Requested `theme.font_family` names missing from loaded fonts.
  pub missing_fonts: Vec<String>,
}

impl TelemetryReport {
  /// Returns `true` if document has 1 page.
  #[must_use]
  pub fn is_single_page(&self) -> bool {
    self.page_count == 1
  }

  /// Returns wrapped items whose rendered line count exceeds `max_lines`.
  #[must_use]
  pub fn items_exceeding_lines(&self, max_lines: usize) -> Vec<&BulletInfo> {
    self
      .line_wraps
      .iter()
      .filter(|b| b.line_count() > max_lines)
      .collect()
  }

  /// Returns wrapped bullets ending in a short orphan trailing line.
  #[must_use]
  pub fn orphan_tails(&self) -> Vec<&BulletInfo> {
    self
      .line_wraps
      .iter()
      .filter(|b| b.is_orphan_tail())
      .collect()
  }
}

/// Queries metadata elements matching `label_name` from document introspector.
fn query_metadata(
  doc: &typst_layout::PagedDocument,
  label_name: &str,
) -> Vec<foundations::Value> {
  let Some(label) = foundations::Label::new(utils::PicoStr::intern(label_name))
  else {
    return Vec::new();
  };

  let elements = introspection::Introspector::query(
    doc.introspector().as_ref(),
    &foundations::Selector::Label(label),
  );
  let mut values = Vec::new();

  for elem in elements {
    if let Some(meta) = elem.to_packed::<introspection::MetadataElem>() {
      values.push(meta.value.clone());
    } else if let Ok(val) = elem.get_by_name("value") {
      values.push(val);
    }
  }

  values
}

/// Handling policy for wrapped bullets ending in a short trailing line.
#[derive(
  Debug, Clone, Copy, Default, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum OrphanPolicy {
  /// Ignore short trailing line wraps.
  #[default]
  Ignore,
  /// Emit a diagnostic warning without failing the run.
  Warn,
  /// Fail layout checks when a wrapped bullet ends with a short tail.
  Error,
}

/// Default single-unit limit fallback (`1`) for telemetry limits.
const fn default_one() -> usize {
  1
}

/// Default `true` helper for `strict_fonts`.
const fn default_true() -> bool {
  true
}

/// Configurable layout telemetry thresholds enforced by `check` and `build`.
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TelemetryPolicy {
  /// Minimum allowed rendered pages (default `1`).
  #[serde(default = "default_one")]
  pub min_pages: usize,
  /// Maximum allowed rendered pages (default `1`).
  #[serde(default = "default_one")]
  pub max_pages: usize,
  /// Maximum rendered lines permitted per guarded bullet or row (default `1`).
  #[serde(default = "default_one")]
  pub max_bullet_lines: usize,
  /// Require requested `theme.font_family` fonts to exist locally without
  /// fallback (default `true`).
  #[serde(default = "default_true")]
  pub strict_fonts: bool,
  /// Optional minimum final-page fill percentage (`0.0`..=`100.0`).
  #[serde(default)]
  pub min_fill: Option<f64>,
  /// Policy for wrapped bullets with short trailing lines (`ignore`, `warn`,
  /// `error`; default `ignore`).
  #[serde(default)]
  pub orphan_lines: OrphanPolicy,
}

impl Default for TelemetryPolicy {
  fn default() -> Self {
    Self {
      min_pages: 1,
      max_pages: 1,
      max_bullet_lines: 1,
      strict_fonts: true,
      min_fill: None,
      orphan_lines: OrphanPolicy::Ignore,
    }
  }
}

/// Outcome of evaluating [`TelemetryPolicy`] against a [`TelemetryReport`].
#[derive(Debug, Default)]
pub struct PolicyOutcome {
  /// Non-fatal telemetry warnings.
  pub warnings: Vec<String>,
  /// Fatal layout policy violations.
  pub failures: Vec<String>,
}

impl PolicyOutcome {
  /// Returns `true` when zero policy violations occurred.
  #[must_use]
  pub fn passed(&self) -> bool {
    self.failures.is_empty()
  }
}

impl TelemetryPolicy {
  /// Evaluates this policy against `report` and collects warnings and failures.
  #[must_use]
  pub fn evaluate(&self, report: &TelemetryReport) -> PolicyOutcome {
    let mut out = PolicyOutcome::default();

    if !report.missing_fonts.is_empty() {
      let missing = report.missing_fonts.join(", ");
      if self.strict_fonts {
        out.failures.push(format!(
          "missing requested font(s): {missing} (add font files to ./fonts/ or \
           set font_path)"
        ));
      } else {
        out.warnings.push(format!(
          "missing requested font(s): {missing} (using fallback font)"
        ));
      }
    }

    if report.page_count < self.min_pages {
      out.failures.push(format!(
        "too few pages: {} page(s) (minimum {})",
        report.page_count, self.min_pages
      ));
    }

    if report.page_count > self.max_pages {
      out.failures.push(format!(
        "page limit exceeded: {} page(s) (maximum {})",
        report.page_count, self.max_pages
      ));
    }

    if let Some(min_fill) = self.min_fill
      && report.fill_pct < min_fill
    {
      out.failures.push(format!(
        "final page fill {:.1}% is below minimum {min_fill:.1}%",
        report.fill_pct
      ));
    }

    let excess = report.items_exceeding_lines(self.max_bullet_lines);
    if !excess.is_empty() {
      out.failures.push(format!(
        "{} item(s) exceed max_bullet_lines ({})",
        excess.len(),
        self.max_bullet_lines
      ));
    }

    let orphans = report.orphan_tails();
    if !orphans.is_empty() {
      match self.orphan_lines {
        OrphanPolicy::Ignore => {}
        OrphanPolicy::Warn => {
          for b in orphans {
            let tail = b.tail_fill.unwrap_or(0.0);
            out.warnings.push(format!(
              "orphan tail wrap ({tail:.0}% final line): \"{}\"",
              b.text
            ));
          }
        }
        OrphanPolicy::Error => {
          out.failures.push(format!(
            "{} bullet(s) end with a short trailing wrap (< {ORPHAN_TAIL_PCT:.0}% \
             last line)",
            orphans.len()
          ));
        }
      }
    }

    out
  }
}

/// Evaluates layout telemetry from a compiled Typst document.
///
/// # Errors
/// Returns [`Error::Deserialization`] if probe extraction fails.
pub fn evaluate(
  doc: &typst_layout::PagedDocument,
) -> Result<TelemetryReport, Error> {
  let doc_pages = doc.pages().len();

  let page_values = query_metadata(doc, "pageinfo");
  let page_info = if let Some(first) = page_values.first() {
    let json_val = serde_json::to_value(first)?;
    serde_json::from_value::<PageInfo>(json_val).unwrap_or_default()
  } else {
    PageInfo::default()
  };

  let bullet_values = query_metadata(doc, "bulletinfo");
  let mut bullets = Vec::new();
  for val in bullet_values {
    if let Ok(json_val) = serde_json::to_value(val)
      && let Ok(bullet) = serde_json::from_value::<BulletInfo>(json_val)
    {
      bullets.push(bullet);
    }
  }

  let effective_pages = usize::max(doc_pages, page_info.pages);
  let available_h = (page_info.page_h - 2.0 * page_info.margin).max(1.0);
  let used_h = (page_info.y - page_info.margin).clamp(0.0, page_info.page_h);

  let fill_pct = (used_h / available_h * 100.0).clamp(0.0, 100.0);
  let spare_in =
    ((page_info.page_h - page_info.margin - page_info.y) / 72.0).max(0.0);

  let mut page_fills = Vec::new();
  for i in 1..=effective_pages {
    if i < effective_pages {
      page_fills.push(100.0);
    } else {
      page_fills.push(fill_pct);
    }
  }

  let line_wraps: Vec<BulletInfo> =
    bullets.into_iter().filter(BulletInfo::is_wrapped).collect();

  Ok(TelemetryReport {
    page_count: effective_pages,
    page_fills,
    fill_pct,
    spare_in,
    line_wraps,
    missing_fonts: Vec::new(),
  })
}
