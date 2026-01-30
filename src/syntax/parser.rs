//! Syntax-level parsing interface.
//!
//! This module provides a unified interface for parsing SysML and KerML files
//! using the rowan-based parser.

use crate::base::constants::{KERML_EXT, SYSML_EXT};
use crate::syntax::file::{FileExtension, SyntaxFile};
use std::path::{Path, PathBuf};

/// Parse error type for syntax-level errors
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
    /// Position info (for compatibility)
    pub position: ParseErrorPosition,
}

/// Position information in a parse error
#[derive(Debug, Clone, Copy)]
pub struct ParseErrorPosition {
    pub line: usize,
    pub column: usize,
}

impl ParseError {
    pub fn syntax_error(message: &str, line: usize, column: usize) -> Self {
        Self {
            message: message.to_string(),
            line,
            column,
            position: ParseErrorPosition { line, column },
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.column, self.message)
    }
}

impl std::error::Error for ParseError {}

/// Parse result containing content and any errors
#[derive(Debug)]
pub struct ParseResult<T> {
    /// The parsed content (using `content` internally, aliased as `syntax_file` for convenience)
    pub content: Option<T>,
    pub errors: Vec<ParseError>,
}

impl<T> ParseResult<T> {
    pub fn with_errors(errors: Vec<ParseError>) -> Self {
        Self {
            content: None,
            errors,
        }
    }

    pub fn ok(content: T) -> Self {
        Self {
            content: Some(content),
            errors: Vec::new(),
        }
    }

    pub fn with_content_and_errors(content: T, errors: Vec<ParseError>) -> Self {
        Self {
            content: Some(content),
            errors,
        }
    }

    /// Check if parsing succeeded without errors
    pub fn is_ok(&self) -> bool {
        self.content.is_some() && self.errors.is_empty()
    }

    /// Check if there are any parse errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get the syntax file (alias for content)
    pub fn syntax_file(&self) -> Option<&T> {
        self.content.as_ref()
    }
}

/// Get file extension from path
pub fn get_extension(path: &Path) -> Result<&str, ParseError> {
    path.extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| ParseError::syntax_error("No file extension", 0, 0))
}

/// Validate that the extension is a supported language file
pub fn validate_extension(path: &Path) -> Result<&str, String> {
    let ext = get_extension(path).map_err(|e| e.message)?;
    if ext == SYSML_EXT || ext == KERML_EXT {
        Ok(ext)
    } else {
        Err(format!("Unsupported file extension: {}", ext))
    }
}

/// Load file contents
pub fn load_file(path: &PathBuf) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))
}

/// Loads and parses a language file (SysML or KerML) based on file extension.
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be read
/// - The file has an invalid extension
pub fn load_and_parse(path: &PathBuf) -> Result<SyntaxFile, String> {
    let ext = validate_extension(path)?;
    let content = load_file(path)?;

    let extension = match ext {
        SYSML_EXT => FileExtension::SysML,
        KERML_EXT => FileExtension::KerML,
        _ => return Err(format!("Unsupported extension: {}", ext)),
    };

    Ok(SyntaxFile::new(&content, extension))
}

/// Parses language content from a string based on file extension.
pub fn parse_content(content: &str, path: &Path) -> Result<SyntaxFile, String> {
    let ext = validate_extension(path)?;

    let extension = match ext {
        SYSML_EXT => FileExtension::SysML,
        KERML_EXT => FileExtension::KerML,
        _ => return Err(format!("Unsupported extension: {}", ext)),
    };

    Ok(SyntaxFile::new(content, extension))
}

/// Parses content and returns a ParseResult with detailed error information.
/// This is the primary function for LSP usage - errors don't fail, they're captured.
pub fn parse_with_result(content: &str, path: &Path) -> ParseResult<SyntaxFile> {
    let ext = match get_extension(path) {
        Ok(e) => e,
        Err(e) => return ParseResult::with_errors(vec![e]),
    };

    let extension = match ext {
        SYSML_EXT => FileExtension::SysML,
        KERML_EXT => FileExtension::KerML,
        _ => {
            return ParseResult::with_errors(vec![ParseError::syntax_error(
                "Unsupported file extension",
                0,
                0,
            )]);
        }
    };

    let syntax_file = SyntaxFile::new(content, extension);
    
    // Convert rowan syntax errors to our ParseError type with line/column info
    let line_index = crate::base::LineIndex::new(content);
    let errors: Vec<ParseError> = syntax_file
        .errors()
        .iter()
        .map(|e| {
            // Convert TextRange start to line/column
            let line_col = line_index.line_col(e.range.start());
            ParseError::syntax_error(&e.message, line_col.line as usize, line_col.col as usize)
        })
        .collect();

    if errors.is_empty() {
        ParseResult::ok(syntax_file)
    } else {
        ParseResult::with_content_and_errors(syntax_file, errors)
    }
}
