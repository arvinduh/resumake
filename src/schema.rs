//! In-process JSON Schema validation, version extraction, and schema
//! export for content.yaml.

use crate::models::generate_builtin_schema;
use std::fs;
use std::path::{Path, PathBuf};

const INIT_TEMPLATE_RAW: &str = include_str!("embedded/init_template.yaml");

/// Raw embedded template for the GitHub Actions CI workflow stub.
pub const CI_WORKFLOW_RAW: &str = include_str!("embedded/workflows/ci.yml");

/// Raw embedded template for the GitHub Actions release workflow stub.
pub const RELEASE_WORKFLOW_RAW: &str =
  include_str!("embedded/workflows/release.yml");

/// Validates a content YAML file against an optional JSON schema file.
///
/// If `schema_path` is `None` or the file does not exist, the built-in
/// canonical schema derived from [`crate::models::ResumeDocument`] is
/// used.
/// Validated with the `jsonschema` crate, which auto-detects the draft from
/// the schema's `$schema` field (the built-in schema is Draft-07).
///
/// Every model struct in `src/models.rs` carries
/// `#[serde(deny_unknown_fields)]`, which `schemars` turns into
/// `"additionalProperties": false` on the generated schema — so a renamed
/// or misspelled field (the exact class of bug this attribute was added
/// for — see `docs/schema-guide.md`) fails loudly here instead of being
/// silently dropped, with no separate check needed beyond the schema
/// itself. The same `additionalProperties: false` is what makes IDE YAML
/// plugins flag it live, since they validate against this same schema.
///
/// One real gap is worth knowing: [`crate::models::Section`] accepts
/// content through either a strongly-typed shorthand field (`education`,
/// `experience`, ...) or a generic `items: serde_json::Value` fallback
/// used with an explicit `type:`. `items` is intentionally untyped so any
/// block's content can be supplied that way, so it has no fixed
/// `properties` in the schema and a typo inside it is not caught — only
/// the shorthand form is.
///
/// # Errors
/// Returns a list of formatted validation error strings if the YAML is
/// invalid or does not conform to the schema.
pub fn validate_schema_auto(
  content_path: &Path,
  schema_path: Option<&Path>,
) -> Result<(), Vec<String>> {
  // Step 1: Resolve the JSON schema Value (from custom file path if
  // exists, otherwise generate_builtin_schema())
  let schema_json: serde_json::Value = match schema_path {
    Some(p) if p.exists() => {
      let schema_str = fs::read_to_string(p).map_err(|e| {
        vec![format!(
          "Failed to read schema file '{}': {}",
          p.display(),
          e
        )]
      })?;
      serde_json::from_str(&schema_str).map_err(|e| {
        vec![format!(
          "Failed to parse schema JSON from '{}': {}",
          p.display(),
          e
        )]
      })?
    }
    _ => generate_builtin_schema(),
  };

  // Step 2: Read and deserialize the content YAML file into a serde_json::Value
  let content_str = fs::read_to_string(content_path).map_err(|e| {
    vec![format!(
      "Failed to read content file '{}': {}",
      content_path.display(),
      e
    )]
  })?;

  let content_json: serde_json::Value = serde_yaml::from_str(&content_str)
    .map_err(|e| {
      vec![format!(
        "Failed to parse content YAML from '{}': {}",
        content_path.display(),
        e
      )]
    })?;

  // Step 3: Compile validator and collect error messages
  let validator = jsonschema::validator_for(&schema_json).map_err(|e| {
    vec![format!("Failed to compile JSON schema validator: {}", e)]
  })?;

  let mut errors = Vec::new();
  for error in validator.iter_errors(&content_json) {
    let instance_path = error.instance_path().to_string();
    let location = if instance_path.is_empty() {
      "root".to_string()
    } else {
      instance_path
    };
    errors.push(format!(
      "Schema validation error at {}: {}",
      location, error
    ));
  }

  if errors.is_empty() {
    Ok(())
  } else {
    Err(errors)
  }
}

/// Exports the built-in JSON schema to a file or returns it as a
/// formatted string.
///
/// # Errors
/// Returns an error string if serialization or disk write fails.
pub fn export_builtin_schema(
  output_path: Option<&Path>,
) -> Result<String, String> {
  let schema_json = generate_builtin_schema();
  let schema_str = serde_json::to_string_pretty(&schema_json)
    .map_err(|e| format!("Failed to format JSON schema: {}", e))?;

  if let Some(path) = output_path {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
      fs::create_dir_all(parent).map_err(|e| {
        format!(
          "Failed to create schema directory '{}': {}",
          parent.display(),
          e
        )
      })?;
    }
    fs::write(path, &schema_str).map_err(|e| {
      format!("Failed to write schema to '{}': {}", path.display(), e)
    })?;
  }

  Ok(schema_str)
}

/// GitHub Release asset URL for the JSON Schema matching this schema version.
///
/// Published under schema releases (e.g. `s1.0`, `s1.1`) matching Formality's
/// schema distribution pattern.
fn schema_url() -> String {
  "https://github.com/arvinduh/resumake/releases/download/s1.0/resume.schema.json".to_string()
}

/// Generates a starter `resume.yaml` scaffold with schema directives and
/// examples.
pub fn generate_init_template(candidate_name: &str) -> String {
  let name = if candidate_name.trim().is_empty() {
    "Jane Doe"
  } else {
    candidate_name.trim()
  };

  INIT_TEMPLATE_RAW
    .replace("CANDIDATE_NAME", name)
    .replace("RESUMAKE_SCHEMA_URL", &schema_url())
}

/// Generates a starter GitHub Actions CI workflow stub for downstream repositories.
///
/// Substitutes `RSMK_VERSION` with `version` if provided and non-empty,
/// otherwise defaults to the current binary version ([`env!("CARGO_PKG_VERSION")`]).
pub fn generate_ci_workflow(version: Option<&str>) -> String {
  let v = match version {
    Some(s) if !s.trim().is_empty() => s.trim(),
    _ => env!("CARGO_PKG_VERSION"),
  };
  CI_WORKFLOW_RAW.replace("RSMK_VERSION", v)
}

/// Generates a starter GitHub Actions release workflow stub for downstream repositories.
///
/// Substitutes `RSMK_VERSION` with `version` if provided and non-empty,
/// otherwise defaults to the current binary version ([`env!("CARGO_PKG_VERSION")`]).
pub fn generate_release_workflow(version: Option<&str>) -> String {
  let v = match version {
    Some(s) if !s.trim().is_empty() => s.trim(),
    _ => env!("CARGO_PKG_VERSION"),
  };
  RELEASE_WORKFLOW_RAW.replace("RSMK_VERSION", v)
}

/// Loads the semantic version string (`meta.version`) from a content YAML file.
pub fn load_content_version(content_path: &Path) -> Result<String, String> {
  let content_str = fs::read_to_string(content_path).map_err(|e| {
    format!(
      "Failed to read content file '{}': {}",
      content_path.display(),
      e
    )
  })?;

  let val: serde_yaml::Value =
    serde_yaml::from_str(&content_str).map_err(|e| {
      format!(
        "Failed to parse YAML from '{}': {}",
        content_path.display(),
        e
      )
    })?;

  let version_val =
    val
      .get("meta")
      .and_then(|m| m.get("version"))
      .ok_or_else(|| {
        format!(
          "Missing 'meta.version' field in '{}'",
          content_path.display()
        )
      })?;

  let version_str = version_val
    .as_str()
    .ok_or_else(|| "Field 'meta.version' is not a string".to_string())?;

  Ok(version_str.to_string())
}

/// Loads the author's name (`meta.name`) from a content YAML file.
pub fn load_content_name(content_path: &Path) -> Result<String, String> {
  let content_str = fs::read_to_string(content_path).map_err(|e| {
    format!(
      "Failed to read content file '{}': {}",
      content_path.display(),
      e
    )
  })?;

  let val: serde_yaml::Value =
    serde_yaml::from_str(&content_str).map_err(|e| {
      format!(
        "Failed to parse YAML from '{}': {}",
        content_path.display(),
        e
      )
    })?;

  let name_val =
    val.get("meta").and_then(|m| m.get("name")).ok_or_else(|| {
      format!("Missing 'meta.name' field in '{}'", content_path.display())
    })?;

  let name_str = name_val
    .as_str()
    .ok_or_else(|| "Field 'meta.name' is not a string".to_string())?;

  Ok(name_str.to_string())
}

/// Derives the output PDF filename from the résumé author's name in
/// `content.yaml`.
///
/// For example, "Jane Doe" becomes `janedoe_resume.pdf`.
pub fn derive_output_filename(content_path: &Path) -> PathBuf {
  if let Ok(name) = load_content_name(content_path) {
    let sanitized: String = name
      .chars()
      .filter(|c| c.is_alphanumeric())
      .collect::<String>()
      .to_lowercase();
    if !sanitized.is_empty() {
      return PathBuf::from(format!("{}_resume.pdf", sanitized));
    }
  }
  PathBuf::from("resume.pdf")
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::TempDir;

  #[test]
  fn test_validate_schema_auto_builtin() {
    let temp = TempDir::new().unwrap();
    let content_file = temp.path().join("content.yaml");

    fs::write(
      &content_file,
      r#"
meta:
  name: "Test User"
  version: "1.2.3"
  contact:
    - name: "test@example.com"
sections: []
"#,
    )
    .unwrap();

    assert!(validate_schema_auto(&content_file, None).is_ok());
  }

  #[test]
  fn test_validate_schema_auto_catches_invalid_yaml() {
    let temp = TempDir::new().unwrap();
    let content_file = temp.path().join("content.yaml");

    fs::write(
      &content_file,
      r#"
meta:
  version: "1.0.0"
"#,
    )
    .unwrap();

    let res = validate_schema_auto(&content_file, None);
    assert!(res.is_err());
  }

  #[test]
  fn test_export_builtin_schema() {
    let temp = TempDir::new().unwrap();
    let out_file = temp.path().join("schema.json");
    let res = export_builtin_schema(Some(&out_file));
    assert!(res.is_ok());
    assert!(out_file.exists());
    let content = fs::read_to_string(&out_file).unwrap();
    assert!(content.contains("ResumeDocument"));
  }

  #[test]
  fn test_generate_init_template() {
    let tmpl = generate_init_template("Jane Doe");
    assert!(tmpl.contains("Jane Doe"));
    assert!(tmpl.contains("yaml-language-server"));
    assert!(tmpl.contains("sections:"));
    assert!(tmpl.contains("Libertinus Serif"));
    assert!(!tmpl.contains("Linux Libertine"));
    // The schema URL placeholder must be substituted with a concrete,
    // schema-pinned release-asset URL, not left dangling.
    assert!(!tmpl.contains("RESUMAKE_SCHEMA_URL"));
    assert!(tmpl.contains("/releases/download/s1.0"));
    assert!(tmpl.contains("resume.schema.json"));
  }

  #[test]
  fn test_init_scaffold_matches_current_schema() {
    // Regression guard for the schema/model drift class of bug: the
    // scaffold `resumake init` generates today must always validate
    // against the schema `resumake` derives from its own models today.
    // This does not need Typst, so it stays fast enough to run on every
    // `cargo test`, ahead of the slower end-to-end compile test in
    // tests/cli.rs.
    let temp = TempDir::new().unwrap();
    let content_file = temp.path().join("content.yaml");
    fs::write(&content_file, generate_init_template("Jane Doe")).unwrap();

    let result = validate_schema_auto(&content_file, None);
    assert!(
      result.is_ok(),
      "init scaffold no longer matches the current schema: {result:?}"
    );
  }

  #[test]
  fn test_load_name_version_and_derive_filename() {
    let temp = TempDir::new().unwrap();
    let content_file = temp.path().join("content.yaml");

    let yaml = r#"
meta:
  name: "Jane Doe"
  version: "2.1.0"
sections: []
"#;
    fs::write(&content_file, yaml).unwrap();

    assert_eq!(load_content_name(&content_file).unwrap(), "Jane Doe");
    assert_eq!(load_content_version(&content_file).unwrap(), "2.1.0");
    assert_eq!(
      derive_output_filename(&content_file),
      PathBuf::from("janedoe_resume.pdf")
    );
  }

  #[test]
  fn test_validate_schema_auto_fails_on_renamed_field() {
    // Regression test for the exact bug that motivated
    // `deny_unknown_fields`: an old `meta.role` (the model now expects
    // `meta.title`) must fail loudly, not silently vanish.
    let temp = TempDir::new().unwrap();
    let content_file = temp.path().join("content.yaml");
    fs::write(
      &content_file,
      r#"
meta:
  name: "Jane Doe"
  version: "1.0.0"
  role: "Staff Engineer"
  contact:
    - name: "jane@example.com"
sections: []
"#,
    )
    .unwrap();

    let errors = validate_schema_auto(&content_file, None).unwrap_err();
    assert!(errors.iter().any(|e| e.contains("role")));
  }

  #[test]
  fn test_validate_schema_auto_fails_on_typo() {
    let temp = TempDir::new().unwrap();
    let content_file = temp.path().join("content.yaml");
    fs::write(
      &content_file,
      r#"
meta:
  name: "Jane Doe"
  version: "1.0.0"
  titel: "Staff Engineer"
  contact:
    - name: "jane@example.com"
sections: []
"#,
    )
    .unwrap();

    let errors = validate_schema_auto(&content_file, None).unwrap_err();
    assert!(errors.iter().any(|e| e.contains("titel")));
  }

  #[test]
  fn test_validate_schema_auto_with_meta_extra() {
    let temp = TempDir::new().unwrap();
    let content_file = temp.path().join("content.yaml");
    fs::write(
      &content_file,
      r#"
meta:
  name: "Jane Doe"
  version: "1.0.0"
  contact:
    - name: "jane@example.com"
  extra:
    status: "active"
    years_experience: 10
    verified: true
    skills_tags:
      - "rust"
      - "distributed-systems"
    attributes:
      clearance: "Secret"
      relocation: false
sections: []
"#,
    )
    .unwrap();

    assert!(validate_schema_auto(&content_file, None).is_ok());
  }

  #[test]
  fn test_validate_schema_auto_does_not_check_generic_items_payload() {
    // Documents the known coverage gap: `items:` is an untyped
    // passthrough (see `Section::items`), so a typo inside it is not
    // caught by `deny_unknown_fields` the way a typo in the strongly
    // typed `education:`/`experience:`/... shorthand fields is. This
    // test exists so that gap is a documented, intentional trade-off
    // rather than a silent regression if it's ever "fixed" by accident
    // in a way that changes behavior without anyone noticing.
    let temp = TempDir::new().unwrap();
    let content_file = temp.path().join("content.yaml");
    fs::write(
      &content_file,
      r#"
meta:
  name: "Jane Doe"
  version: "1.0.0"
  contact:
    - name: "jane@example.com"
  extra:
    custom_field: "allowed"
sections:
  - title: "Education"
    type: "education"
    items:
      insitution: "Typo'd field name"
"#,
    )
    .unwrap();

    assert!(validate_schema_auto(&content_file, None).is_ok());
  }

  #[test]
  fn test_generate_ci_workflow() {
    // Test default binary version
    let default_ci = generate_ci_workflow(None);
    assert!(!default_ci.contains("RSMK_VERSION"));
    assert!(default_ci.contains(env!("CARGO_PKG_VERSION")));
    assert!(default_ci.contains("uses: actions/checkout@v4"));
    assert!(default_ci.contains("uses: arvinduh/resumake/setup@v1"));
    assert!(default_ci.contains("run: rsmk build --check"));

    // Ensure it is valid YAML
    let parsed: serde_yaml::Value = serde_yaml::from_str(&default_ci).unwrap();
    assert_eq!(parsed["name"], "CI");

    // Test explicit version override
    let custom_ci = generate_ci_workflow(Some("0.9.9"));
    assert!(custom_ci.contains(r#"version: "0.9.9""#));
  }

  #[test]
  fn test_generate_release_workflow() {
    // Test default binary version
    let default_release = generate_release_workflow(None);
    assert!(!default_release.contains("RSMK_VERSION"));
    assert!(default_release.contains(env!("CARGO_PKG_VERSION")));
    assert!(default_release.contains("uses: actions/checkout@v4"));
    assert!(default_release.contains("uses: arvinduh/resumake/setup@v1"));
    assert!(default_release.contains("run: rsmk build"));
    assert!(default_release.contains("uses: softprops/action-gh-release@v2"));
    assert!(default_release.contains(r#"files: "*.pdf""#));

    // Ensure it is valid YAML
    let parsed: serde_yaml::Value =
      serde_yaml::from_str(&default_release).unwrap();
    assert_eq!(parsed["name"], "Release");

    // Test explicit version override
    let custom_release = generate_release_workflow(Some("1.2.3"));
    assert!(custom_release.contains(r#"version: "1.2.3""#));
  }

  #[test]
  fn test_setup_action_is_valid_yaml() {
    let action_str = include_str!("../setup/action.yml");
    let parsed: serde_yaml::Value = serde_yaml::from_str(action_str).unwrap();
    assert_eq!(parsed["name"], "Setup Resumake (rsmk)");
    assert_eq!(parsed["runs"]["using"], "composite");
  }
}
