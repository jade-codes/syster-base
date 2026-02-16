// Syntax definitions for supported languages
pub mod file;
pub mod formatter;
pub mod parser;

pub use file::SyntaxFile;
pub use formatter::{FormatOptions, format_async};
pub use parser::{ParseError, ParseResult, load_and_parse, parse_content, parse_with_result};

// Re-export Position and Span from base for backwards compatibility
pub use crate::base::{Position, Span};

#[cfg(test)]
mod tests;
