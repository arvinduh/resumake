//! End-to-end tests of the library pipeline: scaffold, validate, compile.
//!
//! Exercises only the public API, the way the `rsmk` binary drives it. CLI
//! argument parsing and output live in the binary and are not covered here.

use std::path;

use resumake;
use tempfile;

/// Scaffolds a starter project in `dir` without git or workflows.
fn scaffold(dir: &path::Path, eject_template: bool) -> path::PathBuf {
  let options = resumake::scaffold::InitOptions {
    name: "Jane Doe",
    target_dir: dir,
    force: false,
    no_git: true,
    no_workflows: true,
    eject_template,
    template: resumake::templates::DEFAULT_TEMPLATE,
  };
  resumake::init(&options)
    .expect("scaffolding into an empty directory succeeds")
    .content_path
}

#[test]
fn starter_content_passes_the_schema() {
  let dir = tempfile::tempdir().expect("temp dir is creatable");
  let content = scaffold(dir.path(), false);

  resumake::validate_file(&content, None)
    .expect("the scaffolded content.yaml validates against the schema");
}

#[test]
fn starter_content_compiles_to_one_page() {
  let dir = tempfile::tempdir().expect("temp dir is creatable");
  let content = scaffold(dir.path(), false);

  let (_, report) = resumake::compile(&content, None, dir.path(), None)
    .expect("the starter résumé compiles with the embedded template");

  assert!(
    report.is_single_page(),
    "the starter résumé fits on one page"
  );
}

#[test]
fn ejected_template_compiles_from_disk() {
  // Regression: ejected templates once failed to resolve from disk because
  // their absolute path was used as the Typst virtual path.
  let dir = tempfile::tempdir().expect("temp dir is creatable");
  let content = scaffold(dir.path(), true);

  let source =
    resumake::resolve(resumake::templates::DEFAULT_TEMPLATE, dir.path())
      .expect("the ejected template resolves");
  assert!(
    matches!(source, resumake::templates::TemplateSource::File(_)),
    "a template in templates/ wins over the embedded copy"
  );

  resumake::compile(&content, None, dir.path(), None)
    .expect("the résumé compiles with the ejected template");
}

#[test]
fn second_init_without_force_is_refused() {
  let dir = tempfile::tempdir().expect("temp dir is creatable");
  scaffold(dir.path(), false);

  let options = resumake::scaffold::InitOptions {
    name: "Jane Doe",
    target_dir: dir.path(),
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
fn scaffolded_workflows_pin_the_setup_action_to_this_release() {
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
}
