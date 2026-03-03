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
//! │  - elements: IndexMap<ElementId, Element>                │
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

pub mod decompile;
pub mod editing;
mod error;
mod format;
pub mod host;
pub mod integrate;
mod jsonld;
mod kpar;
pub mod metadata;
pub mod model;
pub mod recompile;
pub mod render;
pub mod views;
mod xmi;
mod yaml;

pub use decompile::{DecompileResult, decompile, decompile_with_source};
pub use editing::ChangeTracker;
pub use error::InterchangeError;
pub use format::{FormatCapability, ModelFormat};
pub use host::{ModelHost, ModelHostError};
pub use integrate::{
    ApplyEditsResult, apply_metadata_to_host, model_from_database, model_from_symbols,
    symbols_from_model,
};
pub use jsonld::JsonLd;
pub use kpar::{Kpar, KparManifest};
pub use metadata::{
    Dependency, ElementMeta, ImportMetadata, PackageMetadata, ProjectMetadata, SourceInfo,
};
pub use model::{Element, ElementId, ElementKind, Model, ModelMetadata};
pub use recompile::{restore_element_ids, restore_ids_from_symbols};
pub use render::{SourceMap, render_dirty};
pub use views::{
    ActionView, ConnectionView, DefinitionView, ElementView, PackageView, PortView,
    RequirementView, StateView, UsageView,
};
pub use xmi::Xmi;
pub use yaml::Yaml;

/// Supported file extensions for interchange formats.
pub fn supported_extensions() -> &'static [&'static str] {
    &["xmi", "kpar", "jsonld", "json", "yaml", "yml"]
}

/// Detect format from file extension.
pub fn detect_format(path: &std::path::Path) -> Option<Box<dyn ModelFormat>> {
    let ext = path.extension()?.to_str()?;
    match ext.to_lowercase().as_str() {
        "xmi" => Some(Box::new(Xmi)),
        "kpar" => Some(Box::new(Kpar)),
        "jsonld" | "json" => Some(Box::new(JsonLd)),
        "yaml" | "yml" => Some(Box::new(Yaml)),
        _ => None,
    }
}

/// Detect format from MIME type.
pub fn detect_format_from_mime(mime: &str) -> Option<Box<dyn ModelFormat>> {
    match mime {
        "application/xmi+xml" | "application/xml" => Some(Box::new(Xmi)),
        "application/zip" | "application/kpar" => Some(Box::new(Kpar)),
        "application/ld+json" | "application/json" => Some(Box::new(JsonLd)),
        "application/x-yaml" | "text/yaml" => Some(Box::new(Yaml)),
        _ => None,
    }
}
