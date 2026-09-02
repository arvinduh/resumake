//! Terminal user interface, zero-emoji status badges, and telemetry tables.

use crate::telemetry::TelemetryReport;
use colored::Colorize;
use comfy_table::{
  Attribute, Cell, CellAlignment, Color, ContentArrangement, ContentLineStyle,
  LineStyle, Row, Table, TableStyle,
};

/// Builds the formatted layout telemetry report table using `comfy-table`.
pub fn build_telemetry_table(
  report: &TelemetryReport,
  candidate_name: &str,
  output_pdf: &str,
  version: &str,
) -> Table {
  let table_style = TableStyle {
    top_border: LineStyle {
      left: None,
      fill: Some('─'),
      junction: Some('─'),
      right: None,
    },
    bottom_border: LineStyle {
      left: None,
      fill: Some('─'),
      junction: Some('─'),
      right: None,
    },
    header_lines: ContentLineStyle {
      left: None,
      junction: None,
      right: None,
    },
    header_separator: LineStyle {
      left: None,
      fill: None,
      junction: None,
      right: None,
    },
    content_lines: ContentLineStyle {
      left: None,
      junction: None,
      right: None,
    },
    row_separator: LineStyle {
      left: None,
      fill: None,
      junction: None,
      right: None,
    },
  };

  let mut table = Table::new();
  table
    .load_style(table_style)
    .set_content_arrangement(ContentArrangement::Dynamic);

  // Metadata rows
  table.add_row(Row::from(vec![
    Cell::new("Candidate:").fg(Color::DarkGrey),
    Cell::new(candidate_name).add_attribute(Attribute::Bold),
    Cell::new(""),
  ]));
  table.add_row(Row::from(vec![
    Cell::new("Output:").fg(Color::DarkGrey),
    Cell::new(output_pdf).fg(Color::Cyan),
    Cell::new(""),
  ]));
  table.add_row(Row::from(vec![
    Cell::new("Version:").fg(Color::DarkGrey),
    Cell::new(version).fg(Color::Grey),
    Cell::new(""),
  ]));

  // Page count badge
  let page_badge = if report.page_count == 1 {
    Cell::new("[PASS 1/1]")
      .fg(Color::Green)
      .add_attribute(Attribute::Bold)
      .set_alignment(CellAlignment::Right)
  } else {
    Cell::new(format!("[FAIL {}/1]", report.page_count))
      .fg(Color::Red)
      .add_attribute(Attribute::Bold)
      .set_alignment(CellAlignment::Right)
  };

  // Vertical space fill badge
  let fill_badge = if report.fill_pct >= 90.0 && report.fill_pct <= 99.0 {
    Cell::new("[OPTIMAL]")
      .fg(Color::Green)
      .add_attribute(Attribute::Bold)
      .set_alignment(CellAlignment::Right)
  } else if report.fill_pct > 99.0 {
    Cell::new("[OVERFLOW]")
      .fg(Color::Red)
      .add_attribute(Attribute::Bold)
      .set_alignment(CellAlignment::Right)
  } else {
    Cell::new("[ROOM]")
      .fg(Color::Yellow)
      .add_attribute(Attribute::Bold)
      .set_alignment(CellAlignment::Right)
  };

  // Line wraps badge
  let wrap_badge = if report.line_wraps.is_empty() {
    Cell::new("[0 WRAPS]")
      .fg(Color::Green)
      .add_attribute(Attribute::Bold)
      .set_alignment(CellAlignment::Right)
  } else {
    Cell::new(format!("[{} WRAPS]", report.line_wraps.len()))
      .fg(Color::Red)
      .add_attribute(Attribute::Bold)
      .set_alignment(CellAlignment::Right)
  };

  // Underfills badge
  let under_badge = if report.underfills.is_empty() {
    Cell::new("[0 UNDER]")
      .fg(Color::Green)
      .add_attribute(Attribute::Bold)
      .set_alignment(CellAlignment::Right)
  } else {
    Cell::new(format!("[{} UNDER]", report.underfills.len()))
      .fg(Color::Yellow)
      .add_attribute(Attribute::Bold)
      .set_alignment(CellAlignment::Right)
  };

  // Metrics rows
  table.add_row(Row::from(vec![
    Cell::new("Page Count:").fg(Color::DarkGrey),
    Cell::new(format!("{} page(s)", report.page_count)),
    page_badge,
  ]));
  table.add_row(Row::from(vec![
    Cell::new("Vertical Fill:").fg(Color::DarkGrey),
    Cell::new(format!(
      "{:.1}% (spare: {:.2} in)",
      report.fill_pct, report.spare_in
    )),
    fill_badge,
  ]));
  table.add_row(Row::from(vec![
    Cell::new("Line Wraps:").fg(Color::DarkGrey),
    Cell::new(format!("{} wrapped items", report.line_wraps.len())),
    wrap_badge,
  ]));
  table.add_row(Row::from(vec![
    Cell::new("Underfills:").fg(Color::DarkGrey),
    Cell::new(format!("{} items (<86%)", report.underfills.len())),
    under_badge,
  ]));

  // Layout warnings if any wrapped bullets exist
  if !report.line_wraps.is_empty() {
    for (i, w) in report.line_wraps.iter().enumerate() {
      let label_cell = if i == 0 {
        Cell::new("Layout Warnings:")
          .fg(Color::Yellow)
          .add_attribute(Attribute::Bold)
      } else {
        Cell::new("")
      };
      table.add_row(Row::from(vec![
        label_cell,
        Cell::new(format!("- [{:.1}% fill] \"{}\"", w.fill, w.text))
          .fg(Color::DarkGrey),
        Cell::new(""),
      ]));
    }
  }

  // Final status row
  let (status_text, status_color) = if report.is_pass() {
    ("SUCCESS (Strict 1-page layout verified)", Color::Green)
  } else {
    (
      "FAILED (Document violates single-page geometry constraints)",
      Color::Red,
    )
  };
  table.add_row(Row::from(vec![
    Cell::new("Status:").fg(Color::DarkGrey),
    Cell::new(status_text)
      .fg(status_color)
      .add_attribute(Attribute::Bold),
    Cell::new(""),
  ]));

  table
}

/// Formats the layout telemetry report table into a string using `comfy-table`.
pub fn format_telemetry_table(
  report: &TelemetryReport,
  candidate_name: &str,
  output_pdf: &str,
  version: &str,
) -> String {
  build_telemetry_table(report, candidate_name, output_pdf, version).to_string()
}

/// Prints the formatted layout telemetry report table to terminal using `comfy-table`.
pub fn print_telemetry_table(
  report: &TelemetryReport,
  candidate_name: &str,
  output_pdf: &str,
  version: &str,
) {
  println!(
    "{}",
    format_telemetry_table(report, candidate_name, output_pdf, version)
  );
}

/// Formats a standardized success status badge message as a string.
pub fn format_success(msg: &str) -> String {
  format!("{} {msg}", "[PASS]".green().bold())
}

/// Prints a standardized success status badge message.
pub fn print_success(msg: &str) {
  println!("{}", format_success(msg));
}

/// Formats a standardized failure status badge message as a string.
pub fn format_error(msg: &str) -> String {
  format!("{} {msg}", "[FAIL]".red().bold())
}

/// Prints a standardized failure status badge message.
pub fn print_error(msg: &str) {
  eprintln!("{}", format_error(msg));
}

/// Formats a standardized informational status badge message as a string.
pub fn format_info(msg: &str) -> String {
  format!("{} {msg}", "[INFO]".blue().bold())
}

/// Prints a standardized informational status badge message.
pub fn print_info(msg: &str) {
  println!("{}", format_info(msg));
}

/// Formats a standardized warning status badge message as a string.
pub fn format_warning(msg: &str) -> String {
  format!("{} {msg}", "[WARN]".yellow().bold())
}

/// Prints a standardized warning status badge message.
pub fn print_warning(msg: &str) {
  println!("{}", format_warning(msg));
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::telemetry::BulletInfo;

  #[test]
  fn test_print_telemetry_table_does_not_panic() {
    let report =
      TelemetryReport::new(1, 95.5, 0.45, Vec::new(), Vec::new(), Vec::new());
    print_telemetry_table(&report, "Jane Doe", "janedoe_resume.pdf", "1.0.0");
    print_success("Build complete");
    print_error("Failed");
    print_info("Watching");
  }

  #[test]
  fn test_build_telemetry_table_pass_structure() {
    let report =
      TelemetryReport::new(1, 95.2, 0.38, Vec::new(), Vec::new(), Vec::new());
    let table =
      build_telemetry_table(&report, "Jane Doe", "janedoe_resume.pdf", "1.0.0");
    let output = table.to_string();
    assert!(output.contains("Candidate:"));
    assert!(output.contains("Jane Doe"));
    assert!(output.contains("Output:"));
    assert!(output.contains("janedoe_resume.pdf"));
    assert!(output.contains("Version:"));
    assert!(output.contains("1.0.0"));
    assert!(output.contains("Page Count:"));
    assert!(output.contains("[PASS 1/1]"));
    assert!(output.contains("Vertical Fill:"));
    assert!(output.contains("[OPTIMAL]"));
    assert!(output.contains("Line Wraps:"));
    assert!(output.contains("[0 WRAPS]"));
    assert!(output.contains("Underfills:"));
    assert!(output.contains("[0 UNDER]"));
    assert!(output.contains("Status:"));
    assert!(output.contains("SUCCESS (Strict 1-page layout verified)"));
  }

  #[test]
  fn test_build_telemetry_table_with_warnings_and_failure() {
    let report = TelemetryReport::new(
      2,
      102.5,
      0.0,
      vec![BulletInfo {
        id: "1".to_string(),
        kind: "bullet".to_string(),
        fill: 104.2,
        text: "Long bullet item that wraps to next line".to_string(),
      }],
      vec![BulletInfo {
        id: "2".to_string(),
        kind: "bullet".to_string(),
        fill: 75.0,
        text: "Short bullet item".to_string(),
      }],
      Vec::new(),
    );
    let table =
      build_telemetry_table(&report, "Jane Doe", "janedoe_resume.pdf", "1.0.0");
    let output = table.to_string();
    assert!(output.contains("[FAIL 2/1]"));
    assert!(output.contains("[OVERFLOW]"));
    assert!(output.contains("[1 WRAPS]"));
    assert!(output.contains("[1 UNDER]"));
    assert!(output.contains("Layout Warnings:"));
    assert!(output.contains("Long bullet item that wraps to next line"));
    assert!(output
      .contains("FAILED (Document violates single-page geometry constraints)"));
  }

  #[test]
  fn test_doc_sample_output() {
    let report =
      TelemetryReport::new(1, 95.2, 0.38, Vec::new(), Vec::new(), Vec::new());
    let formatted = format_telemetry_table(
      &report,
      "Jane Doe",
      "janedoe_resume.pdf",
      "1.0.0",
    );
    assert!(formatted.contains("Jane Doe"));
    assert!(formatted.contains("[PASS 1/1]"));
  }
}
