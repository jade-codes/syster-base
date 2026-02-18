//! Ergonomic entry-point for building and querying a [`Model`].
//!
//! `ModelHost` orchestrates the full pipeline — parsing SysML text,
//! loading XMI/JSON-LD/YAML, and providing typed navigational access
//! via the [`views`](super::views) module.
//!
//! ## Quick start
//!
//! ```ignore
//! use syster::ide::AnalysisHost;
//!
//! let mut host = AnalysisHost::new();
//! host.set_file_content("model.sysml", "package P { part def A; part b: A; }");
//! let model = host.model();
//! for root in model.root_views() {
//!     println!("{:?}", root.name());
//! }
//! ```
//!
//! For loading from interchange formats (XMI, KPAR, JSON-LD, YAML):
//!
//! ```ignore
//! use syster::interchange::host::ModelHost;
//!
//! let host = ModelHost::from_xmi(xmi_bytes)?;
//! for root in host.root_views() {
//!     println!("{:?}", root.name());
//! }
//! ```

use super::format::ModelFormat;
use super::model::{ElementId, ElementKind, Model};
use super::views::ElementView;
use super::{InterchangeError, JsonLd, Kpar, Xmi, Yaml};

/// Ergonomic host that owns a [`Model`] and provides typed queries.
///
/// This is the primary entry-point for programmatic model access.
/// It can be constructed from SysML text, XMI bytes, or any
/// supported interchange format.
pub struct ModelHost {
    model: Model,
    /// The original source text (if built from text).
    source: Option<String>,
}

impl ModelHost {
    // ── Construction ─────────────────────────────────────────────────

    /// Build from an XMI byte slice.
    pub fn from_xmi(bytes: &[u8]) -> Result<Self, ModelHostError> {
        Self::from_format(&Xmi, bytes)
    }

    /// Build from a JSON-LD byte slice.
    pub fn from_jsonld(bytes: &[u8]) -> Result<Self, ModelHostError> {
        Self::from_format(&JsonLd, bytes)
    }

    /// Build from a YAML byte slice.
    pub fn from_yaml(bytes: &[u8]) -> Result<Self, ModelHostError> {
        Self::from_format(&Yaml, bytes)
    }

    /// Build from a KPAR (ZIP archive) byte slice.
    pub fn from_kpar(bytes: &[u8]) -> Result<Self, ModelHostError> {
        Self::from_format(&Kpar, bytes)
    }

    /// Build from any supported interchange format.
    pub fn from_format(fmt: &dyn ModelFormat, bytes: &[u8]) -> Result<Self, ModelHostError> {
        let model = fmt.read(bytes).map_err(ModelHostError::Interchange)?;
        Ok(Self {
            model,
            source: None,
        })
    }

    /// Build from a file path — format is auto-detected from extension.
    pub fn from_file(path: &std::path::Path) -> Result<Self, ModelHostError> {
        let fmt = super::detect_format(path).ok_or_else(|| {
            ModelHostError::UnsupportedFormat(
                path.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("?")
                    .to_string(),
            )
        })?;
        let bytes = std::fs::read(path).map_err(ModelHostError::Io)?;
        Self::from_format(fmt.as_ref(), &bytes)
    }

    /// Wrap an already-constructed Model.
    pub fn from_model(model: Model) -> Self {
        Self {
            model,
            source: None,
        }
    }

    // ── Access ───────────────────────────────────────────────────────

    /// The underlying model (borrow).
    pub fn model(&self) -> &Model {
        &self.model
    }

    /// The underlying model (mutable).
    pub fn model_mut(&mut self) -> &mut Model {
        &mut self.model
    }

    /// Consume this host and return the model.
    pub fn into_model(self) -> Model {
        self.model
    }

    /// The original source text (if built via `from_text`).
    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    // ── Navigation (delegated to Model extension methods) ───────────

    /// Views over root elements.
    pub fn root_views(&self) -> Vec<ElementView<'_>> {
        self.model.root_views()
    }

    /// View a specific element by ID.
    pub fn view(&self, id: &ElementId) -> Option<ElementView<'_>> {
        self.model.view(id)
    }

    /// Find elements by declared name.
    pub fn find_by_name(&self, name: &str) -> Vec<ElementView<'_>> {
        self.model.find_by_name(name)
    }

    /// Find an element by fully qualified name.
    pub fn find_by_qualified_name(&self, qn: &str) -> Option<ElementView<'_>> {
        self.model.find_by_qualified_name(qn)
    }

    /// Find all elements of a specific metaclass kind.
    pub fn find_by_kind(&self, kind: ElementKind) -> Vec<ElementView<'_>> {
        self.model.find_by_kind(kind)
    }

    // ── Rendering ───────────────────────────────────────────────────

    /// Render the model back to SysML text via the decompiler.
    pub fn render(&self) -> String {
        super::decompile(&self.model).text
    }

    /// Export to XMI bytes.
    pub fn to_xmi(&self) -> Result<Vec<u8>, ModelHostError> {
        Xmi.write(&self.model).map_err(ModelHostError::Interchange)
    }

    /// Export to JSON-LD bytes.
    pub fn to_jsonld(&self) -> Result<Vec<u8>, ModelHostError> {
        JsonLd
            .write(&self.model)
            .map_err(ModelHostError::Interchange)
    }

    /// Export to YAML bytes.
    pub fn to_yaml(&self) -> Result<Vec<u8>, ModelHostError> {
        Yaml.write(&self.model)
            .map_err(ModelHostError::Interchange)
    }

    // ── Statistics ──────────────────────────────────────────────────

    /// Number of elements in the model.
    pub fn element_count(&self) -> usize {
        self.model.element_count()
    }

    /// Number of relationships in the model.
    pub fn relationship_count(&self) -> usize {
        self.model.relationship_count()
    }
}

impl std::fmt::Debug for ModelHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModelHost")
            .field("elements", &self.model.element_count())
            .field("relationships", &self.model.relationship_count())
            .field("roots", &self.model.roots.len())
            .field("has_source", &self.source.is_some())
            .finish()
    }
}

// ============================================================================
// ERROR TYPE
// ============================================================================

/// Errors from ModelHost operations.
#[derive(Debug)]
pub enum ModelHostError {
    /// The parsed text produced no symbols.
    EmptyModel,
    /// Interchange format error.
    Interchange(InterchangeError),
    /// IO error reading a file.
    Io(std::io::Error),
    /// Unsupported file extension.
    UnsupportedFormat(String),
}

impl std::fmt::Display for ModelHostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyModel => write!(f, "parsed text produced no model elements"),
            Self::Interchange(e) => write!(f, "interchange: {e}"),
            Self::Io(e) => write!(f, "io: {e}"),
            Self::UnsupportedFormat(ext) => write!(f, "unsupported format: .{ext}"),
        }
    }
}

impl std::error::Error for ModelHostError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Interchange(e) => Some(e),
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

// Test-only helper: parse SysML text into a ModelHost for unit tests.
// Production code should use AnalysisHost::model() instead.
#[cfg(test)]
impl ModelHost {
    pub(crate) fn from_text(source: &str) -> Result<Self, ModelHostError> {
        use crate::base::FileId;
        use crate::hir::{FileText, RootDatabase, file_symbols_from_text};
        use crate::interchange::model_from_symbols;

        let db = RootDatabase::new();
        let file_text = FileText::new(&db, FileId::new(0), source.to_string());
        let symbols = file_symbols_from_text(&db, file_text);

        if symbols.is_empty() {
            return Err(ModelHostError::EmptyModel);
        }

        let model = model_from_symbols(&symbols);
        Ok(Self {
            model,
            source: Some(source.to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_from_text_basic() {
        let host = ModelHost::from_text("package P { part def A; }")
            .expect("should parse");

        assert!(host.element_count() > 0);
        assert!(host.source().is_some());

        let roots = host.root_views();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].name(), Some("P"));
    }

    #[test]
    fn host_find_by_name() {
        let host = ModelHost::from_text(
            "package P { part def Vehicle; part def Wheel; part w: Wheel; }",
        )
        .expect("should parse");

        let vehicles = host.find_by_name("Vehicle");
        assert_eq!(vehicles.len(), 1);
        assert_eq!(vehicles[0].kind(), ElementKind::PartDefinition);

        let ws = host.find_by_name("w");
        assert_eq!(ws.len(), 1);
        assert_eq!(ws[0].kind(), ElementKind::PartUsage);
    }

    #[test]
    fn host_find_by_qualified_name() {
        let host =
            ModelHost::from_text("package Outer { part def Inner; }").expect("should parse");

        let found = host.find_by_qualified_name("Outer::Inner");
        assert!(found.is_some());
        assert_eq!(found.unwrap().kind(), ElementKind::PartDefinition);
    }

    #[test]
    fn host_find_by_kind() {
        let host =
            ModelHost::from_text("package P { part def A; part def B; part x: A; }")
                .expect("should parse");

        let defs = host.find_by_kind(ElementKind::PartDefinition);
        assert_eq!(defs.len(), 2);

        let usages = host.find_by_kind(ElementKind::PartUsage);
        assert_eq!(usages.len(), 1);
    }

    #[test]
    fn host_render_roundtrip() {
        let source = "package P {\n    part def A;\n}";
        let host = ModelHost::from_text(source).expect("should parse");

        let rendered = host.render();
        // Re-rendered text should contain the essential elements
        assert!(rendered.contains("package P"));
        assert!(rendered.contains("part def A"));
    }

    #[test]
    fn host_empty_text_errors() {
        let result = ModelHost::from_text("");
        assert!(result.is_err());
    }

    #[test]
    fn host_from_model_direct() {
        let mut model = Model::new();
        use super::super::model::Element;
        let pkg = Element::new("pkg1", ElementKind::Package).with_name("Direct");
        model.add_element(pkg);

        let host = ModelHost::from_model(model);
        assert_eq!(host.element_count(), 1);
        assert!(host.source().is_none());
        assert_eq!(host.find_by_name("Direct").len(), 1);
    }

    #[test]
    fn host_debug_fmt() {
        let host = ModelHost::from_text("package P;").expect("should parse");
        let dbg = format!("{:?}", host);
        assert!(dbg.contains("ModelHost"));
        assert!(dbg.contains("elements"));
    }
}
