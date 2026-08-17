//! Layout geometry evaluation, page overflow bounds, and line wrap detection.

use serde::{Deserialize, Serialize};

/// Document geometry info queried from `<pageinfo>`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PageInfo {
  /// Total number of pages in the compiled document.
  pub pages: usize,
  /// Margin size in points (pt).
  pub margin: f64,
  /// Page width in points (pt).
  pub page_w: f64,
  /// Page height in points (pt).
  pub page_h: f64,
  /// Current Y cursor position at document end in points (pt).
  pub y: f64,
}

impl PageInfo {
  /// Parses `PageInfo` from Typst query JSON string.
  ///
  /// # Errors
  ///
  /// Returns an error string if the JSON is malformed or missing fields.
  pub fn parse(json_str: &str) -> Result<Self, String> {
    let val: serde_json::Value = serde_json::from_str(json_str)
      .map_err(|e| format!("Failed to parse page info JSON: {e}"))?;

    let item = if val.is_array() {
      val
        .get(0)
        .ok_or_else(|| "Empty page info query".to_string())?
    } else {
      &val
    };

    let pages =
      item.get("pages").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
    let margin = item.get("margin").and_then(|v| v.as_f64()).unwrap_or(36.0);
    let page_w = item.get("page_w").and_then(|v| v.as_f64()).unwrap_or(612.0);
    let page_h = item.get("page_h").and_then(|v| v.as_f64()).unwrap_or(792.0);
    let y = item.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);

    Ok(Self {
      pages,
      margin,
      page_w,
      page_h,
      y,
    })
  }
}

/// Metadata probe queried from `<bulletinfo>`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BulletInfo {
  /// Probe identifier tag.
  pub id: String,
  /// Kind of content item (e.g. "bullet", "header").
  pub kind: String,
  /// Measured text fill ratio percentage ($width / available\_width \times 100$).
  pub fill: f64,
  /// Associated text snippet or description.
  pub text: String,
}

impl BulletInfo {
  /// Parses a list of `BulletInfo` items from Typst query JSON string.
  ///
  /// # Errors
  ///
  /// Returns an error string if JSON array parsing fails.
  pub fn parse_list(json_str: &str) -> Result<Vec<Self>, String> {
    if json_str.trim().is_empty() || json_str.trim() == "[]" {
      return Ok(Vec::new());
    }

    serde_json::from_str(json_str)
      .map_err(|e| format!("Failed to parse bullet info query JSON: {e}"))
  }
}

/// Comprehensive layout evaluation report.
#[derive(Debug, Clone, PartialEq)]
pub struct TelemetryReport {
  /// Total number of pages.
  pub page_count: usize,
  /// Document vertical height fill percentage ($0.0\% - 100.0\%$).
  pub fill_pct: f64,
  /// Spare vertical space at the bottom of page 1 in inches.
  pub spare_in: f64,
  /// Bullet items that wrapped onto an extra line ($fill > 100.0\%$).
  pub line_wraps: Vec<BulletInfo>,
  /// Bullet items that underfilled ($fill < 86.0\%$).
  pub underfills: Vec<BulletInfo>,
  /// All queried bullet probes.
  pub all_bullets: Vec<BulletInfo>,
}

impl TelemetryReport {
  /// Constructs a new `TelemetryReport`.
  pub fn new(
    page_count: usize,
    fill_pct: f64,
    spare_in: f64,
    line_wraps: Vec<BulletInfo>,
    underfills: Vec<BulletInfo>,
    all_bullets: Vec<BulletInfo>,
  ) -> Self {
    Self {
      page_count,
      fill_pct,
      spare_in,
      line_wraps,
      underfills,
      all_bullets,
    }
  }

  /// Returns `true` if document passes strict single-page constraints.
  pub fn is_pass(&self) -> bool {
    self.page_count == 1 && self.line_wraps.is_empty()
  }
}

/// Evaluates raw JSON strings from `<pageinfo>` and `<bulletinfo>` queries.
///
/// # Errors
///
/// Returns an error string if parsing JSON queries fails.
pub fn evaluate_telemetry(
  page_json: &str,
  bullets_json: &str,
) -> Result<TelemetryReport, String> {
  let page_info = PageInfo::parse(page_json)?;
  let bullets = BulletInfo::parse_list(bullets_json)?;

  let usable_height_pt = page_info.page_h - (2.0 * page_info.margin);
  let used_height_pt = (page_info.y - page_info.margin).max(0.0);
  let spare_pt = (usable_height_pt - used_height_pt).max(0.0);
  let fill_pct = (used_height_pt / usable_height_pt * 100.0).clamp(0.0, 100.0);
  let spare_in = spare_pt / 72.0;

  let mut wraps = Vec::new();
  let mut underfills = Vec::new();

  for b in &bullets {
    if b.kind == "bullet" {
      if b.fill > 100.0 {
        wraps.push(b.clone());
      } else if b.fill < 86.0 {
        underfills.push(b.clone());
      }
    }
  }

  Ok(TelemetryReport::new(
    page_info.pages,
    fill_pct,
    spare_in,
    wraps,
    underfills,
    bullets,
  ))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_evaluate_valid_single_page_optimal_bullets() {
    let page_json =
      r#"[{"pages":1,"margin":36.0,"page_w":612.0,"page_h":792.0,"y":600.0}]"#;
    let bullets_json = r#"[
      {"id":"b1","kind":"bullet","fill":95.0,"text":"Built distributed service in Rust."},
      {"id":"b2","kind":"bullet","fill":88.0,"text":"Reduced p99 latency by 30%."}
    ]"#;

    let report = evaluate_telemetry(page_json, bullets_json).unwrap();
    assert_eq!(report.page_count, 1);
    assert!(report.is_pass());
    assert!(report.line_wraps.is_empty());
    assert!(report.underfills.is_empty());
    assert_eq!(report.all_bullets.len(), 2);
  }

  #[test]
  fn test_evaluate_multi_page_document_fails() {
    let page_json =
      r#"[{"pages":2,"margin":36.0,"page_w":612.0,"page_h":792.0,"y":100.0}]"#;
    let report = evaluate_telemetry(page_json, "[]").unwrap();
    assert_eq!(report.page_count, 2);
    assert!(!report.is_pass());
  }

  #[test]
  fn test_evaluate_bullet_wrap_detection() {
    let page_json =
      r#"[{"pages":1,"margin":36.0,"page_w":612.0,"page_h":792.0,"y":500.0}]"#;
    let bullets_json = r#"[
      {"id":"b1","kind":"bullet","fill":105.2,"text":"A very long bullet line wrapping."},
      {"id":"b2","kind":"bullet","fill":60.0,"text":"Short underfilled bullet line."}
    ]"#;

    let report = evaluate_telemetry(page_json, bullets_json).unwrap();
    assert_eq!(report.page_count, 1);
    assert!(!report.is_pass());
    assert_eq!(report.line_wraps.len(), 1);
    assert_eq!(report.underfills.len(), 1);
  }
}
