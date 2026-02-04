//! Error types for interchange operations.

use thiserror::Error;

/// Errors that can occur during model interchange operations.
#[derive(Debug, Error)]
pub enum InterchangeError {
    /// XML parsing or serialization error.
    #[error("XML error: {0}")]
    Xml(String),

    /// ZIP archive error (for KPAR).
    #[error("Archive error: {0}")]
    Archive(String),

    /// JSON parsing or serialization error.
    #[error("JSON error: {0}")]
    Json(String),

    /// YAML parsing or serialization error.
    #[error("YAML error: {0}")]
    Yaml(String),

    /// IO error during read/write.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Invalid or unsupported XMI version.
    #[error("Unsupported XMI version: {0}")]
    UnsupportedVersion(String),

    /// Missing required element or attribute.
    #[error("Missing required {kind}: {name}")]
    Missing { kind: &'static str, name: String },

    /// Invalid element type or structure.
    #[error("Invalid {kind}: {message}")]
    Invalid { kind: &'static str, message: String },

    /// Reference to unknown element.
    #[error("Unresolved reference: {0}")]
    UnresolvedReference(String),

    /// Unsupported feature or format variant.
    #[error("Unsupported: {0}")]
    Unsupported(String),

    /// Model validation error.
    #[error("Validation error: {0}")]
    Validation(String),
}

impl InterchangeError {
    /// Create an XML error.
    pub fn xml(message: impl Into<String>) -> Self {
        Self::Xml(message.into())
    }

    /// Create an archive error.
    pub fn archive(message: impl Into<String>) -> Self {
        Self::Archive(message.into())
    }

    /// Create a JSON error.
    pub fn json(message: impl Into<String>) -> Self {
        Self::Json(message.into())
    }

    /// Create a YAML error.
    pub fn yaml(message: impl Into<String>) -> Self {
        Self::Yaml(message.into())
    }

    /// Create a missing element error.
    pub fn missing_element(name: impl Into<String>) -> Self {
        Self::Missing {
            kind: "element",
            name: name.into(),
        }
    }

    /// Create a missing attribute error.
    pub fn missing_attribute(name: impl Into<String>) -> Self {
        Self::Missing {
            kind: "attribute",
            name: name.into(),
        }
    }

    /// Create an invalid element error.
    pub fn invalid_element(message: impl Into<String>) -> Self {
        Self::Invalid {
            kind: "element",
            message: message.into(),
        }
    }

    /// Create an invalid attribute error.
    pub fn invalid_attribute(message: impl Into<String>) -> Self {
        Self::Invalid {
            kind: "attribute",
            message: message.into(),
        }
    }
}
