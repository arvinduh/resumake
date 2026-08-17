//! Strongly-typed résumé data models and JSON Schema definitions.
//!
//! Provides the canonical representation for modular, block-based résumés,
//! deriving JSON Schema Draft 2020-12 via `schemars`.

#![allow(clippy::large_enum_variant)]

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Top-level Résumé document root.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ResumeDocument {
  /// Document metadata, contact details, ATS keywords, and theme tokens.
  pub meta: Meta,
  /// Ordered list of modular résumé sections.
  pub sections: Vec<Section>,
}

/// Metadata, author info, ATS targets, contact items, and styling theme.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct Meta {
  /// Full display name of the candidate.
  pub name: String,
  /// Semantic milestone version of the résumé (e.g. "1.0.0").
  pub version: String,
  /// Professional headline or role tagline (e.g. "Staff AI Systems Engineer").
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
  /// Contact items (email, phone, LinkedIn, GitHub, portfolio, etc.).
  pub contact: Vec<ContactItem>,
  /// Visual styling tokens (typography, margins, colors, paper size).
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub theme: Option<ThemeConfig>,
}

/// A contact item representation (string or structured object).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
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
  Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default,
)]
pub struct ThemeConfig {
  /// Font family for the document (e.g. "Crimson Pro", "Inter", "Linux Libertine").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub font_family: Option<String>,
  /// Body font size (e.g. "11.5pt", "10pt").
  #[serde(
    default,
    alias = "body_size",
    skip_serializing_if = "Option::is_none"
  )]
  pub font_size: Option<String>,
  /// Candidate name heading font size (e.g. "25pt").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub name_size: Option<String>,
  /// Section heading font size (e.g. "13pt").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub section_size: Option<String>,
  /// Organization / company font size (e.g. "12pt").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub org_size: Option<String>,
  /// Section horizontal divider rule thickness (e.g. "0.5pt").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub rule_thickness: Option<String>,
  /// Paper size ("us-letter" or "a4").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub paper_size: Option<String>,
  /// Page margin (e.g. "0.5in", "0.4in", "1.5cm").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub margin: Option<String>,
  /// Primary text ink color (hex string, e.g. "#111111").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub ink_color: Option<String>,
  /// Accent color for rules and highlights (hex string, e.g. "#2a2a2a").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub accent_color: Option<String>,
  /// Muted color for secondary metadata, dates, and locations (hex string, e.g. "#444444").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub muted_color: Option<String>,
}

/// Generic string or list of strings.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(untagged)]
pub enum StringOrList {
  /// Single string.
  Single(String),
  /// List of strings.
  List(Vec<String>),
}

/// Generic string or number (for years / counts).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(untagged)]
pub enum StringOrNumber {
  /// String value.
  Str(String),
  /// Numeric integer value.
  Num(i64),
}

/// A modular résumé section supporting polymorphic block types.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct Section {
  /// Section header title (e.g., "Education", "Experience", "Technical Skills", "Projects").
  pub title: String,
  /// Explicit block type descriptor (e.g. "experience", "education", "skills", "projects", "publications", "split_line", "columns", "lines", "bullets", "text").
  #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
  pub section_type: Option<String>,

  /// Generic items payload (supports polymorphic items for any section).
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
  pub lines: Option<BTreeMap<String, String>>,

  /// Generic bullet points shorthand.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub bullets: Option<Vec<String>>,

  /// Freeform markdown prose paragraph.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub text: Option<String>,
}

/// Polymorphic representation of education (single entry or multiple entries).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum EducationValue {
  /// Single university / institution.
  Single(EducationItem),
  /// List of multiple degrees or institutions.
  Multiple(Vec<EducationItem>),
}

/// Academic degree or institution item.
#[derive(
  Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default,
)]
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
  pub lines: Option<BTreeMap<String, String>>,
}

/// Academic thesis specification.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
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
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(untagged)]
pub enum SkillsValue {
  /// Key-value dictionary mapping category name to comma-separated skills or array of skills.
  Dictionary(BTreeMap<String, StringOrList>),
  /// List of skill categories with explicit category name and items.
  List(Vec<SkillCategoryItem>),
}

/// Explicit skill category item.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SkillCategoryItem {
  /// Category name (e.g. "Languages", "ML & Systems", "Cloud").
  #[serde(alias = "name")]
  pub category: String,
  /// Comma-separated string or array of skills.
  pub items: StringOrList,
}

/// Work experience organization holding one or more roles.
#[derive(
  Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default,
)]
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
  /// Organization website or URL.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub url: Option<String>,
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
#[derive(
  Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default,
)]
pub struct RoleItem {
  /// Job title or position (e.g. "Staff AI Systems Engineer").
  #[serde(alias = "role", alias = "position")]
  pub title: String,
  /// Employment date range (e.g. "Jul 2023 – Present").
  #[serde(alias = "date")]
  pub dates: String,
  /// Team, group, or division name (e.g. "Inference Systems").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub team: Option<String>,
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
#[derive(
  Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default,
)]
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
  /// Key metric or stats callout (e.g. "1.2k GitHub Stars").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub stats: Option<String>,
}

/// Academic publication or preprint.
#[derive(
  Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default,
)]
pub struct PublicationItem {
  /// Publication title.
  pub title: String,
  /// Authors list or string.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub authors: Option<StringOrList>,
  /// Conference, journal, or workshop venue (e.g. "NeurIPS 2024").
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
#[derive(
  Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default,
)]
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
  /// Expiration date.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub expiry: Option<String>,
  /// Credential ID.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub credential_id: Option<String>,
  /// Verification URL.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub url: Option<String>,
}

/// Award, honor, or competitive fellowship.
#[derive(
  Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default,
)]
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
#[derive(
  Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default,
)]
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
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
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
#[derive(
  Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default,
)]
pub struct ReferenceItem {
  /// Reference person's name.
  pub name: String,
  /// Job title or role.
  pub role: String,
  /// Company or university.
  pub org: String,
  /// Optional email or contact line.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub contact: Option<String>,
}

/// Generates the canonical JSON Schema value for the Résumé document model.
pub fn generate_builtin_schema() -> serde_json::Value {
  let schema = schemars::schema_for!(ResumeDocument);
  serde_json::to_value(schema).expect("Failed to serialize derived JSON schema")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_generate_builtin_schema_valid() {
    let schema_json = generate_builtin_schema();
    assert!(schema_json.is_object());
    let schema_str = serde_json::to_string_pretty(&schema_json).unwrap();
    assert!(schema_str.contains("ResumeDocument"));
    assert!(schema_str.contains("EducationItem"));
    assert!(schema_str.contains("ExperienceItem"));
  }

  #[test]
  fn test_deserialize_minimal_yaml() {
    let yaml = r#"
meta:
  name: "Jane Doe"
  version: "1.0.0"
  contact:
    - name: "jane@example.com"
      link: "mailto:jane@example.com"
sections:
  - title: "Education"
    education:
      institution: "University Name"
      degree: "B.S. in Computer Science"
      dates: "2020 - 2024"
"#;
    let doc: ResumeDocument = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(doc.meta.name, "Jane Doe");
    assert_eq!(doc.sections.len(), 1);
  }
}
