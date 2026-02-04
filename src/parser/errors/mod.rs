//! Parser error handling module
//!
//! This module provides enhanced error handling for the SysML/KerML parser:
//! - Categorized error codes for filtering and documentation
//! - Context-aware error messages
//! - Suggestions/hints for common mistakes
//! - Related span tracking (e.g., "opened here" for unclosed braces)

mod codes;
mod context;
mod error;

pub use codes::ErrorCode;
pub use context::ParseContext;
pub use error::{RelatedInfo, Severity, SyntaxError};

#[cfg(test)]
mod tests;
