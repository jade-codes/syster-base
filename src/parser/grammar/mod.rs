//! Grammar modules for KerML and SysML parsing
//!
//! This module contains the language-specific parsing logic organized by grammar:
//! - `kerml` - Core KerML constructs (definitions, usages, relationships)
//! - `kerml_expressions` - Expression parsing (shared between KerML and SysML)
//! - `sysml` - SysML-specific extensions (action bodies, state machines, requirements)
//!
//! The parsing functions are generic over a trait (`ExpressionParser` / `KerMLParser` / `SysMLParser`)
//! so they can be used with any parser implementation.

pub mod kerml;
pub mod kerml_expressions;
pub mod sysml;

pub use kerml_expressions::ExpressionParser;
pub use kerml::{KerMLParser, parse_kerml_file, parse_namespace_element};
pub use sysml::SysMLParser;
