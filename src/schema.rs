//! In-process JSON Schema validation, version extraction, and schema export for content.yaml.

use crate::models::generate_builtin_schema;
use std::fs;
use std::path::{Path, PathBuf};

const INIT_TEMPLATE_RAW: &str = include_str!("embedded/init_template.yaml");

/// Validates a content YAML file against an optional JSON schema file.
///
/// If `schema_path` is `None` or the file does not exist, the built-in canonical schema
/// derived from [`crate::models::ResumeDocument`] is used.
/// Uses the `jsonschema` crate with Draft 2020-12 support.
///
/// # Errors
/// Returns a list of formatted validation error strings if the YAML is invalid or does not conform.
pub fn validate_schema_auto(
  content_path: &Path,
  schema_path: Option<&Path>,
) -> Result<(), Vec<String>> {
  // Step 1: Resolve the JSON schema Value (from custom file path if exists, otherwise generate_builtin_schema())
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
    let instance_path = error.instance_path.to_string();
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

/// Exports the built-in JSON schema to a file or returns it as a formatted string.
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

/// Generates a starter `resume.yaml` scaffold with schema directives and examples.
pub fn generate_init_template(candidate_name: &str) -> String {
  let name = if candidate_name.trim().is_empty() {
    "Jane Doe"
  } else {
    candidate_name.trim()
  };

  INIT_TEMPLATE_RAW.replace("CANDIDATE_NAME", name)
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

/// Derives the output PDF filename from the résumé author's name in `content.yaml`.
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
}
