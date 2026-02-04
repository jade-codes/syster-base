//! KPAR (Kernel Package Archive) format support.
//!
//! KPAR is a ZIP-based archive format for packaging SysML v2/KerML models
//! with metadata. It contains:
//!
//! - One or more XMI files containing the model
//! - A manifest file describing the package contents
//! - Optional additional resources (documentation, diagrams, etc.)
//!
//! ## KPAR Structure
//!
//! ```text
//! package.kpar (ZIP archive)
//! ├── META-INF/
//! │   └── manifest.xml       # Package manifest
//! ├── model/
//! │   ├── main.xmi           # Primary model file
//! │   └── library.xmi        # Referenced library
//! └── resources/
//!     └── readme.md          # Optional documentation
//! ```

use super::model::Model;
use super::{FormatCapability, InterchangeError, ModelFormat, Xmi};

/// Standard paths within a KPAR archive.
pub mod paths {
    /// Manifest file location.
    pub const MANIFEST: &str = "META-INF/manifest.xml";
    /// Model directory.
    pub const MODEL_DIR: &str = "model/";
}

/// KPAR format handler.
#[derive(Debug, Clone, Copy, Default)]
pub struct Kpar;

impl ModelFormat for Kpar {
    fn name(&self) -> &'static str {
        "KPAR"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["kpar"]
    }

    fn mime_type(&self) -> &'static str {
        "application/kpar"
    }

    fn capabilities(&self) -> FormatCapability {
        FormatCapability::FULL
    }

    fn read(&self, input: &[u8]) -> Result<Model, InterchangeError> {
        #[cfg(feature = "interchange")]
        {
            KparReader::new().read(input)
        }
        #[cfg(not(feature = "interchange"))]
        {
            let _ = input;
            Err(InterchangeError::Unsupported(
                "KPAR reading requires the 'interchange' feature".to_string(),
            ))
        }
    }

    fn write(&self, model: &Model) -> Result<Vec<u8>, InterchangeError> {
        #[cfg(feature = "interchange")]
        {
            KparWriter::new().write(model)
        }
        #[cfg(not(feature = "interchange"))]
        {
            let _ = model;
            Err(InterchangeError::Unsupported(
                "KPAR writing requires the 'interchange' feature".to_string(),
            ))
        }
    }

    fn validate(&self, input: &[u8]) -> Result<(), InterchangeError> {
        // Quick check for ZIP magic number
        if input.len() < 4 {
            return Err(InterchangeError::archive("File too small"));
        }

        // ZIP files start with PK\x03\x04
        if &input[0..4] != b"PK\x03\x04" {
            return Err(InterchangeError::archive("Not a valid ZIP archive"));
        }

        Ok(())
    }
}

// ============================================================================
// KPAR READER (requires interchange feature)
// ============================================================================

#[cfg(feature = "interchange")]
mod reader {
    use super::*;
    use std::io::{Cursor, Read};
    use zip::ZipArchive;

    /// KPAR archive reader.
    pub struct KparReader {
        /// The XMI format handler.
        xmi: Xmi,
    }

    impl KparReader {
        pub fn new() -> Self {
            Self { xmi: Xmi }
        }

        pub fn read(&self, input: &[u8]) -> Result<Model, InterchangeError> {
            let cursor = Cursor::new(input);
            let mut archive = ZipArchive::new(cursor)
                .map_err(|e| InterchangeError::archive(format!("Failed to open archive: {e}")))?;

            let mut combined_model = Model::new();

            // Find and parse all XMI files in the model/ directory
            let xmi_files: Vec<String> = (0..archive.len())
                .filter_map(|i| {
                    let file = archive.by_index(i).ok()?;
                    let name = file.name().to_string();
                    if name.starts_with(paths::MODEL_DIR) && name.ends_with(".xmi") {
                        Some(name)
                    } else {
                        None
                    }
                })
                .collect();

            // Parse each XMI file
            for xmi_path in xmi_files {
                let mut file = archive.by_name(&xmi_path).map_err(|e| {
                    InterchangeError::archive(format!("Failed to read {xmi_path}: {e}"))
                })?;

                let mut xmi_content = Vec::new();
                file.read_to_end(&mut xmi_content).map_err(|e| {
                    InterchangeError::archive(format!("Failed to read {xmi_path}: {e}"))
                })?;

                // Parse XMI and merge into combined model
                let model = self.xmi.read(&xmi_content)?;
                merge_models(&mut combined_model, model);
            }

            Ok(combined_model)
        }
    }

    /// Merge source model into target model.
    fn merge_models(target: &mut Model, source: Model) {
        // Add all elements from source
        for (id, element) in source.elements {
            if !target.elements.contains_key(&id) {
                target.elements.insert(id, element);
            }
        }

        // Add all relationships from source
        for rel in source.relationships {
            target.relationships.push(rel);
        }

        // Merge roots (avoid duplicates)
        for root in source.roots {
            if !target.roots.contains(&root) {
                target.roots.push(root);
            }
        }
    }
}

#[cfg(feature = "interchange")]
use reader::KparReader;

// ============================================================================
// KPAR WRITER (requires interchange feature)
// ============================================================================

#[cfg(feature = "interchange")]
mod writer {
    use super::*;
    use std::io::{Cursor, Write};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    /// KPAR archive writer.
    pub struct KparWriter {
        /// The XMI format handler.
        xmi: Xmi,
    }

    impl KparWriter {
        pub fn new() -> Self {
            Self { xmi: Xmi }
        }

        pub fn write(&self, model: &Model) -> Result<Vec<u8>, InterchangeError> {
            let mut buffer = Cursor::new(Vec::new());
            let mut zip = ZipWriter::new(&mut buffer);

            let options =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

            // Write manifest
            let manifest = generate_manifest(model);
            zip.start_file(paths::MANIFEST, options).map_err(|e| {
                InterchangeError::archive(format!("Failed to create manifest: {e}"))
            })?;
            zip.write_all(manifest.as_bytes())
                .map_err(|e| InterchangeError::archive(format!("Failed to write manifest: {e}")))?;

            // Write model as XMI
            let xmi_content = self.xmi.write(model)?;
            zip.start_file(format!("{}main.xmi", paths::MODEL_DIR), options)
                .map_err(|e| {
                    InterchangeError::archive(format!("Failed to create XMI file: {e}"))
                })?;
            zip.write_all(&xmi_content)
                .map_err(|e| InterchangeError::archive(format!("Failed to write XMI: {e}")))?;

            // Finish the archive
            zip.finish().map_err(|e| {
                InterchangeError::archive(format!("Failed to finalize archive: {e}"))
            })?;

            Ok(buffer.into_inner())
        }
    }

    /// Generate a simple manifest XML for the model.
    fn generate_manifest(model: &Model) -> String {
        let name = model.metadata.name.as_deref().unwrap_or("unnamed");
        let version = model.metadata.version.as_deref().unwrap_or("1.0.0");

        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<manifest xmlns="http://www.omg.org/spec/SysML/20230201/kpar">
  <package name="{name}" version="{version}">
    <model-files>
      <file>model/main.xmi</file>
    </model-files>
  </package>
</manifest>
"#
        )
    }
}

#[cfg(feature = "interchange")]
use writer::KparWriter;

// Stub implementations when feature is disabled
#[cfg(not(feature = "interchange"))]
struct KparReader;

#[cfg(not(feature = "interchange"))]
impl KparReader {
    fn new() -> Self {
        Self
    }

    fn read(&self, _input: &[u8]) -> Result<Model, InterchangeError> {
        Err(InterchangeError::Unsupported(
            "KPAR reading requires the 'interchange' feature".to_string(),
        ))
    }
}

#[cfg(not(feature = "interchange"))]
struct KparWriter;

#[cfg(not(feature = "interchange"))]
impl KparWriter {
    fn new() -> Self {
        Self
    }

    fn write(&self, _model: &Model) -> Result<Vec<u8>, InterchangeError> {
        Err(InterchangeError::Unsupported(
            "KPAR writing requires the 'interchange' feature".to_string(),
        ))
    }
}

/// Manifest information for a KPAR archive.
#[derive(Debug, Clone)]
pub struct KparManifest {
    /// Package name.
    pub name: String,
    /// Package version.
    pub version: Option<String>,
    /// Package description.
    pub description: Option<String>,
    /// List of model files.
    pub model_files: Vec<String>,
    /// Dependencies on other packages.
    pub dependencies: Vec<KparDependency>,
}

/// A dependency reference in a KPAR manifest.
#[derive(Debug, Clone)]
pub struct KparDependency {
    /// Dependency name.
    pub name: String,
    /// Required version (semver).
    pub version: Option<String>,
    /// URI to fetch the dependency.
    pub uri: Option<String>,
}

impl KparManifest {
    /// Create a new manifest with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: None,
            description: None,
            model_files: Vec::new(),
            dependencies: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kpar_format_metadata() {
        let kpar = Kpar;
        assert_eq!(kpar.name(), "KPAR");
        assert_eq!(kpar.extensions(), &["kpar"]);
        assert_eq!(kpar.mime_type(), "application/kpar");
        assert!(kpar.capabilities().read);
        assert!(kpar.capabilities().write);
    }

    #[test]
    fn test_kpar_validate_valid_zip() {
        let kpar = Kpar;
        // Minimal ZIP file header
        let input = b"PK\x03\x04rest of zip...";
        assert!(kpar.validate(input).is_ok());
    }

    #[test]
    fn test_kpar_validate_invalid() {
        let kpar = Kpar;
        let input = b"not a zip file";
        assert!(kpar.validate(input).is_err());
    }

    #[test]
    fn test_kpar_manifest_new() {
        let manifest = KparManifest::new("TestPackage");
        assert_eq!(manifest.name, "TestPackage");
        assert!(manifest.version.is_none());
        assert!(manifest.model_files.is_empty());
    }

    #[cfg(feature = "interchange")]
    mod interchange_tests {
        use super::*;
        use crate::interchange::model::{Element, ElementId, ElementKind};

        #[test]
        fn test_kpar_write_creates_valid_zip() {
            let mut model = Model::new();
            model.add_element(Element::new("pkg1", ElementKind::Package).with_name("TestPackage"));

            let kpar_bytes = Kpar.write(&model).expect("Failed to write KPAR");

            // Verify it's a valid ZIP
            assert!(Kpar.validate(&kpar_bytes).is_ok());
            assert!(kpar_bytes.starts_with(b"PK\x03\x04"));
        }

        #[test]
        fn test_kpar_roundtrip() {
            let mut model = Model::new();
            model.metadata.name = Some("RoundtripTest".to_string());
            model.metadata.version = Some("1.0.0".to_string());

            let pkg = Element::new("pkg1", ElementKind::Package).with_name("Vehicles");
            model.add_element(pkg);

            let part = Element::new("part1", ElementKind::PartDefinition)
                .with_name("Car")
                .with_owner("pkg1");
            model.add_element(part);

            // Update ownership
            if let Some(pkg) = model.elements.get_mut(&ElementId::new("pkg1")) {
                pkg.owned_elements.push(ElementId::new("part1"));
            }

            // Write to KPAR
            let kpar_bytes = Kpar.write(&model).expect("Write failed");

            // Read back
            let model2 = Kpar.read(&kpar_bytes).expect("Read failed");

            // Verify
            assert_eq!(model2.element_count(), 2);
            let pkg2 = model2.get(&ElementId::new("pkg1")).unwrap();
            assert_eq!(pkg2.name.as_deref(), Some("Vehicles"));
        }

        #[test]
        fn test_kpar_contains_manifest() {
            use std::io::Cursor;
            use zip::ZipArchive;

            let mut model = Model::new();
            model.metadata.name = Some("ManifestTest".to_string());
            model.add_element(Element::new("pkg1", ElementKind::Package).with_name("Test"));

            let kpar_bytes = Kpar.write(&model).expect("Write failed");

            // Open the archive and verify contents
            let cursor = Cursor::new(kpar_bytes);
            let mut archive = ZipArchive::new(cursor).expect("Failed to open archive");

            // Check manifest exists
            assert!(
                archive.by_name(paths::MANIFEST).is_ok(),
                "Manifest not found in KPAR"
            );

            // Check XMI file exists
            assert!(
                archive.by_name("model/main.xmi").is_ok(),
                "XMI file not found in KPAR"
            );
        }
    }
}
