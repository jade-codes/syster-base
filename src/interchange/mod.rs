//! Model interchange formats for SysML v2.
//!
//! This module provides serialization and deserialization for various
//! model interchange formats:
//!
//! - **XMI** - XML Model Interchange (OMG standard)
//! - **KPAR** - Kernel Package Archive (ZIP with XMI + metadata)
//! - **JSON-LD** - JSON Linked Data format
//!
//! ## Architecture
//!
//! All formats work with a standalone [`Model`] type, decoupled from the
//! Salsa database. This enables:
//! - Loading models without text parsing
//! - Exporting models to various formats
//! - Easy testing and composition
//!
//! ```text
//! ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
//! │   XMI File   │     │  KPAR File   │     │  JSON-LD     │
//! └──────┬───────┘     └──────┬───────┘     └──────┬───────┘
//!        │                    │                    │
//!        ▼                    ▼                    ▼
//! ┌──────────────────────────────────────────────────────────┐
//! │                    ModelFormat trait                      │
//! │  - read(&[u8]) -> Result<Model>                          │
//! │  - write(&Model) -> Result<Vec<u8>>                      │
//! └──────────────────────────────────────────────────────────┘
//!        │
//!        ▼
//! ┌──────────────────────────────────────────────────────────┐
//! │                    Model (standalone)                     │
//! │  - elements: HashMap<ElementId, Element>                 │
//! │  - relationships: Vec<Relationship>                      │
//! │  - roots: Vec<ElementId>                                 │
//! └──────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```ignore
//! use syster::interchange::{Xmi, ModelFormat, Model};
//!
//! // Read a model from XMI
//! let xmi_bytes = std::fs::read("model.xmi")?;
//! let model = Xmi.read(&xmi_bytes)?;
//!
//! // Export to JSON-LD
//! let jsonld_bytes = JsonLd.write(&model)?;
//! ```

mod error;
mod format;
pub mod integrate;
mod jsonld;
mod kpar;
pub mod model;
mod xmi;

pub use error::InterchangeError;
pub use format::{FormatCapability, ModelFormat};
pub use integrate::{model_from_database, model_from_symbols};
pub use jsonld::JsonLd;
pub use kpar::{Kpar, KparManifest};
pub use model::{Element, ElementId, ElementKind, Model, ModelMetadata, Relationship, RelationshipKind};
pub use xmi::Xmi;

/// Supported file extensions for interchange formats.
pub fn supported_extensions() -> &'static [&'static str] {
    &["xmi", "kpar", "jsonld", "json"]
}

/// Detect format from file extension.
pub fn detect_format(path: &std::path::Path) -> Option<Box<dyn ModelFormat>> {
    let ext = path.extension()?.to_str()?;
    match ext.to_lowercase().as_str() {
        "xmi" => Some(Box::new(Xmi)),
        "kpar" => Some(Box::new(Kpar)),
        "jsonld" | "json" => Some(Box::new(JsonLd)),
        _ => None,
    }
}

/// Detect format from MIME type.
pub fn detect_format_from_mime(mime: &str) -> Option<Box<dyn ModelFormat>> {
    match mime {
        "application/xmi+xml" | "application/xml" => Some(Box::new(Xmi)),
        "application/zip" | "application/kpar" => Some(Box::new(Kpar)),
        "application/ld+json" | "application/json" => Some(Box::new(JsonLd)),
        _ => None,
    }
}
