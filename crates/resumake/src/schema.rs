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

/// Supported serialization formats for résumé content documents.
#[derive(Debug, Clone, Copy)]
pub enum ContentFormat {
  /// YAML document (`.yaml`, `.yml`).
  Yaml,
  /// JSON document (`.json`).
  Json,
}

impl ContentFormat {
  /// Infers the content format from a file path extension, defaulting to YAML.
  #[must_use]
  pub fn from_path(path: &path::Path) -> Self {
    match path.extension().and_then(|e| e.to_str()) {
      Some(ext) if ext.eq_ignore_ascii_case("json") => Self::Json,
      _ => Self::Yaml,
    }
  }

  /// Canonical starter filename for this content format.
  #[must_use]
  pub const fn default_filename(self) -> &'static str {
    match self {
      Self::Yaml => "content.yaml",
      Self::Json => "content.json",
    }
  }
}

/// Errors originating from schema validation and document parsing.
#[derive(Debug, thiserror::Error)]
pub enum Error {
  /// YAML parsing failure.
  #[error("failed to parse YAML content at '{}': {source}", path.display())]
  YamlParse {
    /// Target file path.
    path: path::PathBuf,
    /// Underlying YAML error.
    #[source]
    source: serde_yaml::Error,
  },
  /// JSON content parsing failure.
  #[error("failed to parse JSON content at '{}': {source}", path.display())]
  JsonContentParse {
    /// Target file path.
    path: path::PathBuf,
    /// Underlying JSON error.
    #[source]
    source: serde_json::Error,
  },
  /// JSON schema parsing or serialization failure.
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
#[derive(Debug)]
pub struct ContentMeta {
  /// Candidate name from `meta.name`.
  pub name: String,
}

/// Top-level Résumé document root.
#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResumeDocument {
  /// Optional JSON Schema URI for editor completion.
  #[serde(
    rename = "$schema",
    default,
    skip_serializing_if = "Option::is_none"
  )]
  pub schema: Option<String>,
  /// Candidate metadata and contact details.
  pub meta: Meta,
  /// Visual styling tokens (typography, margins, colors, paper size).
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub theme: Option<ThemeConfig>,
  /// Ordered list of modular résumé sections.
  pub sections: Vec<Section>,
}

/// Candidate identity and contact items.
#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Meta {
  /// Full display name of the candidate.
  pub name: String,
  /// Professional headline or role tagline.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub title: Option<String>,
  /// Availability badge, clearance, or work authorization status.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub badge: Option<String>,
  /// Contact items (email, phone, `LinkedIn`, GitHub, portfolio, etc.).
  pub contact: Vec<ContactItem>,
  /// Extensible custom metadata key-values.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub extra: Option<collections::BTreeMap<String, serde_json::Value>>,
}

/// A contact item representation (string or structured object).
#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
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
  },
}

/// Visual theme design tokens configurable in the root `theme` block.
#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
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
  /// Candidate name font weight (e.g. "semibold", "bold").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub name_weight: Option<String>,
  /// Organization / employer heading size (e.g. "12pt").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub org_size: Option<String>,
  /// Section header font size (e.g. "13pt").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub section_size: Option<String>,
  /// Section header font weight (e.g. "semibold", "bold").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub section_weight: Option<String>,
  /// Section header case transform ("uppercase", "small-caps", "none").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub section_case: Option<String>,
  /// Section header underline style ("underline", "none").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub section_style: Option<String>,
  /// Section divider line stroke thickness (e.g. "0.5pt", "0pt").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub rule_thickness: Option<String>,
  /// Adaptive vertical spacing base unit (e.g. "0.52em").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub leading: Option<String>,
  /// Adaptive space elasticity multiplier (e.g. 1.0, 0.0).
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub stretch: Option<f64>,
  /// Bullet left indentation (e.g. "0.55em", "0.35em").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub bullet_indent: Option<String>,
  /// Space between bullet marker and body text (e.g. "0.50em").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub bullet_body_indent: Option<String>,
  /// Bullet glyph preset ("bullet", "circle", "square", "dash", "triangle").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub bullet_marker: Option<String>,
  /// Secondary text color for dates, locations, and metadata.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub muted_color: Option<String>,
  /// Header alignment ("center" or "left").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub header_align: Option<String>,
  /// Spacing density preset ("compact", "comfortable", "spacious").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub density: Option<String>,
  /// Extensible custom theme tokens for ejected or third-party templates.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub extra: Option<collections::BTreeMap<String, serde_json::Value>>,
}

/// Generic string or list of strings.
#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum StringOrList {
  /// Single string.
  Single(String),
  /// List of strings.
  List(Vec<String>),
}

/// Generic string or number (for years / counts).
#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum StringOrNumber {
  /// String value.
  Str(String),
  /// Numeric integer value.
  Num(i64),
}

/// A modular résumé section supporting polymorphic block types.
#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
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

/// Polymorphic representation of education (single entry or multiple entries).
#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum EducationValue {
  /// Single university / institution.
  Single(Box<EducationItem>),
  /// List of multiple degrees or institutions.
  Multiple(Vec<EducationItem>),
}

/// Academic degree or institution item.
#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EducationItem {
  /// Name of university or institution.
  #[serde(
    alias = "school",
    alias = "university",
    default,
    skip_serializing_if = "Option::is_none"
  )]
  pub institution: Option<String>,
  /// City, State / Country.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub location: Option<String>,
  /// Degree and major (e.g. "B.S. in Computer Science").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub degree: Option<String>,
  /// GPA representation (e.g. "3.95 / 4.00").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub gpa: Option<String>,
  /// Attendance dates (e.g. "Sep 2020 – Jun 2022").
  #[serde(alias = "date", default, skip_serializing_if = "Option::is_none")]
  pub dates: Option<String>,
  /// Academic honors or distinctions (e.g. "Summa Cum Laude").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub honors: Option<StringOrList>,
  /// Thesis title and optional advisor.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub thesis: Option<ThesisValue>,
  /// Relevant coursework (inline tags or comma-separated).
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub coursework: Option<StringOrList>,
  /// Custom key-value lines attached to this degree.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub lines: Option<collections::BTreeMap<String, String>>,
}

/// Academic thesis specification.
#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum ThesisValue {
  /// Simple thesis title string.
  Title(String),
  /// Structured thesis title and advisor.
  Detailed {
    /// Thesis title.
    title: String,
    /// Advisor name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    advisor: Option<String>,
  },
}

/// Skills representation (dictionary or list of categorized skills).
#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum SkillsValue {
  /// Key-value dictionary mapping category name to comma-separated
  /// skills or array of skills.
  Dictionary(collections::BTreeMap<String, StringOrList>),
  /// List of skill categories with explicit category name and items.
  List(Vec<SkillCategoryItem>),
}

/// Explicit skill category item.
#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SkillCategoryItem {
  /// Category name (e.g. "Languages", "ML & Systems", "Cloud").
  #[serde(alias = "name")]
  pub category: String,
  /// Comma-separated string or array of skills.
  pub items: StringOrList,
}

/// Work experience organization holding one or more roles.
#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExperienceItem {
  /// Company or organization name.
  #[serde(
    alias = "company",
    alias = "organization",
    default,
    skip_serializing_if = "Option::is_none"
  )]
  pub org: Option<String>,
  /// City, State or "Remote".
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub location: Option<String>,
  /// One or more roles held at this organization (supports promotion ladders).
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub roles: Option<Vec<RoleItem>>,

  // Single-role shorthand fields:
  /// Job title (if specifying a single-role company without `roles` array).
  #[serde(
    alias = "role",
    alias = "position",
    default,
    skip_serializing_if = "Option::is_none"
  )]
  pub title: Option<String>,
  /// Employment dates (for single-role shorthand).
  #[serde(alias = "date", default, skip_serializing_if = "Option::is_none")]
  pub dates: Option<String>,
  /// Tech stack or tools used.
  #[serde(
    alias = "technologies",
    default,
    skip_serializing_if = "Option::is_none"
  )]
  pub stack: Option<String>,
  /// Accomplishment bullet points (for single-role shorthand).
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub bullets: Option<Vec<String>>,
}

/// A specific role within an organization.
#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RoleItem {
  /// Job title or position (e.g. "Staff AI Systems Engineer").
  #[serde(alias = "role", alias = "position")]
  pub title: String,
  /// Employment date range (e.g. "Jul 2023 – Present").
  #[serde(alias = "date")]
  pub dates: String,
  /// Tech stack or technologies used in this role.
  #[serde(
    alias = "technologies",
    default,
    skip_serializing_if = "Option::is_none"
  )]
  pub stack: Option<String>,
  /// High-impact accomplishment bullet points.
  #[serde(default)]
  pub bullets: Vec<String>,
}

/// Technical project or open-source initiative.
#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectItem {
  /// Project title.
  pub name: String,
  /// Tech stack or subtitle (e.g. "Rust · CUDA · Triton").
  #[serde(
    alias = "technologies",
    default,
    skip_serializing_if = "Option::is_none"
  )]
  pub stack: Option<String>,
  /// Date or period (e.g. "Jan 2024").
  #[serde(alias = "dates", default, skip_serializing_if = "Option::is_none")]
  pub date: Option<String>,
  /// Link to demo or repository.
  #[serde(alias = "link", default, skip_serializing_if = "Option::is_none")]
  pub url: Option<String>,
  /// Project accomplishment bullet points.
  #[serde(default)]
  pub bullets: Vec<String>,
}

/// Academic publication or preprint.
#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PublicationItem {
  /// Publication title.
  pub title: String,
  /// Authors list or string.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub authors: Option<StringOrList>,
  /// Conference, journal, or workshop venue (e.g. "`NeurIPS` 2024").
  #[serde(
    alias = "journal",
    alias = "conference",
    default,
    skip_serializing_if = "Option::is_none"
  )]
  pub venue: Option<String>,
  /// Publication year or date.
  #[serde(alias = "date", default, skip_serializing_if = "Option::is_none")]
  pub year: Option<StringOrNumber>,
  /// DOI or publication link.
  #[serde(alias = "doi", default, skip_serializing_if = "Option::is_none")]
  pub url: Option<String>,
  /// Acceptance rate or extra notes (e.g. "Oral Presentation, top 2%").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub notes: Option<String>,
}

/// Professional certification or licensure.
#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CertificationItem {
  /// Credential name.
  pub name: String,
  /// Issuing body or authority (e.g. "AWS", "Google Cloud").
  #[serde(
    alias = "organization",
    default,
    skip_serializing_if = "Option::is_none"
  )]
  pub issuer: Option<String>,
  /// Issue date.
  #[serde(alias = "dates", default, skip_serializing_if = "Option::is_none")]
  pub date: Option<String>,
  /// Verification URL.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub url: Option<String>,
}

/// Award, honor, or competitive fellowship.
#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AwardItem {
  /// Award title.
  pub name: String,
  /// Issuing organization or institution.
  #[serde(
    alias = "organization",
    default,
    skip_serializing_if = "Option::is_none"
  )]
  pub issuer: Option<String>,
  /// Date or year awarded.
  #[serde(alias = "year", default, skip_serializing_if = "Option::is_none")]
  pub date: Option<StringOrNumber>,
  /// Summary or award context.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub summary: Option<String>,
}

/// Speaking engagement or conference presentation.
#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SpeakingItem {
  /// Talk title.
  pub title: String,
  /// Conference or event name.
  #[serde(
    alias = "conference",
    default,
    skip_serializing_if = "Option::is_none"
  )]
  pub event: Option<String>,
  /// Location or virtual venue.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub location: Option<String>,
  /// Date of presentation.
  #[serde(alias = "year", default, skip_serializing_if = "Option::is_none")]
  pub date: Option<StringOrNumber>,
  /// Slides or recording URL.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub link: Option<String>,
}

/// References representation.
#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum ReferencesValue {
  /// Array of references.
  List(Vec<ReferenceItem>),
  /// Dictionary containing items list.
  Structured {
    /// References list.
    items: Vec<ReferenceItem>,
  },
}

/// Professional reference item.
#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReferenceItem {
  /// Reference person's name.
  pub name: String,
  /// Job title or role.
  pub role: String,
  /// Company or university.
  pub org: String,
}

/// Generates the canonical JSON Schema (Draft-07) for `ResumeDocument`.
#[must_use]
pub fn generate_schema() -> serde_json::Value {
  let settings = schemars::generate::SchemaSettings::draft07().with(|s| {
    s.definitions_path = "#/$defs/".into();
  });
  let generator = settings.into_generator();
  let schema = generator.into_root_schema_for::<ResumeDocument>();
  serde_json::to_value(schema).unwrap_or(serde_json::Value::Null)
}

/// Parses YAML or JSON content into a canonical [`serde_json::Value`].
fn parse_value(
  content: &str,
  format: ContentFormat,
  path: &path::Path,
) -> Result<serde_json::Value, Error> {
  match format {
    ContentFormat::Yaml => {
      let yaml_val: serde_yaml::Value =
        serde_yaml::from_str(content).map_err(|source| Error::YamlParse {
          path: path.to_path_buf(),
          source,
        })?;
      Ok(serde_json::to_value(&yaml_val)?)
    }
    ContentFormat::Json => {
      serde_json::from_str(content).map_err(|source| Error::JsonContentParse {
        path: path.to_path_buf(),
        source,
      })
    }
  }
}

/// Extracts candidate metadata from a YAML or JSON content string.
///
/// # Errors
/// Returns [`Error::YamlParse`] or [`Error::JsonContentParse`] if the content
/// cannot be parsed.
pub fn load_meta(
  content: &str,
  format: ContentFormat,
) -> Result<ContentMeta, Error> {
  let val =
    parse_value(content, format, path::Path::new(format.default_filename()))?;

  let name = val
    .get("meta")
    .and_then(|m| m.get("name"))
    .and_then(serde_json::Value::as_str)
    .unwrap_or("Candidate")
    .to_string();

  Ok(ContentMeta { name })
}

/// Extracts requested `theme.font_family` names from a résumé content file.
///
/// # Errors
/// Returns [`Error`] if the file cannot be read or parsed.
pub fn requested_fonts_from_file(
  content_path: &path::Path,
) -> Result<Vec<String>, Error> {
  let raw = fs::read_to_string(content_path)?;
  let val =
    parse_value(&raw, ContentFormat::from_path(content_path), content_path)?;
  Ok(extract_requested_fonts(&val))
}

/// Pulls font family names from `theme.font_family`.
fn extract_requested_fonts(val: &serde_json::Value) -> Vec<String> {
  let Some(font_val) = val.get("theme").and_then(|t| t.get("font_family"))
  else {
    return Vec::new();
  };
  match font_val {
    serde_json::Value::String(s) if !s.trim().is_empty() => {
      vec![s.trim().to_string()]
    }
    serde_json::Value::Array(items) => items
      .iter()
      .filter_map(serde_json::Value::as_str)
      .map(str::trim)
      .filter(|s| !s.is_empty())
      .map(str::to_string)
      .collect(),
    _ => Vec::new(),
  }
}

/// Validates parsed JSON content against either a custom schema or the built-in
/// schema.
fn validate_parsed(
  json_content: &serde_json::Value,
  custom_schema_json: Option<&str>,
) -> Result<(), Error> {
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
  for err in validator.iter_errors(json_content) {
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

/// Validates a YAML or JSON content string against JSON Schema.
///
/// # Errors
/// Returns [`Error::ValidationFailed`] if validation checks fail, or parse/JSON
/// errors on malformed content.
pub fn validate(
  content: &str,
  format: ContentFormat,
  custom_schema_json: Option<&str>,
) -> Result<(), Error> {
  let json_content =
    parse_value(content, format, path::Path::new(format.default_filename()))?;
  validate_parsed(&json_content, custom_schema_json)
}

/// Validates a content file from disk, optionally using a custom schema file.
///
/// # Errors
/// Returns [`Error`] if reading files fails or validation fails.
pub fn validate_file(
  content_path: &path::Path,
  custom_schema_path: Option<&path::Path>,
) -> Result<(), Error> {
  let raw = fs::read_to_string(content_path)?;
  let json_content =
    parse_value(&raw, ContentFormat::from_path(content_path), content_path)?;
  let custom_schema = match custom_schema_path {
    Some(p) => Some(fs::read_to_string(p)?),
    None => None,
  };
  validate_parsed(&json_content, custom_schema.as_deref())
}
