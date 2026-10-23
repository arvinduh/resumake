pub mod templates;
pub(crate) mod world;

use std::fs;
use std::path::{Path, PathBuf};

use typst::foundations::{Label, Selector};
use typst::introspection::Introspector;
use typst::utils::PicoStr;
use typst_layout::PagedDocument;

use crate::engine::world::ResumakeWorld;
use crate::schema;
use crate::telemetry::{self, TelemetryReport};
use crate::utils::fs as utils_fs;

/// Errors originating from the Typst execution engine.
#[derive(Debug, thiserror::Error)]
pub enum Error {
  /// A user-specified font directory was not found.
  #[error("No valid font directory found. Searched locations:\n{}", .searched.iter().map(|p| format!("  - {}", p.display())).collect::<Vec<_>>().join("\n"))]
  FontDirNotFound {
    /// Searched locations.
    searched: Vec<PathBuf>,
  },
  /// The requested `--template <name>` is not registered.
  #[error("Unknown template '{name}'. Available templates: {}", .known.join(", "))]
  TemplateNotFound {
    /// The requested template name.
    name: String,
    /// Names of templates actually bundled into the binary.
    known: Vec<String>,
  },
  /// In-process Typst compilation failed with diagnostics.
  #[error("Typst compilation failed:\n{stderr}")]
  CompilationFailed {
    /// Captured diagnostic messages.
    stderr: String,
  },
  /// Metadata query failed.
  #[error("Typst query failed:\n{stderr}")]
  QueryFailed {
    /// Diagnostic error message.
    stderr: String,
  },
  /// Destination directory already exists and `--force` was not specified.
  #[error(
    "Destination directory '{}' already exists. Use --force to overwrite.",
    utils_fs::display_path(path)
  )]
  DestinationAlreadyExists {
    /// Destination directory path.
    path: PathBuf,
  },
  /// Content file was not found.
  #[error("Content file not found: '{}'", utils_fs::display_path(path))]
  ContentNotFound {
    /// Path to content file.
    path: PathBuf,
  },
  /// Document failed strict single-page layout geometry constraints.
  #[error("Dry-run check failed strict single-page layout constraints.")]
  LayoutConstraintViolation,
  /// Schema validation error.
  #[error(transparent)]
  Schema(#[from] schema::Error),
  /// Telemetry error.
  #[error(transparent)]
  Telemetry(#[from] telemetry::Error),
  /// Underlying I/O error.
  #[error("I/O error: {0}")]
  Io(#[from] std::io::Error),
}

/// The core in-process Typst compiler facade.
pub struct TypstEngine {
  font_path: Option<PathBuf>,
  root_path: PathBuf,
}

impl TypstEngine {
  /// Constructs a new [`TypstEngine`].
  ///
  /// # Errors
  /// Returns [`Error`] if font directory search fails.
  pub fn new(font_override: Option<&Path>) -> Result<Self, Error> {
    let root_path = crate::utils::fs::find_project_root();
    let font_path = world::discover_font_dir(&root_path, font_override)?;
    Ok(Self {
      font_path,
      root_path,
    })
  }

  /// Constructs a [`TypstEngine`] with a specific root path and font path.
  #[must_use]
  pub fn with_root(root_path: PathBuf, font_path: Option<PathBuf>) -> Self {
    Self {
      font_path,
      root_path,
    }
  }

  /// Returns the configured font directory, if any.
  #[must_use]
  pub fn font_path(&self) -> Option<&Path> {
    self.font_path.as_deref()
  }

  /// Resolves the Typst entry file from `--template` (named built-in, local
  /// file, or directory).
  ///
  /// # Errors
  /// Returns [`Error::TemplateNotFound`] if a named template is unknown.
  pub fn resolve_template(
    &self,
    template_name: &str,
  ) -> Result<PathBuf, Error> {
    let direct = Path::new(template_name);
    if direct.is_file() {
      return Ok(direct.to_path_buf());
    }
    if direct.is_dir() {
      let main_typ = direct.join("main.typ");
      if main_typ.is_file() {
        return Ok(main_typ);
      }
    }

    if Path::new(template_name)
      .extension()
      .is_some_and(|ext| ext.eq_ignore_ascii_case("typ"))
    {
      let joined = self.root_path.join(template_name);
      if joined.is_file() {
        return Ok(joined);
      }
    }

    let custom_dir = self.root_path.join("templates").join(template_name);
    if custom_dir.is_dir() {
      let custom_main = custom_dir.join("main.typ");
      if custom_main.is_file() {
        return Ok(custom_main);
      }
    }

    let custom_single = self
      .root_path
      .join("templates")
      .join(format!("{template_name}.typ"));
    if custom_single.is_file() {
      return Ok(custom_single);
    }

    if let Some(_template) = templates::find_embedded_template(template_name) {
      return Ok(PathBuf::from(format!("{template_name}/main.typ")));
    }

    Err(Error::TemplateNotFound {
      name: template_name.to_string(),
      known: templates::known_template_names(),
    })
  }

  /// Compiles a document to a [`PagedDocument`] in-memory.
  ///
  /// # Errors
  /// Returns [`Error`] if Typst compilation produces errors.
  pub fn compile_paged(
    &self,
    template_path: &Path,
    content_path: &Path,
  ) -> Result<PagedDocument, Error> {
    let world = ResumakeWorld::new(
      self.root_path.clone(),
      template_path.to_path_buf(),
      content_path.to_path_buf(),
      self.font_path.clone(),
    );

    let result = typst::compile(&world);
    let doc = result.output.map_err(|diags| {
      let stderr = world::format_diagnostics(&world, &diags);
      Error::CompilationFailed { stderr }
    })?;

    Ok(doc)
  }

  /// Renders a compiled [`PagedDocument`] to PDF bytes and writes to
  /// `output_path`.
  ///
  /// # Errors
  /// Returns [`Error`] if PDF rendering fails or writing to disk fails.
  pub fn render_pdf(
    &self,
    doc: &PagedDocument,
    output_path: &Path,
  ) -> Result<(), Error> {
    let pdf_bytes = typst_pdf::pdf(doc, &typst_pdf::PdfOptions::default())
      .map_err(|diags| {
        let stderr = diags
          .iter()
          .map(|d| {
            let severity = match d.severity {
              typst::diag::Severity::Error => "error",
              typst::diag::Severity::Warning => "warning",
            };
            let location = if let Some(id) = d.span.id() {
              format!("{}: ", id.vpath().get_with_slash())
            } else {
              String::new()
            };
            let mut msg = format!("{location}{severity}: {}", d.message);
            for hint in &d.hints {
              use std::fmt::Write as _;
              let _ = write!(msg, "\n  = hint: {}", hint.v);
            }
            msg
          })
          .collect::<Vec<_>>()
          .join("\n");
        Error::CompilationFailed { stderr }
      })?;

    if let Some(parent) = output_path.parent() {
      fs::create_dir_all(parent)?;
    }
    fs::write(output_path, pdf_bytes)?;

    Ok(())
  }

  /// Compiles a document to PDF bytes and writes to `output_path`.
  ///
  /// # Errors
  /// Returns [`Error`] if compilation fails or writing to disk fails.
  pub fn compile(
    &self,
    template_path: &Path,
    content_path: &Path,
    output_path: &Path,
  ) -> Result<(), Error> {
    let doc = self.compile_paged(template_path, content_path)?;
    self.render_pdf(&doc, output_path)
  }

  /// Compiles document in-memory and queries metadata values for `selector`.
  ///
  /// # Errors
  /// Returns [`Error`] if compilation fails or query fails.
  pub fn query_metadata(
    &self,
    template_path: &Path,
    content_path: &Path,
    selector: &str,
  ) -> Result<String, Error> {
    let doc = self.compile_paged(template_path, content_path)?;
    query_doc_metadata(&doc, selector)
  }
}

/// Queries metadata value(s) matching `selector` from a [`PagedDocument`] and
/// serializes to JSON.
///
/// # Errors
/// Returns [`Error::QueryFailed`] if serialization fails.
pub fn query_doc_metadata(
  doc: &PagedDocument,
  selector: &str,
) -> Result<String, Error> {
  let label_str = selector
    .trim()
    .trim_start_matches('<')
    .trim_end_matches('>');
  let Some(label) = Label::new(PicoStr::intern(label_str)) else {
    return Ok("[]".to_string());
  };
  let elems = doc.introspector().query(&Selector::Label(label));

  let mut values = Vec::new();
  for elem in elems {
    if let Some(metadata) =
      elem.to_packed::<typst::introspection::MetadataElem>()
    {
      values.push(metadata.value.clone());
    } else if let Ok(val) = elem.get_by_name("value") {
      values.push(val);
    }
  }

  serde_json::to_string(&values).map_err(|e| Error::QueryFailed {
    stderr: e.to_string(),
  })
}

/// Runs a fast, complete layout and content verification check on a document.
///
/// # Errors
/// Returns an [`Error`] if validation or compilation fails.
pub fn verify_content(
  content: &Path,
  template_name: &str,
  schema: Option<&Path>,
  font_path: Option<&Path>,
) -> Result<TelemetryReport, Error> {
  if !content.exists() {
    return Err(Error::ContentNotFound {
      path: content.to_path_buf(),
    });
  }

  schema::validate_schema_auto(content, schema)?;

  let engine = TypstEngine::new(font_path)?;
  let resolved_template = engine.resolve_template(template_name)?;
  let doc = engine.compile_paged(&resolved_template, content)?;
  let page_json = query_doc_metadata(&doc, "<pageinfo>")?;
  let bullets_json = query_doc_metadata(&doc, "<bulletinfo>")?;
  let report = telemetry::evaluate_telemetry(&page_json, &bullets_json)?;

  if !report.is_pass() {
    return Err(Error::LayoutConstraintViolation);
  }

  Ok(report)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_resolve_template_builtin_classic() {
    let engine = TypstEngine::with_root(PathBuf::from("."), None);

    let resolved = engine
      .resolve_template(templates::DEFAULT_TEMPLATE)
      .unwrap();
    assert_eq!(resolved, PathBuf::from("classic/main.typ"));
  }

  #[test]
  fn test_resolve_template_rejects_unknown_name() {
    let engine = TypstEngine::with_root(PathBuf::from("."), None);

    let err = engine.resolve_template("does-not-exist").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("does-not-exist"));
    assert!(msg.contains("classic"));
  }

  #[test]
  fn test_embedded_templates_discovery() {
    let templates = templates::embedded_templates();
    assert_ne!(templates.len(), 0);
    let classic = templates
      .iter()
      .find(|t| t.name == "classic")
      .expect("classic template must be found");
    assert_eq!(classic.name, "classic");
    assert_ne!(classic.entry, "");
    assert!(classic.files.iter().any(|f| f.rel_path == "tokens.typ"));
    assert!(classic.files.iter().any(|f| f.rel_path == "primitives.typ"));
    assert!(classic
      .files
      .iter()
      .any(|f| f.rel_path == "blocks/experience.typ"));
  }

  #[test]
  fn test_all_block_files_are_registered_in_main_typ() {
    let blocks = [
      "education",
      "experience",
      "projects",
      "skills",
      "publications",
      "split_line",
      "references",
      "lines",
    ];

    let classic = templates::find_embedded_template("classic")
      .expect("classic template must exist");
    for block in blocks {
      assert!(
        classic.entry.contains(&format!("blocks/{block}.typ")),
        "Block '{block}' is missing an #import in main.typ!"
      );
    }
  }
}
