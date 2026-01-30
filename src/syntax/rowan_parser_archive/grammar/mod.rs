//! Grammar modules organized by language section
//!
//! This mirrors the Pest grammar structure:
//! - `kerml_expressions.rs` - Shared expression sub-language (precedence chain)
//! - `kerml.rs` - Core KerML grammar (class, struct, feature, relationships, etc.)
//! - `sysml.rs` - SysML extensions (part, action, requirement, etc.)
//!
//! ## Architecture
//!
//! Parsing functions are generic over traits that the main parser implements:
//! - `ExpressionParser` - Base trait for expression parsing
//! - `KerMLParser` - Extends ExpressionParser with KerML-specific methods
//! - `SysMLParser` - Extends KerMLParser with SysML-specific methods

pub mod kerml_expressions;
pub mod kerml;
pub mod sysml;

pub use kerml_expressions::ExpressionParser;
pub use kerml::{
    is_kerml_definition_keyword, is_kerml_usage_keyword, is_standalone_relationship_keyword,
    KerMLParser, parse_standalone_relationship, parse_annotation,
    STANDALONE_RELATIONSHIP_KEYWORDS,
};
pub use sysml::{is_sysml_definition_keyword, is_sysml_usage_keyword, SysMLParser};
