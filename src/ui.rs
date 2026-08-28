//! Terminal user interface, zero-emoji status badges, and telemetry tables.

use crate::telemetry::TelemetryReport;
use colored::Colorize;
use comfy_table::presets::NOTHING;
use comfy_table::{Cell, CellAlignment, ContentArrangement, Row, Table};

/// Prints the formatted layout telemetry report table to terminal using `comfy-table`.
pub fn print_telemetry_table(
  report: &TelemetryReport,
  candidate_name: &str,
  output_pdf: &str,
  version: &str,
) {
  let divider =
    "────────────────────────────────────────────────────────────────"
      .truecolor(60, 60, 60);

  println!("{divider}");

  let mut meta_table = Table::new();
  meta_table
    .load_style(NOTHING)
    .set_content_arrangement(ContentArrangement::Dynamic);

  meta_table.add_row(Row::from(vec![
    Cell::new("Candidate:").fg(comfy_table::Color::DarkGrey),
    Cell::new(candidate_name).add_attribute(comfy_table::Attribute::Bold),
  ]));
  meta_table.add_row(Row::from(vec![
    Cell::new("Output:").fg(comfy_table::Color::DarkGrey),
    Cell::new(output_pdf).fg(comfy_table::Color::Cyan),
  ]));
  meta_table.add_row(Row::from(vec![
    Cell::new("Version:").fg(comfy_table::Color::DarkGrey),
    Cell::new(version).fg(comfy_table::Color::Grey),
  ]));

  println!("{meta_table}");
  println!("{divider}");

  // Page count badge
  let page_badge = if report.page_count == 1 {
    "[PASS 1/1]".green().bold().to_string()
  } else {
    format!("[FAIL {}/1]", report.page_count)
      .red()
      .bold()
      .to_string()
  };

  // Vertical space fill badge
  let fill_badge = if report.fill_pct >= 90.0 && report.fill_pct <= 99.0 {
    "[OPTIMAL]".green().bold().to_string()
  } else if report.fill_pct > 99.0 {
    "[OVERFLOW]".red().bold().to_string()
  } else {
    "[ROOM]".yellow().bold().to_string()
  };

  // Line wraps badge
  let wrap_badge = if report.line_wraps.is_empty() {
    "[0 WRAPS]".green().bold().to_string()
  } else {
    format!("[{} WRAPS]", report.line_wraps.len())
      .red()
      .bold()
      .to_string()
  };

  // Underfills badge
  let under_badge = if report.underfills.is_empty() {
    "[0 UNDER]".green().bold().to_string()
  } else {
    format!("[{} UNDER]", report.underfills.len())
      .yellow()
      .bold()
      .to_string()
  };

  let mut metrics_table = Table::new();
  metrics_table
    .load_style(NOTHING)
    .set_content_arrangement(ContentArrangement::Dynamic);

  metrics_table.add_row(Row::from(vec![
    Cell::new("Page Count:").fg(comfy_table::Color::DarkGrey),
    Cell::new(format!("{} page(s)", report.page_count)),
    Cell::new(page_badge).set_alignment(CellAlignment::Right),
  ]));
  metrics_table.add_row(Row::from(vec![
    Cell::new("Vertical Fill:").fg(comfy_table::Color::DarkGrey),
    Cell::new(format!(
      "{:.1}% (spare: {:.2} in)",
      report.fill_pct, report.spare_in
    )),
    Cell::new(fill_badge).set_alignment(CellAlignment::Right),
  ]));
  metrics_table.add_row(Row::from(vec![
    Cell::new("Line Wraps:").fg(comfy_table::Color::DarkGrey),
    Cell::new(format!("{} wrapped items", report.line_wraps.len())),
    Cell::new(wrap_badge).set_alignment(CellAlignment::Right),
  ]));
  metrics_table.add_row(Row::from(vec![
    Cell::new("Underfills:").fg(comfy_table::Color::DarkGrey),
    Cell::new(format!("{} items (<86%)", report.underfills.len())),
    Cell::new(under_badge).set_alignment(CellAlignment::Right),
  ]));

  println!("{metrics_table}");
  println!("{divider}");

  // Print warnings for wrapped bullets
  if !report.line_wraps.is_empty() {
    println!("{}", " Layout Warnings:".yellow().bold());
    for w in &report.line_wraps {
      println!(
        "   {} [{:.1}% fill] \"{}\"",
        "-".red().bold(),
        w.fill,
        w.text.truecolor(180, 180, 180)
      );
    }
    println!("{divider}");
  }

  // Final status line
  if report.is_pass() {
    println!(
      " Status: {}",
      "SUCCESS (Strict 1-page layout verified)".green().bold()
    );
  } else {
    println!(
      " Status: {}",
      "FAILED (Document violates single-page geometry constraints)"
        .red()
        .bold()
    );
  }
  println!("{divider}");
}

/// Prints a standardized success status badge message.
pub fn print_success(msg: &str) {
  println!("{} {msg}", "[PASS]".green().bold());
}

/// Prints a standardized failure status badge message.
pub fn print_error(msg: &str) {
  eprintln!("{} {msg}", "[FAIL]".red().bold());
}

/// Prints a standardized informational status badge message.
pub fn print_info(msg: &str) {
  println!("{} {msg}", "[INFO]".blue().bold());
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_print_telemetry_table_does_not_panic() {
    let report =
      TelemetryReport::new(1, 95.5, 0.45, Vec::new(), Vec::new(), Vec::new());
    print_telemetry_table(&report, "Jane Doe", "janedoe_resume.pdf", "1.0.0");
    print_success("Build complete");
    print_error("Failed");
    print_info("Watching");
  }
}
