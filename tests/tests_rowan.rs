//! Rowan Parser Test Suite
//!
//! This module contains duplicate tests migrated from the Pest parser to the rowan parser.
//! Each test file here mirrors a corresponding test file in the tests/ directory.
//!
//! Run with: `cargo test --test tests_rowan`
//!
//! NOTE: The rowan test modules are currently disabled as the tests have been
//! integrated into the main library tests (src/parser/ast.rs tests).
//!
//! The test infrastructure in tests/rowan/ and tests/parser/ was lost and needs
//! to be recreated. For now, all rowan parser tests are in src/parser/ast.rs.

// All rowan tests are now in src/parser/ast.rs as unit tests