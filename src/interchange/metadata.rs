//! KPAR-standard metadata for SysML projects.
//!
//! This module provides types aligned with the KPAR (Kernel Package Archive) format:
//! - `project.json` - Project metadata with name, version, description, dependencies
//! - `meta.json` - File index with element ID mapping for lossless round-trip
//!
//! ## Data Flow
//!
//! ```text
//! Import: XMI/KPAR → decompile → SysML files + project.json + meta.json
//! Export: SysML → parse → HIR + meta.json → recompile → XMI (with original IDs)
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// project.json - Project-level metadata
// ============================================================================

/// Project metadata (stored as `project.json`).
///
/// Defines the project name, version, description, and dependencies.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ProjectMetadata {
    /// Human-readable project name.
    pub name: String,
    
    /// Semantic version (e.g., "2.0.0").
    pub version: String,
    
    /// Project description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Dependencies on other packages/libraries.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub usage: Vec<Dependency>,
}

impl ProjectMetadata {
    /// Create a new project with name and version.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            description: None,
            usage: Vec::new(),
        }
    }
    
    /// Set the description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
    
    /// Add a dependency.
    pub fn with_dependency(mut self, dep: Dependency) -> Self {
        self.usage.push(dep);
        self
    }
}

/// A dependency on another package/library.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Dependency {
    /// URI or path to the dependency.
    pub resource: String,
    
    /// Version constraint (e.g., "1.0.0", ">=2.0.0").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "versionConstraint")]
    pub version_constraint: Option<String>,
}

impl Dependency {
    /// Create a dependency on a resource.
    pub fn new(resource: impl Into<String>) -> Self {
        Self {
            resource: resource.into(),
            version_constraint: None,
        }
    }
    
    /// Set version constraint.
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version_constraint = Some(version.into());
        self
    }
}

// ============================================================================
// meta.json - KPAR file index (standard format)
// ============================================================================

/// Package metadata (stored as `meta.json` in KPAR archives).
///
/// This is the standard KPAR format - just file index + basic metadata.
/// Does NOT contain element IDs - those are XMI-specific.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PackageMetadata {
    /// Map of namespace name → relative file path.
    #[serde(default)]
    pub index: HashMap<String, String>,
    
    /// Creation/import timestamp (ISO 8601).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    
    /// Metamodel URI (e.g., "https://www.omg.org/spec/SysML/20250201").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metamodel: Option<String>,
}

impl PackageMetadata {
    /// Create new empty metadata.
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set the creation timestamp.
    pub fn with_created(mut self, timestamp: impl Into<String>) -> Self {
        self.created = Some(timestamp.into());
        self
    }
    
    /// Set the metamodel URI.
    pub fn with_metamodel(mut self, uri: impl Into<String>) -> Self {
        self.metamodel = Some(uri.into());
        self
    }
    
    /// Add a file to the index.
    pub fn add_file(&mut self, namespace: impl Into<String>, file: impl Into<String>) {
        self.index.insert(namespace.into(), file.into());
    }
}

// ============================================================================
// ImportMetadata - XMI round-trip metadata (separate file)
// ============================================================================

/// Metadata for XMI round-trip preservation.
///
/// Stored as a companion file (e.g., `.xmi-metadata.json`) alongside
/// decompiled SysML text to preserve original XMI element IDs.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ImportMetadata {
    /// Schema version for forward compatibility.
    pub version: u32,
    
    /// Information about the source file.
    pub source: SourceInfo,
    
    /// Per-element metadata, keyed by qualified name.
    pub elements: HashMap<String, ElementMeta>,
}

impl ImportMetadata {
    /// Current schema version.
    pub const CURRENT_VERSION: u32 = 1;
    
    /// Create new metadata with current version.
    pub fn new() -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            source: SourceInfo::default(),
            elements: HashMap::new(),
        }
    }
    
    /// Create metadata with source info.
    pub fn with_source(mut self, source: SourceInfo) -> Self {
        self.source = source;
        self
    }
    
    /// Add element metadata.
    pub fn add_element(&mut self, qualified_name: impl Into<String>, meta: ElementMeta) {
        self.elements.insert(qualified_name.into(), meta);
    }
    
    /// Get element metadata by qualified name.
    pub fn get_element(&self, qualified_name: &str) -> Option<&ElementMeta> {
        self.elements.get(qualified_name)
    }
}

/// Information about the original source file.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SourceInfo {
    /// Original file path or URI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    
    /// Format of the source (xmi, jsonld, kpar).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    
    /// Timestamp when the file was imported (ISO 8601).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imported_at: Option<String>,
    
    /// Tool that exported the original file (if known).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    
    /// Tool version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_version: Option<String>,
}

impl SourceInfo {
    /// Create source info from a file path.
    pub fn from_path(path: impl Into<String>) -> Self {
        Self {
            path: Some(path.into()),
            ..Default::default()
        }
    }
    
    /// Set the format.
    pub fn with_format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }
    
    /// Set the import timestamp.
    pub fn with_timestamp(mut self, timestamp: impl Into<String>) -> Self {
        self.imported_at = Some(timestamp.into());
        self
    }
}

/// Metadata for a single element.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ElementMeta {
    /// Original XMI element ID (xmi:id or @id).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "originalId")]
    pub original_id: Option<String>,
    
    /// Declared element ID if different (elementId attribute).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "declaredId")]
    pub declared_id: Option<String>,
    
    /// Attributes that couldn't be mapped to SysML text syntax.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[serde(rename = "unmappedAttributes")]
    pub unmapped_attributes: HashMap<String, serde_json::Value>,
    
    /// Original order among siblings (for deterministic re-export).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "siblingOrder")]
    pub sibling_order: Option<u32>,
}

impl ElementMeta {
    /// Create new element metadata with original ID.
    pub fn with_id(id: impl Into<String>) -> Self {
        Self {
            original_id: Some(id.into()),
            ..Default::default()
        }
    }
    
    /// Set declared element ID.
    pub fn with_declared_id(mut self, id: impl Into<String>) -> Self {
        self.declared_id = Some(id.into());
        self
    }
    
    /// Add an unmapped attribute.
    pub fn with_unmapped(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.unmapped_attributes.insert(key.into(), value);
        self
    }
    
    /// Set sibling order.
    pub fn with_order(mut self, order: u32) -> Self {
        self.sibling_order = Some(order);
        self
    }
    
    /// Get the element ID (original or declared).
    pub fn element_id(&self) -> Option<&str> {
        self.original_id.as_deref().or(self.declared_id.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_project_metadata() {
        let project = ProjectMetadata::new("Vehicle Model", "1.0.0")
            .with_description("A sample vehicle model")
            .with_dependency(
                Dependency::new("https://www.omg.org/spec/SysML/20250201/Systems-Library.kpar")
                    .with_version("2.0.0")
            );
        
        let json = serde_json::to_string_pretty(&project).unwrap();
        assert!(json.contains("\"name\": \"Vehicle Model\""));
        assert!(json.contains("\"versionConstraint\": \"2.0.0\""));
        
        let parsed: ProjectMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "Vehicle Model");
        assert_eq!(parsed.usage.len(), 1);
    }
    
    #[test]
    fn test_package_metadata() {
        let mut meta = PackageMetadata::new()
            .with_created("2025-03-13T00:00:00Z")
            .with_metamodel("https://www.omg.org/spec/SysML/20250201");
        
        meta.add_file("CausationConnections", "CausationConnections.sysml");
        meta.add_file("CauseAndEffect", "CauseAndEffect.sysml");
        
        let json = serde_json::to_string_pretty(&meta).unwrap();
        assert!(json.contains("\"CausationConnections\": \"CausationConnections.sysml\""));
        assert!(!json.contains("elements"), "PackageMetadata should not have elements field");
        
        let parsed: PackageMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.index.len(), 2);
        assert_eq!(parsed.created.as_deref(), Some("2025-03-13T00:00:00Z"));
        assert_eq!(parsed.metamodel.as_deref(), Some("https://www.omg.org/spec/SysML/20250201"));
    }
    
    #[test]
    fn test_import_metadata_with_element_ids() {
        let mut meta = ImportMetadata::new()
            .with_source(
                SourceInfo::from_path("model.xmi")
                    .with_format("xmi")
            );
        
        meta.add_element("Package1", ElementMeta::with_id("pkg-1"));
        meta.add_element("Package1::Vehicle", ElementMeta::with_id("vehicle-1"));
        
        assert_eq!(meta.version, ImportMetadata::CURRENT_VERSION);
        assert_eq!(meta.get_element("Package1").unwrap().original_id.as_deref(), Some("pkg-1"));
        assert_eq!(meta.get_element("Package1::Vehicle").unwrap().original_id.as_deref(), Some("vehicle-1"));
    }
    
    #[test]
    fn test_element_meta_with_unmapped() {
        let meta = ElementMeta::with_id("xyz-456")
            .with_declared_id("MyElement")
            .with_unmapped("customAttr", serde_json::json!(42))
            .with_order(3);
        
        assert_eq!(meta.original_id.as_deref(), Some("xyz-456"));
        assert_eq!(meta.declared_id.as_deref(), Some("MyElement"));
        assert_eq!(meta.unmapped_attributes.get("customAttr"), Some(&serde_json::json!(42)));
        assert_eq!(meta.sibling_order, Some(3));
        assert_eq!(meta.element_id(), Some("xyz-456"));
    }
    
    #[test]
    fn test_serialize_roundtrip() {
        let mut meta = ImportMetadata::new()
            .with_source(
                SourceInfo::from_path("model.xmi")
                    .with_format("xmi")
            );
        
        meta.add_element(
            "Package1",
            ElementMeta::with_id("pkg-1").with_order(0),
        );
        meta.add_element(
            "Package1::Part1",
            ElementMeta::with_id("part-1")
                .with_unmapped("isIndividual", serde_json::json!(true)),
        );
        
        let json = serde_json::to_string_pretty(&meta).unwrap();
        let parsed: ImportMetadata = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed.version, ImportMetadata::CURRENT_VERSION);
        assert_eq!(parsed.source.path.as_deref(), Some("model.xmi"));
        assert_eq!(parsed.elements.len(), 2);
        
        let pkg = parsed.get_element("Package1").unwrap();
        assert_eq!(pkg.original_id.as_deref(), Some("pkg-1"));
        
        let part = parsed.get_element("Package1::Part1").unwrap();
        assert_eq!(part.unmapped_attributes.get("isIndividual"), Some(&serde_json::json!(true)));
    }
    
    #[test]
    fn test_empty_metadata_serializes_minimal() {
        let meta = PackageMetadata::new();
        let json = serde_json::to_string(&meta).unwrap();
        
        // Should not contain empty unmapped_attributes or null optionals
        assert!(!json.contains("unmappedAttributes"));
        assert!(!json.contains("elements"));
    }
}

// ============================================================================
// File I/O for ImportMetadata
// ============================================================================

impl ImportMetadata {
    /// Read ImportMetadata from a JSON file.
    pub fn read_from_file(path: impl AsRef<std::path::Path>) -> Result<Self, std::io::Error> {
        let content = std::fs::read_to_string(path.as_ref())?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
    
    /// Write ImportMetadata to a JSON file.
    pub fn write_to_file(&self, path: impl AsRef<std::path::Path>) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path.as_ref(), json)
    }
}
