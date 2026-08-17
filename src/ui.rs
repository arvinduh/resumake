//! Terminal user interface, zero-emoji status badges, and 64-column summary tables.

use crate::telemetry::TelemetryReport;
use colored::Colorize;

/// Prints the formatted 64-column layout telemetry report table to terminal.
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
  println!(
    " {:<16} {:<45}",
    "Candidate:".truecolor(120, 120, 120),
    candidate_name.white().bold()
  );
  println!(
    " {:<16} {:<45}",
    "Output:".truecolor(120, 120, 120),
    output_pdf.cyan()
  );
  println!(
    " {:<16} {:<45}",
    "Version:".truecolor(120, 120, 120),
    version.truecolor(160, 160, 160)
  );
  println!("{divider}");

  // Page count badge
  let page_badge = if report.page_count == 1 {
    "[PASS 1/1]".green().bold()
  } else {
    format!("[FAIL {}/1]", report.page_count).red().bold()
  };
  println!(
    " {:<16} {:<30} {:>15}",
    "Page Count:".truecolor(120, 120, 120),
    format!("{} page(s)", report.page_count).white(),
    page_badge
  );

  // Vertical space fill badge
  let fill_badge = if report.fill_pct >= 90.0 && report.fill_pct <= 99.0 {
    "[OPTIMAL]".green().bold()
  } else if report.fill_pct > 99.0 {
    "[OVERFLOW]".red().bold()
  } else {
    "[ROOM]".yellow().bold()
  };
  println!(
    " {:<16} {:<30} {:>15}",
    "Vertical Fill:".truecolor(120, 120, 120),
    format!("{:.1}% (spare: {:.2} in)", report.fill_pct, report.spare_in)
      .white(),
    fill_badge
  );

  // Line wraps badge
  let wrap_badge = if report.line_wraps.is_empty() {
    "[0 WRAPS]".green().bold()
  } else {
    format!("[{} WRAPS]", report.line_wraps.len()).red().bold()
  };
  println!(
    " {:<16} {:<30} {:>15}",
    "Line Wraps:".truecolor(120, 120, 120),
    format!("{} wrapped items", report.line_wraps.len()).white(),
    wrap_badge
  );

  // Underfills badge
  let under_badge = if report.underfills.is_empty() {
    "[0 UNDER]".green().bold()
  } else {
    format!("[{} UNDER]", report.underfills.len())
      .yellow()
      .bold()
  };
  println!(
    " {:<16} {:<30} {:>15}",
    "Underfills:".truecolor(120, 120, 120),
    format!("{} items (<86%)", report.underfills.len()).white(),
    under_badge
  );

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
