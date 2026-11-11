//! End-to-end tests of the library pipeline: scaffold, manifest, validate,
//! compile, and telemetry gates.
//!
//! Exercises only the public API, the way the `rsmk` binary drives it. CLI
//! argument parsing and output live in the binary and are covered there.

use std::fmt;
use std::fs;
use std::path;

use jsonschema;
use resumake;
use serde_json;
use tempfile;
use toml;

/// Scaffolds a starter project in `dir` without git or workflows.
fn scaffold(
  dir: &path::Path,
  format: resumake::ContentFormat,
  no_manifest: bool,
  eject_template: bool,
) -> resumake::scaffold::InitOutcome {
  let options = resumake::scaffold::InitOptions {
    name: "Jane Doe",
    target_dir: dir,
    format,
    no_manifest,
    force: false,
    no_git: true,
    no_workflows: true,
    eject_template,
    template: resumake::templates::DEFAULT_TEMPLATE,
  };
  resumake::init(&options)
    .expect("scaffolding into an empty directory succeeds")
}

#[test]
fn starter_content_and_manifest_pass_validation() {
  let dir = tempfile::tempdir().expect("temp dir is creatable");
  let outcome =
    scaffold(dir.path(), resumake::ContentFormat::Yaml, false, false);

  resumake::validate_file(&outcome.content_path, None)
    .expect("scaffolded content.yaml validates against content schema");

  let manifest_path = outcome
    .manifest_path
    .expect("starter resumake.toml is created by default");
  let manifest = resumake::manifest::load(&manifest_path)
    .expect("starter resumake.toml parses");
  let main_doc = manifest
    .docs
    .get("main")
    .expect("starter manifest defines docs.main");
  assert!(main_doc.publish, "starter docs.main opts into publish");

  let raw_toml = fs::read_to_string(&manifest_path).expect("manifest readable");
  let parsed_toml: toml::Value =
    toml::from_str(&raw_toml).expect("starter manifest is valid TOML");
  let json_manifest =
    serde_json::to_value(parsed_toml).expect("TOML converts to JSON value");
  let manifest_schema = resumake::manifest::generate_schema();
  let validator = jsonschema::validator_for(&manifest_schema)
    .expect("manifest JSON schema compiles");
  assert!(
    validator.is_valid(&json_manifest),
    "starter resumake.toml validates against manifest JSON schema"
  );
}

#[test]
fn starter_content_compiles_to_one_page() {
  let dir = tempfile::tempdir().expect("temp dir is creatable");
  let outcome =
    scaffold(dir.path(), resumake::ContentFormat::Yaml, false, false);

  let (_, report) =
    resumake::compile(&outcome.content_path, None, dir.path(), None)
      .expect("starter résumé compiles with the embedded template");

  assert!(
    report.is_single_page(),
    "the starter résumé fits on one page"
  );
  assert!(
    report.missing_fonts.is_empty(),
    "starter embedded Libertinus Serif font is always present"
  );
}

#[test]
fn json_starter_with_dollar_schema_validates_and_compiles() {
  let dir = tempfile::tempdir().expect("temp dir is creatable");
  let outcome =
    scaffold(dir.path(), resumake::ContentFormat::Json, false, false);

  let raw =
    fs::read_to_string(&outcome.content_path).expect("content.json readable");
  assert!(
    raw.contains("\"$schema\""),
    "starter JSON includes $schema IntelliSense key"
  );

  resumake::validate_file(&outcome.content_path, None)
    .expect("starter content.json validates against content schema");

  let (_, report) =
    resumake::compile(&outcome.content_path, None, dir.path(), None)
      .expect("starter content.json compiles via Typst json()");
  assert!(
    report.is_single_page(),
    "JSON starter résumé fits on one page"
  );
}

#[test]
fn yml_speaking_and_a4_paper_size_compile_accurately() {
  let dir = tempfile::tempdir().expect("temp dir is creatable");
  let path = dir.path().join("cv.yml");
  let content = r#"
meta:
  name: "Jane Doe"
  contact:
    - "jane@example.com"

theme:
  font_family: "Libertinus Serif"
  paper_size: "a4"
  bullet_marker: "dash"
  name_weight: "bold"
  section_weight: "bold"
  section_case: "small-caps"
  section_style: "none"

sections:
  - title: "Talks"
    type: "speaking"
    items:
      - title: "Designing Fast Compilers"
        event: "SystemsConf"
        date: 2025
        link: "https://example.com/talk"
        location: "Berlin"
"#;
  fs::write(&path, content).expect("cv.yml is writable");

  resumake::validate_file(&path, None)
    .expect(".yml file with top-level theme and speaking title validates");

  let (doc, report) = resumake::compile(&path, None, dir.path(), None)
    .expect(".yml file with speaking section and A4 renders cleanly");
  assert!(report.is_single_page(), "A4 speaking résumé is one page");

  let width_pt = doc.pages()[0].frame.width().to_pt();
  let a4_width_pt = 210.0 / 25.4 * 72.0;
  assert!(
    (width_pt - a4_width_pt).abs() < 1.0,
    "paper_size: a4 must render ~595.28pt width, got {width_pt}"
  );
}

#[test]
fn legacy_meta_theme_and_removed_fields_are_rejected() {
  let cases = [
    (
      "meta.version",
      r#"
meta:
  name: "Jane Doe"
  version: "1.0.0"
  contact: ["a@b.c"]
sections: []
"#,
    ),
    (
      "meta.theme",
      r#"
meta:
  name: "Jane Doe"
  contact: ["a@b.c"]
  theme:
    paper_size: "us-letter"
sections: []
"#,
    ),
    (
      "meta.keywords",
      r#"
meta:
  name: "Jane Doe"
  keywords: ["rust"]
  contact: ["a@b.c"]
sections: []
"#,
    ),
    (
      "role.team",
      r#"
meta:
  name: "Jane Doe"
  contact: ["a@b.c"]
sections:
  - title: "Experience"
    experience:
      - org: "Acme"
        roles:
          - title: "Engineer"
            dates: "2024"
            team: "Core"
"#,
    ),
  ];

  for (field_name, yaml) in cases {
    let err = resumake::validate(yaml, resumake::ContentFormat::Yaml, None)
      .expect_err("removed legacy fields must fail strict schema validation");
    assert!(
      matches!(err, resumake::schema::Error::ValidationFailed { .. }),
      "expected ValidationFailed for removed field {field_name}"
    );
  }
}

#[test]
fn multi_page_reports_true_final_page_fill_and_enforces_min_fill() {
  let dir = tempfile::tempdir().expect("temp dir is creatable");
  let path = dir.path().join("content.yaml");

  let mut items = String::new();
  for i in 1..=26 {
    let _ = fmt::Write::write_fmt(
      &mut items,
      format_args!(
        "          - title: \"Engineer {i}\"\n            dates: \
         \"2024\"\n            bullets:\n              \
         - \"Short bullet {i}.\"\n"
      ),
    );
  }
  let yaml = format!(
    "meta:\n  name: \"Jane Doe\"\n  contact: [\"a@b.c\"]\ntheme:\n  \
     font_family: \"Libertinus Serif\"\nsections:\n  - title: \
     \"Experience\"\n    experience:\n      - org: \"Acme\"\n        roles:\n\
     {items}"
  );
  fs::write(&path, yaml).expect("multi-page content writable");

  let (_, report) = resumake::compile(&path, None, dir.path(), None)
    .expect("multi-page résumé compiles");

  assert!(report.page_count >= 2, "document spans multiple pages");
  assert!(
    report.fill_pct < 98.0,
    "final page fill ({}) must not be hardcoded to 100%",
    report.fill_pct
  );
  assert!(
    report.spare_in > 0.2,
    "final page spare inches ({}) must be > 0",
    report.spare_in
  );

  let strict_fill = resumake::TelemetryPolicy {
    max_pages: report.page_count,
    min_fill: Some(99.0),
    ..resumake::TelemetryPolicy::default()
  };
  assert!(
    !strict_fill.evaluate(&report).passed(),
    "min_fill: 99% fails when final page is partially filled"
  );
}

#[test]
fn wrapped_bullet_reports_line_count_and_orphan_tail() {
  let dir = tempfile::tempdir().expect("temp dir is creatable");
  let path = dir.path().join("content.yaml");
  let yaml = r#"
meta:
  name: "Jane Doe"
  contact: ["a@b.c"]
theme:
  font_family: "Libertinus Serif"
sections:
  - title: "Experience"
    experience:
      - org: "Acme"
        roles:
          - title: "Engineer"
            dates: "2024"
            bullets:
              - "Engineered distributed streaming ingestion
                pipeline handling high-throughput workloads
                reliably across regions daily."
"#;
  fs::write(&path, yaml).expect("wrapped bullet content writable");

  let (_, report) = resumake::compile(&path, None, dir.path(), None)
    .expect("wrapped bullet document compiles");
  assert_eq!(report.line_wraps.len(), 1, "one wrapped bullet expected");
  let wrapped = &report.line_wraps[0];
  assert_eq!(wrapped.line_count(), 2, "bullet wrapped to exactly 2 lines");
  let tail = wrapped
    .tail_fill
    .expect("classic.typ reports tail_fill on wrapped bullets");
  assert!(
    tail > 0.0 && tail < resumake::telemetry::ORPHAN_TAIL_PCT,
    "expected short trailing wrap (< 20%), got {tail}%"
  );

  let warn_policy = resumake::TelemetryPolicy {
    max_bullet_lines: 2,
    orphan_lines: resumake::telemetry::OrphanPolicy::Warn,
    ..resumake::TelemetryPolicy::default()
  };
  let warn_out = warn_policy.evaluate(&report);
  assert!(warn_out.passed(), "orphan_lines=warn does not fail build");
  assert_eq!(warn_out.warnings.len(), 1, "emits one orphan warning");

  let error_policy = resumake::TelemetryPolicy {
    max_bullet_lines: 2,
    orphan_lines: resumake::telemetry::OrphanPolicy::Error,
    ..resumake::TelemetryPolicy::default()
  };
  assert!(
    !error_policy.evaluate(&report).passed(),
    "orphan_lines=error fails on short trailing wrap"
  );
}

#[test]
fn strict_fonts_detects_missing_font_family() {
  let dir = tempfile::tempdir().expect("temp dir is creatable");
  let path = dir.path().join("content.yaml");
  let yaml = r#"
meta:
  name: "Jane Doe"
  contact: ["a@b.c"]
theme:
  font_family: "ResumakeNonexistentFontXYZ"
sections: []
"#;
  fs::write(&path, yaml).expect("missing font YAML writable");

  let (_, report) = resumake::compile(&path, None, dir.path(), None)
    .expect("Typst compiles via fallback font list");
  assert_eq!(
    report.missing_fonts,
    vec!["ResumakeNonexistentFontXYZ"],
    "FontBook reports the missing requested font family"
  );

  let strict = resumake::TelemetryPolicy::default();
  assert!(
    !strict.evaluate(&report).passed(),
    "strict_fonts=true rejects missing requested font"
  );

  let relaxed = resumake::TelemetryPolicy {
    strict_fonts: false,
    ..resumake::TelemetryPolicy::default()
  };
  let relaxed_out = relaxed.evaluate(&report);
  assert!(relaxed_out.passed(), "strict_fonts=false permits fallback");
  assert_eq!(relaxed_out.warnings.len(), 1, "warns on font fallback");
}

#[test]
fn manifest_publish_defaults_false_and_discovery_stops_at_git() {
  let manifest = resumake::manifest::parse(
    r#"
[docs.draft]
content = "tracks/draft.yaml"

[docs.pub]
content = "content.yaml"
publish = true
"#,
  )
  .expect("manifest parses");
  assert!(
    !manifest.docs["draft"].publish,
    "publish defaults to false on [docs.<name>] entries"
  );
  assert!(
    manifest.docs["pub"].publish,
    "explicit publish=true sets true"
  );

  let unknown = resumake::manifest::parse("[telemetry]\nmax_fill = 85.0\n");
  assert!(
    unknown.is_err(),
    "unknown fields in resumake.toml are rejected"
  );

  let dir = tempfile::tempdir().expect("temp dir is creatable");
  fs::write(dir.path().join("resumake.toml"), "").expect("outer manifest");
  let repo = dir.path().join("inner");
  let nested = repo.join("tracks");
  fs::create_dir_all(repo.join(".git")).expect(".git dir creatable");
  fs::create_dir_all(&nested).expect("nested dir creatable");

  assert!(
    resumake::manifest::discover(&nested).is_none(),
    "manifest discovery halts at inner .git boundary"
  );
}

#[test]
fn ejected_template_compiles_from_disk() {
  // Regression: ejected templates once failed to resolve from disk because
  // their absolute path was used as the Typst virtual path.
  let dir = tempfile::tempdir().expect("temp dir is creatable");
  let outcome =
    scaffold(dir.path(), resumake::ContentFormat::Yaml, false, true);

  let source =
    resumake::resolve(resumake::templates::DEFAULT_TEMPLATE, dir.path())
      .expect("the ejected template resolves");
  assert!(
    matches!(source, resumake::templates::TemplateSource::File(_)),
    "a template in templates/ wins over the embedded copy"
  );

  resumake::compile(&outcome.content_path, None, dir.path(), None)
    .expect("the résumé compiles with the ejected template");
}

#[test]
fn second_init_without_force_is_refused() {
  let dir = tempfile::tempdir().expect("temp dir is creatable");
  scaffold(dir.path(), resumake::ContentFormat::Yaml, false, false);

  let options = resumake::scaffold::InitOptions {
    name: "Jane Doe",
    target_dir: dir.path(),
    format: resumake::ContentFormat::Yaml,
    no_manifest: false,
    force: false,
    no_git: true,
    no_workflows: true,
    eject_template: false,
    template: resumake::templates::DEFAULT_TEMPLATE,
  };
  let err = resumake::init(&options)
    .expect_err("re-initializing an existing project needs force");
  assert!(
    matches!(err, resumake::scaffold::Error::AlreadyExists { .. }),
    "the refusal names the existing file"
  );
}

#[test]
fn unknown_template_is_rejected() {
  let dir = tempfile::tempdir().expect("temp dir is creatable");

  let err = resumake::resolve("no-such-template", dir.path())
    .expect_err("an unknown template name does not resolve");
  assert!(
    matches!(err, resumake::templates::Error::NotFound { .. }),
    "the error lists the known templates"
  );
}

#[test]
fn scaffolded_workflows_pin_the_setup_action_and_stage_dist() {
  let pinned = format!("setup@{}", env!("CARGO_PKG_VERSION"));

  for workflow in [
    resumake::scaffold::ci_workflow(),
    resumake::scaffold::release_workflow(),
  ] {
    assert!(
      workflow.contains(&pinned),
      "the setup action ref is a tag that exists for this release"
    );
  }

  let release = resumake::scaffold::release_workflow();
  assert!(
    release.contains("rsmk build --out-dir .rsmk/dist"),
    "release workflow stages outputs into .rsmk/dist"
  );
  assert!(
    release.contains(".rsmk/dist/*.pdf"),
    "release workflow attaches .rsmk/dist/*.pdf"
  );
}

#[test]
fn committed_schemas_match_generated_output() {
  let schemas_dir =
    path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas");

  let committed_content: serde_json::Value = serde_json::from_str(
    &fs::read_to_string(schemas_dir.join("content.schema.json"))
      .expect("schemas/content.schema.json readable"),
  )
  .expect("schemas/content.schema.json parses as JSON");
  assert_eq!(
    committed_content,
    resumake::generate_schema(),
    "schemas/content.schema.json must match resumake::generate_schema()"
  );

  let committed_manifest: serde_json::Value = serde_json::from_str(
    &fs::read_to_string(schemas_dir.join("manifest.schema.json"))
      .expect("schemas/manifest.schema.json readable"),
  )
  .expect("schemas/manifest.schema.json parses as JSON");
  assert_eq!(
    committed_manifest,
    resumake::manifest::generate_schema(),
    "schemas/manifest.schema.json must match \
     resumake::manifest::generate_schema()"
  );
}
