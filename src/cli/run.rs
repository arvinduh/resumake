//! Shared build and check pipeline execution for `rsmk`.
//!
//! Resolves targets from `resumake.toml` manifests or standalone content files,
//! merges `[build]` and `[telemetry]` defaults with CLI flag overrides, and
//! executes validation, Typst compilation, PDF export, and telemetry gates.

use std::fmt::Write;
use std::fs;
use std::path;

use clap;
use resumake;

use crate::cli;

/// CLI argument representation for orphan trailing line wrap policy.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum OrphanArg {
  /// Ignore short trailing line wraps.
  Ignore,
  /// Emit warnings for short trailing line wraps.
  Warn,
  /// Fail checks on short trailing line wraps.
  Error,
}

impl From<OrphanArg> for resumake::telemetry::OrphanPolicy {
  fn from(arg: OrphanArg) -> Self {
    match arg {
      OrphanArg::Ignore => Self::Ignore,
      OrphanArg::Warn => Self::Warn,
      OrphanArg::Error => Self::Error,
    }
  }
}

/// Shared build and telemetry options accepted by both `build` and `check`.
#[derive(clap::Args, Default)]
pub struct PipelineOptions {
  /// Named built-in layout, local template directory, or .typ file to render
  /// with
  #[arg(short, long)]
  pub template: Option<String>,

  /// Path to custom JSON schema file (overrides manifest or built-in schema)
  #[arg(long)]
  pub schema: Option<path::PathBuf>,

  /// Custom font directory (auto-detects ./fonts if present)
  #[arg(long)]
  pub font_path: Option<path::PathBuf>,

  /// Minimum pages required before failing
  #[arg(long)]
  pub min_pages: Option<usize>,

  /// Most pages allowed before failing (default 1)
  #[arg(long)]
  pub max_pages: Option<usize>,

  /// Maximum lines permitted per bullet or skill row (default 1)
  #[arg(long)]
  pub max_bullet_lines: Option<usize>,

  /// Minimum final-page fill percentage required (`0.0`..=`100.0`)
  #[arg(long)]
  pub min_fill: Option<f64>,

  /// Require requested fonts to exist locally (`true` or `false`)
  #[arg(long, action = clap::ArgAction::Set)]
  pub strict_fonts: Option<bool>,

  /// Policy for short orphan trailing wraps (`ignore`, `warn`, `error`)
  #[arg(long, value_enum)]
  pub orphan_lines: Option<OrphanArg>,
}

/// Execution mode distinguishing `check` (no PDF) from `build`.
#[derive(Clone, Copy)]
pub(crate) enum Mode<'a> {
  /// Verify schema, compilation, and telemetry only.
  Check,
  /// Compile PDF(s) to disk and verify telemetry.
  Build {
    /// Explicit single-target output PDF path.
    output: Option<&'a path::Path>,
    /// Output directory overriding manifest `[build].out_dir`.
    out_dir: Option<&'a path::Path>,
    /// Include all `[docs]` entries even when `publish = false`.
    all: bool,
  },
}

/// Concrete resolved execution parameters for one résumé document.
struct ResolvedTarget {
  /// Human-readable target label (doc name or content path).
  label: String,
  /// Manifest document key under `[docs]`, if resolved from a manifest.
  doc_name: Option<String>,
  /// Path to `.yaml`, `.yml`, or `.json` résumé content file.
  content_path: path::PathBuf,
  /// Project root directory for resolving `fonts/`, `templates/`, and schemas.
  project_root: path::PathBuf,
  /// Effective template name or path override.
  template: Option<String>,
  /// Effective custom content JSON Schema override.
  schema_path: Option<path::PathBuf>,
  /// Effective custom font directory override.
  font_path: Option<path::PathBuf>,
  /// Optional `[docs.<name>].output` filename or path.
  doc_output: Option<path::PathBuf>,
  /// Optional `[build].out_dir` path resolved from `project_root`.
  manifest_out_dir: Option<path::PathBuf>,
}

/// Checks if a target string appears to name a filesystem path rather than a
/// manifest doc key.
fn looks_like_path(target: &str, cwd: &path::Path) -> bool {
  target.contains('/')
    || target.contains('\\')
    || path::Path::new(target).extension().is_some_and(|ext| {
      ext.eq_ignore_ascii_case("yaml")
        || ext.eq_ignore_ascii_case("yml")
        || ext.eq_ignore_ascii_case("json")
    })
    || resolve_cli_path(cwd, path::Path::new(target)).is_file()
}

/// Resolves a CLI path relative to `cwd` when not already absolute.
fn resolve_cli_path(cwd: &path::Path, p: &path::Path) -> path::PathBuf {
  if p.is_absolute() {
    p.to_path_buf()
  } else {
    cwd.join(p)
  }
}

/// Auto-discovers `content.{yaml,yml,json}` inside `dir`.
fn discover_default_content(dir: &path::Path) -> Option<path::PathBuf> {
  ["content.yaml", "content.yml", "content.json"]
    .iter()
    .map(|name| dir.join(name))
    .find(|candidate| candidate.is_file())
}

/// Normalizes text into a lowercase underscore-delimited filename stem.
fn slugify(raw: &str) -> String {
  raw
    .trim()
    .to_lowercase()
    .replace(|c: char| !c.is_alphanumeric(), "_")
    .trim_matches('_')
    .to_string()
}

/// Derives output PDF destination for a built document.
fn resolve_output_pdf(
  target: &ResolvedTarget,
  explicit_output: Option<&path::Path>,
  cli_out_dir: Option<&path::Path>,
) -> path::PathBuf {
  if let Some(out) = explicit_output {
    return out.to_path_buf();
  }

  let effective_dir = cli_out_dir.or(target.manifest_out_dir.as_deref());

  if let Some(doc_out) = &target.doc_output {
    if let Some(dir) = effective_dir {
      let file_name = doc_out.file_name().unwrap_or(doc_out.as_os_str());
      return dir.join(file_name);
    }
    return resumake::manifest::resolve_path(&target.project_root, doc_out);
  }

  let candidate_slug = fs::read_to_string(&target.content_path)
    .ok()
    .and_then(|raw| {
      resumake::schema::load_meta(
        &raw,
        resumake::schema::ContentFormat::from_path(&target.content_path),
      )
      .ok()
    })
    .map(|meta| slugify(&meta.name))
    .filter(|s| !s.is_empty());

  let filename = match (&target.doc_name, candidate_slug) {
    (Some(doc), Some(cand)) if doc != "main" => {
      let doc_slug = slugify(doc);
      if doc_slug.is_empty() {
        format!("{cand}_resume.pdf")
      } else {
        format!("{cand}_{doc_slug}_resume.pdf")
      }
    }
    (Some(doc), None) if doc != "main" => {
      let doc_slug = slugify(doc);
      if doc_slug.is_empty() {
        "resume.pdf".to_string()
      } else {
        format!("{doc_slug}_resume.pdf")
      }
    }
    (_, Some(cand)) => format!("{cand}_resume.pdf"),
    _ => "resume.pdf".to_string(),
  };

  match effective_dir {
    Some(dir) => dir.join(filename),
    None => path::PathBuf::from(filename),
  }
}

/// Converts a manifest doc entry plus build defaults and CLI flags into a
/// resolved target.
fn target_from_manifest(
  doc_name: String,
  doc: &resumake::manifest::DocConfig,
  manifest_root: &path::Path,
  build: &resumake::manifest::BuildDefaults,
  opts: &PipelineOptions,
) -> ResolvedTarget {
  let resolve =
    |p: &path::Path| resumake::manifest::resolve_path(manifest_root, p);

  ResolvedTarget {
    label: doc_name.clone(),
    doc_name: Some(doc_name),
    content_path: resolve(&doc.content),
    project_root: manifest_root.to_path_buf(),
    template: opts
      .template
      .clone()
      .or_else(|| doc.template.clone())
      .or_else(|| build.template.clone()),
    schema_path: opts
      .schema
      .clone()
      .or_else(|| doc.schema.as_deref().map(resolve))
      .or_else(|| build.schema.as_deref().map(resolve)),
    font_path: opts
      .font_path
      .clone()
      .or_else(|| doc.font_path.as_deref().map(resolve))
      .or_else(|| build.font_path.as_deref().map(resolve)),
    doc_output: doc.output.clone(),
    manifest_out_dir: build.out_dir.as_deref().map(resolve),
  }
}

/// Builds the effective [`resumake::telemetry::TelemetryPolicy`] from manifest
/// defaults and CLI overrides.
fn effective_policy(
  manifest: Option<&resumake::manifest::Manifest>,
  opts: &PipelineOptions,
) -> resumake::telemetry::TelemetryPolicy {
  let mut policy = manifest.map(|m| m.telemetry.clone()).unwrap_or_default();
  if let Some(v) = opts.min_pages {
    policy.min_pages = v;
  }
  if let Some(v) = opts.max_pages {
    policy.max_pages = v;
  }
  if let Some(v) = opts.max_bullet_lines {
    policy.max_bullet_lines = v;
  }
  if let Some(v) = opts.min_fill {
    policy.min_fill = Some(v);
  }
  if let Some(v) = opts.strict_fonts {
    policy.strict_fonts = v;
  }
  if let Some(v) = opts.orphan_lines {
    policy.orphan_lines = v.into();
  }
  policy
}

/// Resolves document targets given an optional loaded manifest and target arg.
fn select_targets(
  target_arg: Option<&str>,
  manifest_data: Option<&(path::PathBuf, resumake::manifest::Manifest)>,
  opts: &PipelineOptions,
  include_all: bool,
  cwd: &path::Path,
) -> Result<Vec<ResolvedTarget>, cli::Error> {
  match (target_arg, manifest_data) {
    (Some(arg), Some((root, manifest))) => {
      if let Some(doc) = manifest.docs.get(arg) {
        return Ok(vec![target_from_manifest(
          arg.to_string(),
          doc,
          root,
          &manifest.build,
          opts,
        )]);
      }
      if !looks_like_path(arg, cwd) && !manifest.docs.is_empty() {
        let names = manifest
          .docs
          .keys()
          .map(String::as_str)
          .collect::<Vec<_>>()
          .join(", ");
        return Err(cli::Error::Config(format!(
          "unknown target '{arg}' in resumake.toml (available docs: {names})"
        )));
      }
      Ok(vec![ad_hoc_target(
        resolve_cli_path(cwd, path::Path::new(arg)),
        Some((root, &manifest.build)),
        opts,
      )])
    }
    (Some(arg), None) => Ok(vec![ad_hoc_target(
      resolve_cli_path(cwd, path::Path::new(arg)),
      None,
      opts,
    )]),
    (None, Some((root, manifest))) if !manifest.docs.is_empty() => {
      let selected: Vec<_> = manifest
        .docs
        .iter()
        .filter(|(_, doc)| include_all || doc.publish)
        .map(|(name, doc)| {
          target_from_manifest(name.clone(), doc, root, &manifest.build, opts)
        })
        .collect();

      if selected.is_empty() {
        return Err(cli::Error::Config(
          "no documents in resumake.toml have publish = true; set publish = \
           true on a [docs.<name>] entry, specify a target name, or pass --all"
            .to_string(),
        ));
      }
      Ok(selected)
    }
    (None, Some((root, manifest))) => {
      let content = discover_default_content(cwd)
        .or_else(|| discover_default_content(root))
        .ok_or_else(|| {
          cli::Error::Config(
            "no content.yaml, content.yml, or content.json found; add \
             [docs.<name>] to resumake.toml or pass a file path"
              .to_string(),
          )
        })?;
      Ok(vec![ad_hoc_target(
        content,
        Some((root, &manifest.build)),
        opts,
      )])
    }
    (None, None) => {
      let content = discover_default_content(cwd).ok_or_else(|| {
        cli::Error::Config(
          "no resumake.toml or content.{yaml,yml,json} found (run `rsmk init` \
           or pass a content path)"
            .to_string(),
        )
      })?;
      Ok(vec![ad_hoc_target(content, None, opts)])
    }
  }
}

/// Resolves document targets and effective telemetry policy from the workspace.
fn resolve_plan(
  target_arg: Option<&str>,
  opts: &PipelineOptions,
  include_all: bool,
  cwd: &path::Path,
) -> Result<
  (Vec<ResolvedTarget>, resumake::telemetry::TelemetryPolicy),
  cli::Error,
> {
  let discovery_start = target_arg.map_or_else(
    || cwd.to_path_buf(),
    |arg| {
      let resolved = resolve_cli_path(cwd, path::Path::new(arg));
      if resolved.is_file() || looks_like_path(arg, cwd) {
        resolved
          .parent()
          .filter(|d| !d.as_os_str().is_empty())
          .unwrap_or(cwd)
          .to_path_buf()
      } else {
        cwd.to_path_buf()
      }
    },
  );

  let manifest_data = match resumake::manifest::discover(&discovery_start) {
    Some(manifest_path) => {
      let root = manifest_path
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .unwrap_or(cwd)
        .to_path_buf();
      let manifest = resumake::manifest::load(&manifest_path)?;
      Some((root, manifest))
    }
    None => None,
  };

  let policy = effective_policy(manifest_data.as_ref().map(|(_, m)| m), opts);
  let targets =
    select_targets(target_arg, manifest_data.as_ref(), opts, include_all, cwd)?;

  Ok((targets, policy))
}

/// Builds a [`ResolvedTarget`] for an explicit or auto-discovered content file.
fn ad_hoc_target(
  content_path: path::PathBuf,
  manifest_ctx: Option<(&path::PathBuf, &resumake::manifest::BuildDefaults)>,
  opts: &PipelineOptions,
) -> ResolvedTarget {
  let root = manifest_ctx
    .map_or_else(|| cli::project_root(&content_path), |(r, _)| r.clone());
  let resolve = |p: &path::Path| resumake::manifest::resolve_path(&root, p);
  let build = manifest_ctx.map(|(_, b)| b);
  let template = opts
    .template
    .clone()
    .or_else(|| build.and_then(|b| b.template.clone()));
  let schema_path = opts
    .schema
    .clone()
    .or_else(|| build.and_then(|b| b.schema.as_deref().map(resolve)));
  let font_path = opts
    .font_path
    .clone()
    .or_else(|| build.and_then(|b| b.font_path.as_deref().map(resolve)));
  let manifest_out_dir = build.and_then(|b| b.out_dir.as_deref().map(resolve));

  ResolvedTarget {
    label: content_path.display().to_string(),
    doc_name: None,
    content_path,
    project_root: root,
    template,
    schema_path,
    font_path,
    doc_output: None,
    manifest_out_dir,
  }
}

/// Executes schema validation, compilation, optional PDF write, and telemetry
/// enforcement for a single target.
fn execute_single(
  target: &ResolvedTarget,
  policy: &resumake::telemetry::TelemetryPolicy,
  explicit_output: Option<&path::Path>,
  cli_out_dir: Option<&path::Path>,
  write_pdf: bool,
  quiet: bool,
) -> Result<(), cli::Error> {
  resumake::schema::validate_file(
    &target.content_path,
    target.schema_path.as_deref(),
  )?;

  let (doc, report) = resumake::render::compile(
    &target.content_path,
    target.template.as_deref(),
    &target.project_root,
    target.font_path.as_deref(),
  )?;

  let prefix = if write_pdf {
    let output_pdf = resolve_output_pdf(target, explicit_output, cli_out_dir);
    resumake::render::export_pdf(&doc, &output_pdf)?;
    format!("[{}] built {}", target.label, output_pdf.display())
  } else {
    format!("[{}] ok", target.label)
  };

  let outcome = policy.evaluate(&report);

  if !quiet {
    cli::print_report(&report, &outcome, &prefix);
  }

  if outcome.passed() {
    Ok(())
  } else {
    let reasons = outcome
      .failures
      .iter()
      .map(|f| format!("  - {f}"))
      .collect::<Vec<_>>()
      .join("\n");
    Err(cli::Error::Layout {
      doc: target.label.clone(),
      reasons,
    })
  }
}

/// Executes the check or build pipeline across all selected documents.
///
/// # Errors
/// Returns [`cli::Error`] when target resolution fails or any selected document
/// fails validation, compilation, or layout telemetry checks.
pub(crate) fn run(
  target_arg: Option<&str>,
  opts: &PipelineOptions,
  mode: Mode<'_>,
  quiet: bool,
) -> Result<(), cli::Error> {
  let (write_pdf, include_all, explicit_output, cli_out_dir) = match mode {
    Mode::Check => (false, true, None, None),
    Mode::Build {
      output,
      out_dir,
      all,
    } => (true, all, output, out_dir),
  };

  let (targets, policy) =
    resolve_plan(target_arg, opts, include_all, path::Path::new("."))?;

  if explicit_output.is_some() && targets.len() > 1 {
    return Err(cli::Error::Config(
      "cannot pass --output (-o) when building multiple documents; use \
       --out-dir instead"
        .to_string(),
    ));
  }

  if targets.len() == 1 {
    return execute_single(
      &targets[0],
      &policy,
      explicit_output,
      cli_out_dir,
      write_pdf,
      quiet,
    );
  }

  let mut failures = Vec::new();
  for target in &targets {
    if let Err(err) =
      execute_single(target, &policy, None, cli_out_dir, write_pdf, quiet)
    {
      failures.push((target.label.clone(), err));
    }
  }

  if failures.is_empty() {
    Ok(())
  } else {
    let mut details = String::new();
    for (i, (label, err)) in failures.iter().enumerate() {
      if i > 0 {
        details.push('\n');
      }
      let _ = write!(details, "* {label}: {err}");
    }
    Err(cli::Error::BatchFailed {
      failed: failures.len(),
      total: targets.len(),
      details,
    })
  }
}

#[cfg(test)]
mod tests {
  use std::fs;
  use std::path;
  use std::sync::atomic;

  use super::*;

  /// Monotonic counter ensuring unique temporary test directories.
  static NEXT_ID: atomic::AtomicU64 = atomic::AtomicU64::new(0);

  /// RAII temporary directory removed automatically on drop.
  struct TempRepo(path::PathBuf);

  impl TempRepo {
    /// Returns the temporary repository path.
    fn path(&self) -> &path::Path {
      &self.0
    }
  }

  impl Drop for TempRepo {
    fn drop(&mut self) {
      let _ = fs::remove_dir_all(&self.0);
    }
  }

  /// Creates an isolated workspace dir with `.git/` so discovery stays local.
  fn make_repo() -> TempRepo {
    let id = NEXT_ID.fetch_add(1, atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir()
      .join(format!("rsmk-test-{}-{id}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join(".git")).expect(".git creatable");
    TempRepo(dir)
  }

  #[test]
  fn option_one_selects_publish_docs_and_allows_draft_by_name_or_all() {
    let repo = make_repo();
    fs::write(
      repo.path().join("resumake.toml"),
      r#"
[telemetry]
max_pages = 2
strict_fonts = false

[docs.main]
content = "content.yaml"
publish = true

[docs.ml]
content = "tracks/ml.yaml"
"#,
    )
    .expect("manifest writable");

    let opts = PipelineOptions::default();

    let (pub_targets, policy) =
      resolve_plan(None, &opts, false, repo.path()).expect("bare build works");
    assert_eq!(pub_targets.len(), 1, "bare build selects only publish=true");
    assert_eq!(pub_targets[0].label, "main", "selected doc is main");
    assert_eq!(policy.max_pages, 2, "manifest telemetry loads max_pages");
    assert!(
      !policy.strict_fonts,
      "manifest telemetry loads strict_fonts"
    );

    let (all_targets, _) =
      resolve_plan(None, &opts, true, repo.path()).expect("check/--all works");
    assert_eq!(
      all_targets.len(),
      2,
      "all/check selects drafts and published"
    );

    let (ml_targets, _) =
      resolve_plan(Some("ml"), &opts, false, repo.path()).expect("named draft");
    assert_eq!(ml_targets.len(), 1, "named draft selects ml");
    assert_eq!(ml_targets[0].label, "ml", "targeted draft selected");
  }

  #[test]
  fn bare_build_fails_when_manifest_docs_have_zero_publish() {
    let repo = make_repo();
    fs::write(
      repo.path().join("resumake.toml"),
      r#"
[docs.draft]
content = "content.yaml"
"#,
    )
    .expect("manifest writable");

    let err =
      resolve_plan(None, &PipelineOptions::default(), false, repo.path())
        .err()
        .expect("bare build must fail cleanly when all docs are drafts");
    assert!(
      matches!(err, cli::Error::Config(_)),
      "returns actionable Config error"
    );
  }

  #[test]
  fn out_dir_stages_flattened_basenames_and_distinct_track_names() {
    let repo = make_repo();
    fs::write(
      repo.path().join("content.yaml"),
      "meta:\n  name: \"Jane Doe\"\n  contact: []\nsections: []\n",
    )
    .expect("content writable");

    let main_target = ResolvedTarget {
      label: "main".to_string(),
      doc_name: Some("main".to_string()),
      content_path: repo.path().join("content.yaml"),
      project_root: repo.path().to_path_buf(),
      template: None,
      schema_path: None,
      font_path: None,
      doc_output: None,
      manifest_out_dir: None,
    };
    let out_dir = path::Path::new(".rsmk/dist");
    assert_eq!(
      resolve_output_pdf(&main_target, None, Some(out_dir)),
      out_dir.join("jane_doe_resume.pdf"),
      "main doc stages as <candidate>_resume.pdf"
    );

    let ml_target = ResolvedTarget {
      label: "ml".to_string(),
      doc_name: Some("ml".to_string()),
      doc_output: None,
      ..main_target
    };
    assert_eq!(
      resolve_output_pdf(&ml_target, None, Some(out_dir)),
      out_dir.join("jane_doe_ml_resume.pdf"),
      "secondary track derives distinct filename"
    );

    let explicit_doc_out = ResolvedTarget {
      doc_output: Some(path::PathBuf::from("nested/custom.pdf")),
      ..ml_target
    };
    assert_eq!(
      resolve_output_pdf(&explicit_doc_out, None, Some(out_dir)),
      out_dir.join("custom.pdf"),
      "explicit --out-dir flattens doc output basename for globs"
    );
  }
}
