//! Parser Tests - SysML Connectors and Flows
//!
//! Phase 1: Parser/AST Layer
//! Tests for connections, interfaces, flows, and allocations.
//!
//! Test data from tests_parser_sysml_pest.rs.archived.

use rstest::rstest;
use syster::parser::{AstNode, SourceFile, parse_sysml};

fn parses_sysml(input: &str) -> bool {
    let parsed = parse_sysml(input);
    SourceFile::cast(parsed.syntax()).is_some()
}

// ============================================================================
// Connection Definitions
// ============================================================================

#[rstest]
#[case("connection def MyConn;")]
#[case("connection def MyConn {}")]
#[case("connection def MyConnection;")]
#[case("connection def MyConnection { }")]
#[case("connection def ProductSelection { item info: SelectionInfo; end [0..1] item cart: ShoppingCart[1]; end [0..*] nonunique item selectedProduct: Product[1]; }")]
#[case("connection def C { end [1] item a: A; }")]
#[case("connection def C { end [0..*] ordered item x: X; }")]
fn test_connection_def(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Connection Usages
// ============================================================================

#[rstest]
#[case("package P { connection myConn; }")]
#[case("package P { connection myConn { } }")]
#[case("package P { connection myConn connect source to target; }")]
fn test_connection_usage(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Interface Definitions
// ============================================================================

#[rstest]
#[case("interface def MyInterface;")]
#[case("interface def MyInterface {}")]
#[case("interface def MyInterface { port p1; port p2; }")]
fn test_interface_def(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Interface Usages
// ============================================================================

#[rstest]
#[case("package P { interface myInterface; }")]
#[case("package P { interface myInterface connect a to b; }")]
fn test_interface_usage(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Flow Definitions
// ============================================================================

#[rstest]
#[case("flow def MyFlow;")]
#[case("flow def MyFlow {}")]
fn test_flow_def(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Flow Usages
// ============================================================================

#[rstest]
#[case("package P { flow myFlow; }")]
#[case("package P { flow myFlow from source to target; }")]
fn test_flow_usage(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Allocation Definitions
// ============================================================================

#[rstest]
#[case("allocation def MyAlloc;")]
#[case("allocation def MyAlloc {}")]
fn test_allocation_def(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Allocation Usages
// ============================================================================

#[rstest]
#[case("package P { allocation myAlloc; }")]
#[case("package P { allocation myAlloc allocate source to target; }")]
fn test_allocation_usage(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Binding Connectors
// ============================================================================

#[rstest]
#[case("part def P { bind x = y; }")]
#[case("part def P { binding b of x = y; }")]
fn test_bindings(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Message and Send
// ============================================================================

#[rstest]
#[case("action def A { send msg via channel; }")]
#[case("action def A { accept msg via channel; }")]
fn test_message_actions(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Perform and Exhibit
// ============================================================================

#[rstest]
#[case("part def P { perform myAction; }")]
#[case("part def P { exhibit myState; }")]
fn test_perform_exhibit(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Include
// ============================================================================

#[rstest]
#[case("use case def UC { include otherCase; }")]
fn test_include(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}
