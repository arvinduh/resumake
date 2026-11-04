//! Résumé document schema models, JSON Schema generation, and validation.
//!
//! Owns strongly-typed representations of résumé YAML documents, Draft-07 JSON
//! Schema generation via `schemars`, and document validation via `jsonschema`.

use std::collections;
use std::fs;
use std::path;

use jsonschema;
use schemars;
use serde;
use serde_json;
use serde_yaml;
use thiserror;

/// Errors originating from schema validation and document parsing.
#[derive(Debug, thiserror::Error)]
pub enum Error {
  /// YAML parsing failure.
  #[error("failed to parse YAML content at '{path}': {source}")]
  YamlParse {
    /// Target file path.
    path: path::PathBuf,
    /// Underlying YAML error.
    #[source]
    source: serde_yaml::Error,
  },
  /// JSON parsing failure.
  #[error("failed to parse JSON schema: {0}")]
  JsonParse(#[from] serde_json::Error),
  /// Content failed JSON Schema validation.
  #[error("content failed schema validation:\n{details}")]
  ValidationFailed {
    /// Aggregated validation diagnostics.
    details: String,
  },
  /// Standard I/O error reading content or schema files.
  #[error(transparent)]
  Io(#[from] std::io::Error),
}

/// Metadata extracted from a résumé document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentMeta {
  /// Candidate name from `meta.name`.
  pub name: String,
  /// Document version from `meta.version`.
  pub version: String,
}

/// Top-level Résumé document root.
#[derive(
  Debug,
  Clone,
  serde::Serialize,
  serde::Deserialize,
  schemars::JsonSchema,
  PartialEq,
)]
#[serde(deny_unknown_fields)]
pub struct ResumeDocument {
  /// Document metadata, contact details, ATS keywords, and theme tokens.
  pub meta: Meta,
  /// Ordered list of modular résumé sections.
  pub sections: Vec<Section>,
}

/// Metadata, author info, ATS targets, contact items, and styling theme.
#[derive(
  Debug,
  Clone,
  serde::Serialize,
  serde::Deserialize,
  schemars::JsonSchema,
  PartialEq,
)]
#[serde(deny_unknown_fields)]
pub struct Meta {
  /// Full display name of the candidate.
  pub name: String,
  /// Semantic milestone version of the résumé (e.g. "1.0.0").
  pub version: String,
  /// Professional headline or role tagline.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub title: Option<String>,
  /// Short bio or document summary.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  /// Availability badge, clearance, or work authorization status.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub badge: Option<String>,
  /// Keywords embedded into PDF document metadata for ATS optimization.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub keywords: Option<Vec<String>>,
  /// Contact items (email, phone, `LinkedIn`, GitHub, portfolio, etc.).
  pub contact: Vec<ContactItem>,
  /// Visual styling tokens (typography, margins, colors, paper size).
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub theme: Option<ThemeConfig>,
  /// Extensible custom metadata key-values.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub extra: Option<collections::BTreeMap<String, serde_json::Value>>,
}

/// A contact item representation (string or structured object).
#[derive(
  Debug,
  Clone,
  serde::Serialize,
  serde::Deserialize,
  schemars::JsonSchema,
  PartialEq,
)]
#[serde(untagged)]
pub enum ContactItem {
  /// Raw string label (e.g. "(555) 019-2834").
  Simple(String),
  /// Structured contact item with label and optional URI link.
  Detailed {
    /// Display name or label.
    #[serde(alias = "label", alias = "value")]
    name: String,
    /// URI link (e.g. "mailto:...", "https://...").
    #[serde(alias = "url", default, skip_serializing_if = "Option::is_none")]
    link: Option<String>,
    /// Optional icon identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    icon: Option<String>,
  },
}

/// Theme design tokens configurable directly via YAML.
#[derive(
  Debug,
  Clone,
  serde::Serialize,
  serde::Deserialize,
  schemars::JsonSchema,
  PartialEq,
  Default,
)]
#[serde(deny_unknown_fields)]
pub struct ThemeConfig {
  /// Pinned font family name (e.g. "Libertinus Serif", "Inter").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub font_family: Option<String>,
  /// Paper size dimension preset ("us-letter", "a4").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub paper_size: Option<String>,
  /// Page margin dimension (e.g. "0.5in", "0.4in", "36pt").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub margin: Option<String>,
  /// Primary hex or named brand accent color (e.g. "#2a2a2a", "#1a365d").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub accent_color: Option<String>,
  /// Primary base body font size (e.g. "11pt", "10.5pt").
  #[serde(
    alias = "font_size",
    default,
    skip_serializing_if = "Option::is_none"
  )]
  pub body_size: Option<String>,
  /// Candidate name display size (e.g. "24pt", "26pt").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub name_size: Option<String>,
  /// Organization / employer heading size (e.g. "12pt").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub org_size: Option<String>,
  /// Section header font size (e.g. "13pt").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub section_size: Option<String>,
  /// Section divider line stroke thickness (e.g. "0.5pt", "0pt").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub rule_thickness: Option<String>,
  /// Adaptive vertical spacing base unit (e.g. "0.52em").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub leading: Option<String>,
  /// Adaptive space elasticity multiplier (e.g. 1.0, 0.0).
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub stretch: Option<f64>,
  /// Secondary text color for dates, locations, and metadata.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub muted_color: Option<String>,
  /// Header alignment ("center" or "left").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub header_align: Option<String>,
  /// Spacing density preset ("compact", "comfortable", "spacious").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub density: Option<String>,
}

/// Generic string or list of strings.
#[derive(
  Debug,
  Clone,
  serde::Serialize,
  serde::Deserialize,
  schemars::JsonSchema,
  PartialEq,
)]
#[serde(untagged)]
pub enum StringOrList {
  /// Single string.
  Single(String),
  /// List of strings.
  List(Vec<String>),
}

/// Generic string or number (for years / counts).
#[derive(
  Debug,
  Clone,
  serde::Serialize,
  serde::Deserialize,
  schemars::JsonSchema,
  PartialEq,
)]
#[serde(untagged)]
pub enum StringOrNumber {
  /// String value.
  Str(String),
  /// Numeric integer value.
  Num(i64),
}

/// A modular résumé section supporting polymorphic block types.
#[derive(
  Debug,
  Clone,
  serde::Serialize,
  serde::Deserialize,
  schemars::JsonSchema,
  PartialEq,
)]
#[serde(deny_unknown_fields)]
pub struct Section {
  /// Section header title.
  pub title: String,
  /// Explicit block type descriptor.
  #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
  pub section_type: Option<String>,
  /// Generic items payload.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub items: Option<serde_json::Value>,
  /// Education block shorthand.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub education: Option<EducationValue>,
  /// Skills block shorthand.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub skills: Option<SkillsValue>,
  /// Work experience block shorthand.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub experience: Option<Vec<ExperienceItem>>,
  /// Technical projects block shorthand.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub projects: Option<Vec<ProjectItem>>,
  /// Academic publications and preprints shorthand.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub publications: Option<Vec<PublicationItem>>,
  /// Professional certifications and licenses shorthand.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub certifications: Option<Vec<CertificationItem>>,
  /// Awards and honors shorthand.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub awards: Option<Vec<AwardItem>>,
  /// Speaking engagements shorthand.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub speaking: Option<Vec<SpeakingItem>>,
  /// References block shorthand.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub references: Option<ReferencesValue>,
  /// Generic key-value dictionary lines shorthand.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub lines: Option<collections::BTreeMap<String, String>>,
  /// Generic bullet points shorthand.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub bullets: Option<Vec<String>>,
  /// Freeform markdown prose paragraph.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub text: Option<String>,
}

/// Polymorphic representation of education entries.
#[derive(
  Debug,
  Clone,
  serde::Serialize,
  serde::Deserialize,
  schemars::JsonSchema,
  PartialEq,
)]
#[serde(untagged)]
pub enum EducationValue {
  /// Multiple education entries.
  List(Vec<EducationItem>),
  /// Single education entry.
  Single(EducationItem),
}

/// Higher education degree or certification program.
#[derive(
  Debug,
  Clone,
  serde::Serialize,
  serde::Deserialize,
  schemars::JsonSchema,
  PartialEq,
  Default,
)]
#[serde(deny_unknown_fields)]
pub struct EducationItem {
  /// Degree title.
  pub degree: String,
  /// College, university, or granting institution name.
  pub institution: String,
  /// City, state, or country.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub location: Option<String>,
  /// Graduation date or active enrollment range.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub dates: Option<StringOrNumber>,
  /// Cumulative GPA.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub gpa: Option<StringOrNumber>,
  /// Academic honors.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub honors: Option<StringOrList>,
  /// Relevant coursework.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub coursework: Option<StringOrList>,
  /// Highlights or key achievements.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub highlights: Option<Vec<String>>,
}

/// Skills representation (dictionary or list).
#[derive(
  Debug,
  Clone,
  serde::Serialize,
  serde::Deserialize,
  schemars::JsonSchema,
  PartialEq,
)]
#[serde(untagged)]
pub enum SkillsValue {
  /// Key-value skill categories (e.g. "Languages: Rust, Python").
  Dict(collections::BTreeMap<String, StringOrList>),
  /// Structured list of skills.
  List(Vec<SkillCategoryItem>),
}

/// Structured skill category.
#[derive(
  Debug,
  Clone,
  serde::Serialize,
  serde::Deserialize,
  schemars::JsonSchema,
  PartialEq,
)]
#[serde(deny_unknown_fields)]
pub struct SkillCategoryItem {
  /// Category name.
  pub category: String,
  /// Comma-separated or listed skill items.
  pub skills: StringOrList,
}

/// Professional work experience role.
#[derive(
  Debug,
  Clone,
  serde::Serialize,
  serde::Deserialize,
  schemars::JsonSchema,
  PartialEq,
  Default,
)]
#[serde(deny_unknown_fields)]
pub struct ExperienceItem {
  /// Employer, organization, or client name.
  pub org: String,
  /// Job title or appointment.
  pub role: String,
  /// Office location.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub location: Option<String>,
  /// Employment date range.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub dates: Option<StringOrNumber>,
  /// Summary description.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub summary: Option<String>,
  /// Bulleted impact statements.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub highlights: Option<Vec<String>>,
}

/// Software project, open source repository, or technical portfolio entry.
#[derive(
  Debug,
  Clone,
  serde::Serialize,
  serde::Deserialize,
  schemars::JsonSchema,
  PartialEq,
  Default,
)]
#[serde(deny_unknown_fields)]
pub struct ProjectItem {
  /// Project title.
  pub name: String,
  /// Role or leadership position.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub role: Option<String>,
  /// Dates or timeline.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub dates: Option<StringOrNumber>,
  /// Project URI.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub url: Option<String>,
  /// Description summary.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub summary: Option<String>,
  /// Bullets of achievements.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub highlights: Option<Vec<String>>,
}

/// Academic publication, conference paper, or preprint.
#[derive(
  Debug,
  Clone,
  serde::Serialize,
  serde::Deserialize,
  schemars::JsonSchema,
  PartialEq,
  Default,
)]
#[serde(deny_unknown_fields)]
pub struct PublicationItem {
  /// Publication title.
  pub title: String,
  /// Journal, proceedings, or publisher name.
  pub venue: String,
  /// Publication year or date.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub date: Option<StringOrNumber>,
  /// Co-authors list.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub authors: Option<StringOrList>,
  /// DOI or URL link.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub url: Option<String>,
}

/// Professional certification or credential.
#[derive(
  Debug,
  Clone,
  serde::Serialize,
  serde::Deserialize,
  schemars::JsonSchema,
  PartialEq,
  Default,
)]
#[serde(deny_unknown_fields)]
pub struct CertificationItem {
  /// Certification title.
  pub name: String,
  /// Issuing organization.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub issuer: Option<String>,
  /// Issue date or validity range.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub date: Option<StringOrNumber>,
}

/// Award or honor.
#[derive(
  Debug,
  Clone,
  serde::Serialize,
  serde::Deserialize,
  schemars::JsonSchema,
  PartialEq,
  Default,
)]
#[serde(deny_unknown_fields)]
pub struct AwardItem {
  /// Award title.
  pub name: String,
  /// Issuing institution.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub issuer: Option<String>,
  /// Date or year awarded.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub date: Option<StringOrNumber>,
}

/// Speaking engagement or conference presentation.
#[derive(
  Debug,
  Clone,
  serde::Serialize,
  serde::Deserialize,
  schemars::JsonSchema,
  PartialEq,
  Default,
)]
#[serde(deny_unknown_fields)]
pub struct SpeakingItem {
  /// Talk or presentation title.
  pub title: String,
  /// Event, conference, or meetup name.
  pub event: String,
  /// Presentation date.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub date: Option<StringOrNumber>,
}

/// References representation.
#[derive(
  Debug,
  Clone,
  serde::Serialize,
  serde::Deserialize,
  schemars::JsonSchema,
  PartialEq,
)]
#[serde(untagged)]
pub enum ReferencesValue {
  /// List of professional references.
  List(Vec<ReferenceItem>),
  /// Availability note (e.g. "Available upon request").
  Text(String),
}

/// Professional reference contact.
#[derive(
  Debug,
  Clone,
  serde::Serialize,
  serde::Deserialize,
  schemars::JsonSchema,
  PartialEq,
  Default,
)]
#[serde(deny_unknown_fields)]
pub struct ReferenceItem {
  /// Reference name.
  pub name: String,
  /// Professional role or title.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub role: Option<String>,
  /// Organization or affiliation.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub org: Option<String>,
  /// Contact email or phone.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub contact: Option<String>,
}

/// Generates the canonical JSON Schema (Draft-07) for `ResumeDocument`.
#[must_use]
pub fn generate_schema() -> serde_json::Value {
  let settings = schemars::generate::SchemaSettings::draft07().with(|s| {
    s.definitions_path = "#/$defs/".to_string().into();
  });
  let generator = settings.into_generator();
  let schema = generator.into_root_schema_for::<ResumeDocument>();
  serde_json::to_value(schema).unwrap_or(serde_json::Value::Null)
}

/// Extracts metadata (name and version) from a content YAML string.
///
/// # Errors
/// Returns [`Error::YamlParse`] if the string cannot be parsed as a value.
pub fn load_meta(content_yaml: &str) -> Result<ContentMeta, Error> {
  let val: serde_yaml::Value =
    serde_yaml::from_str(content_yaml).map_err(|source| Error::YamlParse {
      path: path::PathBuf::from("content.yaml"),
      source,
    })?;

  let meta = val.get("meta");
  let name = meta
    .and_then(|m| m.get("name"))
    .and_then(serde_yaml::Value::as_str)
    .unwrap_or("Candidate")
    .to_string();
  let version = meta
    .and_then(|m| m.get("version"))
    .and_then(serde_yaml::Value::as_str)
    .unwrap_or("1.0.0")
    .to_string();

  Ok(ContentMeta { name, version })
}

/// Validates a YAML content string against JSON Schema.
///
/// # Errors
/// Returns [`Error::ValidationFailed`] if validation checks fail, or
/// [`Error::YamlParse`] / [`Error::JsonParse`] on malformed content.
pub fn validate(
  content_yaml: &str,
  custom_schema_json: Option<&str>,
) -> Result<(), Error> {
  let yaml_val: serde_yaml::Value = serde_yaml::from_str(content_yaml)
    .map_err(|source| Error::YamlParse {
      path: path::PathBuf::from("content.yaml"),
      source,
    })?;

  let json_content = serde_json::to_value(&yaml_val)?;

  let schema_val = if let Some(custom) = custom_schema_json {
    serde_json::from_str::<serde_json::Value>(custom)?
  } else {
    generate_schema()
  };

  let validator = match jsonschema::validator_for(&schema_val) {
    Ok(v) => v,
    Err(e) => {
      return Err(Error::ValidationFailed {
        details: format!("invalid JSON schema: {e}"),
      });
    }
  };

  let mut errors = Vec::new();
  for err in validator.iter_errors(&json_content) {
    let path = if err.instance_path().as_str().is_empty() {
      "<root>".to_string()
    } else {
      err.instance_path().to_string()
    };
    errors.push(format!("  - {path}: {err}"));
  }

  if errors.is_empty() {
    Ok(())
  } else {
    Err(Error::ValidationFailed {
      details: errors.join("\n"),
    })
  }
}

/// Validates a content file from disk, optionally using a custom schema file.
///
/// # Errors
/// Returns [`Error`] if reading files fails or validation fails.
pub fn validate_file(
  content_path: &path::Path,
  custom_schema_path: Option<&path::Path>,
) -> Result<(), Error> {
  let content_yaml = fs::read_to_string(content_path)?;
  let custom_schema = match custom_schema_path {
    Some(p) => Some(fs::read_to_string(p)?),
    None => None,
  };
  validate(&content_yaml, custom_schema.as_deref())
}
