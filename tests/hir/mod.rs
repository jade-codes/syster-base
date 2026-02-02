//! HIR layer tests
//!
//! Tests for the High-level IR (HIR) semantic model:
//! - Symbol extraction from SysML and KerML
//! - Name resolution
//! - Import resolution
//! - Cross-file resolution
//! - Semantic diagnostics
//! - Standard library loading

pub mod tests_chain_member_hover;
pub mod tests_diagnostics;
pub mod tests_edge_cases;
pub mod tests_hover_connections;
pub mod tests_hover_constraint_def;
pub mod tests_hover_control_flow;
pub mod tests_hover_expressions;
pub mod tests_hover_metadata;
pub mod tests_hover_satisfy_via;
pub mod tests_hover_triage;
pub mod tests_import_resolution;
pub mod tests_kerml_extraction;
pub mod tests_name_resolution;
pub mod tests_spans;
pub mod tests_stdlib;
pub mod tests_symbol_extraction;
pub mod tests_type_refs;
