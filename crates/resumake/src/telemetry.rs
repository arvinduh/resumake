//! Document layout telemetry, page overflow analysis, and bullet wrapping.
//!
//! Owns extracting layout geometry probes (`<pageinfo>`, `<bulletinfo>`)
//! from compiled Typst documents and calculating page fill percentages.

use serde;
use serde_json;
use thiserror;
use typst::foundations;
use typst::introspection;
use typst::introspection::Introspector;
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
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct PageInfo {
  /// Total pages in document.
  #[serde(default = "default_pages")]
  pub pages: usize,
  /// Margin size in points.
  #[serde(default = "default_margin")]
  pub margin: f64,
  /// Page width in points.
  #[serde(default = "default_width")]
  pub page_w: f64,
  /// Page height in points.
  #[serde(default = "default_height")]
  pub page_h: f64,
  /// Y cursor coordinate at document end in points.
  #[serde(default)]
  pub y: f64,
}

/// Default page count fallback.
const fn default_pages() -> usize {
  1
}

/// Default page margin fallback in points.
const fn default_margin() -> f64 {
  36.0
}

/// Default page width fallback in points.
const fn default_width() -> f64 {
  612.0
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
      page_w: default_width(),
      page_h: default_height(),
      y: 0.0,
    }
  }
}

/// Text item probe queried from `<bulletinfo>`.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct BulletInfo {
  /// Probe identifier tag.
  #[serde(default)]
  pub id: String,
  /// Content item kind (e.g. "bullet").
  #[serde(default)]
  pub kind: String,
  /// Measured width fill percentage.
  #[serde(default)]
  pub fill: f64,
  /// Explicit wrap flag emitted by newer templates.
  #[serde(default)]
  pub wrapped: Option<bool>,
  /// Text content of the bullet item.
  #[serde(default)]
  pub text: String,
}

impl BulletInfo {
  /// Calculates where a wrapped line breaks onto a second line.
  #[must_use]
  pub fn wrap_boundary(&self) -> Option<WrapBoundary<'_>> {
    calculate_wrap_boundary(&self.text, self.fill)
  }
}

/// Calculated character and text boundaries for a wrapped line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrapBoundary<'a> {
  /// Character index where line 1 ends.
  pub fit_char_idx: usize,
  /// Text fitting on the first line.
  pub fit_text: &'a str,
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
    fit_text,
    spill_text,
  })
}

/// Comprehensive layout evaluation report.
#[derive(Debug, Clone, PartialEq)]
pub struct TelemetryReport {
  /// Total number of pages.
  pub page_count: usize,
  /// Fill percentage for each page.
  pub page_fills: Vec<f64>,
  /// Final page fill percentage.
  pub fill_pct: f64,
  /// Spare vertical space on final page in inches.
  pub spare_in: f64,
  /// List of bullet items that wrapped across lines.
  pub line_wraps: Vec<BulletInfo>,
}

impl TelemetryReport {
  /// Returns `true` if document fits strictly on a single page with no wraps.
  #[must_use]
  pub fn is_single_page_clean(&self) -> bool {
    self.page_count == 1 && self.line_wraps.is_empty()
  }

  /// Returns `true` if document has 1 page.
  #[must_use]
  pub fn is_single_page(&self) -> bool {
    self.page_count == 1
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

  let elements = doc
    .introspector()
    .query(&foundations::Selector::Label(label));
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

  let fill_pct = if effective_pages > 1 {
    100.0
  } else {
    (used_h / available_h * 100.0).clamp(0.0, 100.0)
  };

  let spare_in = if effective_pages > 1 {
    0.0
  } else {
    ((page_info.page_h - page_info.margin - page_info.y) / 72.0).max(0.0)
  };

  let mut page_fills = Vec::new();
  for i in 1..=effective_pages {
    if i < effective_pages {
      page_fills.push(100.0);
    } else {
      page_fills.push(fill_pct);
    }
  }

  let line_wraps: Vec<BulletInfo> = bullets
    .into_iter()
    .filter(|b| b.wrapped.unwrap_or(b.fill > 100.0))
    .collect();

  Ok(TelemetryReport {
    page_count: effective_pages,
    page_fills,
    fill_pct,
    spare_in,
    line_wraps,
  })
}
