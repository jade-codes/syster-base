//! Common trait for model interchange formats.

use super::model::Model;
use super::InterchangeError;

/// Capabilities supported by a format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatCapability {
    /// Can read/import models.
    pub read: bool,
    /// Can write/export models.
    pub write: bool,
    /// Supports streaming for large models.
    pub streaming: bool,
    /// Preserves all semantic information.
    pub lossless: bool,
}

impl FormatCapability {
    /// Full capability (read, write, lossless).
    pub const FULL: Self = Self {
        read: true,
        write: true,
        streaming: false,
        lossless: true,
    };

    /// Read-only capability.
    pub const READ_ONLY: Self = Self {
        read: true,
        write: false,
        streaming: false,
        lossless: true,
    };

    /// Write-only capability.
    pub const WRITE_ONLY: Self = Self {
        read: false,
        write: true,
        streaming: false,
        lossless: true,
    };
}

/// Trait for model interchange formats.
///
/// Implementations provide serialization and deserialization between
/// the standalone `Model` representation and external file formats.
///
/// ## Design
///
/// The format operates on `Model`, not `RootDatabase`. This provides:
/// - Clean separation between parsing and database integration
/// - Ability to work with models without a full Salsa database
/// - Easier testing and composition
///
/// To integrate with `RootDatabase`, use the `interchange::integrate` module.
pub trait ModelFormat: Send + Sync {
    /// Human-readable name of the format.
    fn name(&self) -> &'static str;

    /// File extension(s) for this format.
    fn extensions(&self) -> &'static [&'static str];

    /// MIME type for this format.
    fn mime_type(&self) -> &'static str;

    /// Capabilities of this format implementation.
    fn capabilities(&self) -> FormatCapability;

    /// Read a model from bytes.
    ///
    /// # Arguments
    /// * `input` - Raw bytes of the file content
    ///
    /// # Returns
    /// A standalone `Model` containing all elements and relationships.
    fn read(&self, input: &[u8]) -> Result<Model, InterchangeError>;

    /// Write a model to bytes.
    ///
    /// # Arguments
    /// * `model` - The model to export
    ///
    /// # Returns
    /// The serialized bytes in this format.
    fn write(&self, model: &Model) -> Result<Vec<u8>, InterchangeError>;

    /// Validate that the input is well-formed for this format.
    ///
    /// This is a quick check that doesn't fully parse the content.
    fn validate(&self, input: &[u8]) -> Result<(), InterchangeError> {
        // Default: try to detect format-specific markers
        let _ = input;
        Ok(())
    }
}
