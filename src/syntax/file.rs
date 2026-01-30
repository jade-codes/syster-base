//! Syntax file wrapper for parsed SysML/KerML files.
//!
//! This module provides a unified interface for working with parsed files
//! from the rowan-based parser.

use crate::base::LineIndex;
use crate::parser::{parse_sysml, parse_kerml, AstNode, NamespaceMember, Parse, SourceFile};

/// A parsed syntax file that wraps a rowan Parse result.
///
/// This provides a language-agnostic interface for working with parsed
/// SysML and KerML files.
#[derive(Debug, Clone)]
pub struct SyntaxFile {
    /// The underlying rowan parse result
    parse: Parse,
    /// The file extension (sysml or kerml)
    extension: FileExtension,
}

// Manual PartialEq implementation - two SyntaxFiles are equal if they have the same extension
// and produce the same syntax tree (approximated by checking errors)
impl PartialEq for SyntaxFile {
    fn eq(&self, other: &Self) -> bool {
        self.extension == other.extension && self.parse.errors == other.parse.errors
    }
}

impl Eq for SyntaxFile {}

/// File extension type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileExtension {
    SysML,
    KerML,
}

impl SyntaxFile {
    /// Create a new SyntaxFile from source code and extension
    pub fn new(source: &str, extension: FileExtension) -> Self {
        let parse = match extension {
            FileExtension::SysML => parse_sysml(source),
            FileExtension::KerML => parse_kerml(source),
        };
        Self {
            parse,
            extension,
        }
    }

    /// Create a SysML syntax file
    pub fn sysml(source: &str) -> Self {
        Self {
            parse: parse_sysml(source),
            extension: FileExtension::SysML,
        }
    }

    /// Create a KerML syntax file
    pub fn kerml(source: &str) -> Self {
        Self {
            parse: parse_kerml(source),
            extension: FileExtension::KerML,
        }
    }

    /// Get the underlying parse result
    pub fn parse(&self) -> &Parse {
        &self.parse
    }

    /// Get the root source file AST node
    pub fn source_file(&self) -> Option<SourceFile> {
        SourceFile::cast(self.parse.syntax())
    }

    /// Check if parsing had errors
    pub fn has_errors(&self) -> bool {
        !self.parse.errors.is_empty()
    }

    /// Get parse errors
    pub fn errors(&self) -> &[crate::parser::SyntaxError] {
        &self.parse.errors
    }

    /// Check if this is a SysML file
    pub fn is_sysml(&self) -> bool {
        self.extension == FileExtension::SysML
    }

    /// Check if this is a KerML file
    pub fn is_kerml(&self) -> bool {
        self.extension == FileExtension::KerML
    }

    /// Extract import paths from the file
    pub fn extract_imports(&self) -> Vec<String> {
        let Some(source_file) = self.source_file() else {
            return Vec::new();
        };

        source_file
            .members()
            .filter_map(|member| {
                if let NamespaceMember::Import(import) = member {
                    import.target().map(|t| {
                        let mut path = t.to_string();
                        if import.is_wildcard() {
                            path.push_str("::*");
                        }
                        if import.is_recursive() {
                            if path.ends_with("::*") {
                                path.push('*');
                            } else {
                                path.push_str("::**");
                            }
                        }
                        path
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get the source text of the file
    pub fn source_text(&self) -> String {
        self.parse.syntax().text().to_string()
    }

    /// Create a LineIndex for converting byte offsets to line/column positions
    pub fn line_index(&self) -> LineIndex {
        LineIndex::new(&self.source_text())
    }
}

// Legacy compatibility: provide SysML/KerML specific types as aliases
// These will be removed once migration is complete

/// Legacy type alias - use SyntaxFile instead
pub type SysMLFile = SyntaxFile;

/// Legacy type alias - use SyntaxFile instead  
pub type KerMLFile = SyntaxFile;
