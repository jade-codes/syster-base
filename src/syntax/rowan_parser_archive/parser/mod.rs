//! Parser modules organized by grammar section
//!
//! This mirrors the Pest grammar structure:
//! - `kerml_expressions.rs` - Shared expression sub-language
//! - `kerml.rs` - Core KerML grammar (planned)
//! - `sysml.rs` - SysML extensions (planned)

mod kerml_expressions;

pub use kerml_expressions::ExpressionParser;
