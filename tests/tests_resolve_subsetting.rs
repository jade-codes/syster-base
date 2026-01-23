//! Tests for subsetting inheritance in name resolution.
//!
//! In SysML, when a feature subsets another feature, it should inherit
//! the nested members of the subsetted feature.
//!
//! Example:
//! ```sysml
//! item def Path {
//!     item edges {
//!         item vertices;  // vertices is nested in edges
//!     }
//! }
//!
//! item def Shape :> Path {
//!     item tfe :> edges;  // tfe subsets edges
//!     // tfe.vertices should resolve to edges::vertices
//! }
//! ```

use std::sync::Arc;
use syster::base::FileId;
use syster::hir::{ResolveResult, Resolver, SymbolIndex};
use syster::hir::{HirSymbol, SymbolKind, TypeRefKind, TypeRef};

fn make_symbol(
    name: &str,
    qualified: &str,
    kind: SymbolKind,
    supertypes: Vec<&str>,
) -> HirSymbol {
    HirSymbol {
        name: Arc::from(name),
        short_name: None,
        qualified_name: Arc::from(qualified),
        kind,
        file: FileId::new(0),
        start_line: 0,
        start_col: 0,
        end_line: 0,
        end_col: 0,
        short_name_start_line: None,
        short_name_start_col: None,
        short_name_end_line: None,
        short_name_end_col: None,
        doc: None,
        supertypes: supertypes.into_iter().map(Arc::from).collect(),
        type_refs: Vec::new(),
        is_public: false,
    }
}

fn make_symbol_with_type_refs(
    name: &str,
    qualified: &str,
    kind: SymbolKind,
    supertypes: Vec<&str>,
    type_refs: Vec<TypeRefKind>,
) -> HirSymbol {
    HirSymbol {
        name: Arc::from(name),
        short_name: None,
        qualified_name: Arc::from(qualified),
        kind,
        file: FileId::new(0),
        start_line: 0,
        start_col: 0,
        end_line: 0,
        end_col: 0,
        short_name_start_line: None,
        short_name_start_col: None,
        short_name_end_line: None,
        short_name_end_col: None,
        doc: None,
        supertypes: supertypes.into_iter().map(Arc::from).collect(),
        type_refs,
        is_public: false,
    }
}

/// Test basic subsetting: when `tfe :> edges`, we can find `edges` members on `tfe`
#[test]
fn test_subsetting_inherits_nested_members() {
    let mut index = SymbolIndex::new();
    
    // Setup: Path definition with edges containing vertices
    // item def Path {
    //     item edges { item vertices; }
    // }
    index.add_file(FileId::new(0), vec![
        make_symbol("Path", "Path", SymbolKind::ItemDef, vec![]),
        make_symbol("edges", "Path::edges", SymbolKind::ItemUsage, vec![]),
        make_symbol("vertices", "Path::edges::vertices", SymbolKind::ItemUsage, vec![]),
    ]);
    
    // Setup: Shape that has tfe subsetting edges
    // item def Shape :> Path {
    //     item tfe :> edges;
    // }
    index.add_file(FileId::new(1), vec![
        make_symbol("Shape", "Shape", SymbolKind::ItemDef, vec!["Path"]),
        make_symbol("tfe", "Shape::tfe", SymbolKind::ItemUsage, vec!["edges"]), // subsets edges
    ]);
    
    index.ensure_visibility_maps();
    
    // Test: Can we resolve "vertices" when looking in tfe's context?
    // This simulates resolving `tfe.vertices`
    let result = index.find_member_in_scope("Shape::tfe", "vertices");
    
    assert!(
        result.is_some(),
        "Should find 'vertices' via subsetting inheritance from edges"
    );
    assert_eq!(
        result.unwrap().qualified_name.as_ref(),
        "Path::edges::vertices"
    );
}

/// Test that direct members are found before inherited ones
#[test]
fn test_direct_member_takes_priority_over_subsetted() {
    let mut index = SymbolIndex::new();
    
    // edges with vertices
    index.add_file(FileId::new(0), vec![
        make_symbol("Path", "Path", SymbolKind::ItemDef, vec![]),
        make_symbol("edges", "Path::edges", SymbolKind::ItemUsage, vec![]),
        make_symbol("vertices", "Path::edges::vertices", SymbolKind::ItemUsage, vec![]),
    ]);
    
    // tfe subsets edges BUT also has its own vertices
    index.add_file(FileId::new(1), vec![
        make_symbol("Shape", "Shape", SymbolKind::ItemDef, vec!["Path"]),
        make_symbol("tfe", "Shape::tfe", SymbolKind::ItemUsage, vec!["edges"]),
        make_symbol("vertices", "Shape::tfe::vertices", SymbolKind::ItemUsage, vec![]), // direct member
    ]);
    
    index.ensure_visibility_maps();
    
    // Test: Direct member should be found, not the inherited one
    let result = index.find_member_in_scope("Shape::tfe", "vertices");
    
    assert!(result.is_some());
    assert_eq!(
        result.unwrap().qualified_name.as_ref(),
        "Shape::tfe::vertices",
        "Direct nested member should take priority over subsetted"
    );
}

/// Test chain resolution: a.b.c where b subsets something with c
#[test]
fn test_feature_chain_with_subsetting() {
    let mut index = SymbolIndex::new();
    
    // Base definitions
    index.add_file(FileId::new(0), vec![
        make_symbol("Polyhedron", "Polyhedron", SymbolKind::ItemDef, vec![]),
        make_symbol("edges", "Polyhedron::edges", SymbolKind::ItemUsage, vec![]),
        make_symbol("vertices", "Polyhedron::edges::vertices", SymbolKind::ItemUsage, vec![]),
    ]);
    
    // Shape with tfe subsetting edges
    index.add_file(FileId::new(1), vec![
        make_symbol("Shape", "Shape", SymbolKind::ItemDef, vec!["Polyhedron"]),
        make_symbol("tfe", "Shape::tfe", SymbolKind::ItemUsage, vec!["edges"]),
    ]);
    
    // A usage of Shape
    index.add_file(FileId::new(2), vec![
        make_symbol("myShape", "Test::myShape", SymbolKind::ItemUsage, vec!["Shape"]),
    ]);
    
    index.ensure_visibility_maps();
    
    // Test: Resolve the chain myShape.tfe.vertices
    // 1. myShape -> Shape (via typing)
    // 2. tfe -> Shape::tfe (member of Shape)
    // 3. vertices -> should come from edges via subsetting
    
    // First verify tfe can be found in Shape
    let tfe_result = index.find_member_in_scope("Shape", "tfe");
    assert!(tfe_result.is_some(), "Should find tfe in Shape");
    
    // Then verify vertices can be found via tfe's subsetting
    let vertices_result = index.find_member_in_scope("Shape::tfe", "vertices");
    assert!(
        vertices_result.is_some(),
        "Should find vertices in tfe via subsetting inheritance"
    );
}

/// Test that typing (`:`) and subsetting (`:>`) both work for member inheritance
#[test]
fn test_typing_vs_subsetting_inheritance() {
    let mut index = SymbolIndex::new();
    
    // Definition with nested member
    index.add_file(FileId::new(0), vec![
        make_symbol("EdgeDef", "EdgeDef", SymbolKind::ItemDef, vec![]),
        make_symbol("vertices", "EdgeDef::vertices", SymbolKind::ItemUsage, vec![]),
    ]);
    
    // Base definition with edges usage
    index.add_file(FileId::new(1), vec![
        make_symbol("Base", "Base", SymbolKind::ItemDef, vec![]),
        make_symbol("edges", "Base::edges", SymbolKind::ItemUsage, vec!["EdgeDef"]),
    ]);
    
    // Test extends Base, so it inherits edges
    // Usage typed by definition (: EdgeDef)
    // Usage subsetting inherited member (:> edges)
    index.add_file(FileId::new(2), vec![
        make_symbol("Test", "Test", SymbolKind::ItemDef, vec!["Base"]), // Test :> Base
        make_symbol("typedEdge", "Test::typedEdge", SymbolKind::ItemUsage, vec!["EdgeDef"]),
        make_symbol("subsettingEdge", "Test::subsettingEdge", SymbolKind::ItemUsage, vec!["edges"]),
    ]);
    
    index.ensure_visibility_maps();
    
    // Both should be able to access vertices
    let typed_result = index.find_member_in_scope("Test::typedEdge", "vertices");
    assert!(
        typed_result.is_some(),
        "Typed usage should find members from definition"
    );
    
    // For subsetting, we need to follow: subsettingEdge -> edges (inherited from Base) -> EdgeDef -> vertices
    // This requires following the full chain
    let subsetted_result = index.find_member_in_scope("Test::subsettingEdge", "vertices");
    // Note: This test documents the expected behavior, may fail until implemented
    assert!(
        subsetted_result.is_some(),
        "Subsetting usage should find members from subsetted feature's type"
    );
}

/// Test real-world pattern from ShapeItems library
#[test]
fn test_shapeitems_pattern() {
    let mut index = SymbolIndex::new();
    
    // Simplified version of the ShapeItems pattern:
    // item def Polyhedron {
    //     item edges { item vertices; }
    // }
    // item def CuboidOrTriangularPrism :> Polyhedron {
    //     item tfe :> edges;
    //     // binding bind tfe.vertices = ...
    // }
    
    index.add_file(FileId::new(0), vec![
        make_symbol("Polyhedron", "ShapeItems::Polyhedron", SymbolKind::ItemDef, vec![]),
        make_symbol("edges", "ShapeItems::Polyhedron::edges", SymbolKind::ItemUsage, vec![]),
        make_symbol("vertices", "ShapeItems::Polyhedron::edges::vertices", SymbolKind::ItemUsage, vec![]),
    ]);
    
    index.add_file(FileId::new(1), vec![
        make_symbol("CuboidOrTriangularPrism", "ShapeItems::CuboidOrTriangularPrism", SymbolKind::ItemDef, vec!["Polyhedron"]),
        make_symbol("tfe", "ShapeItems::CuboidOrTriangularPrism::tfe", SymbolKind::ItemUsage, vec!["edges"]),
    ]);
    
    index.ensure_visibility_maps();
    
    // The key test: can we find vertices through tfe's subsetting of edges?
    let result = index.find_member_in_scope("ShapeItems::CuboidOrTriangularPrism::tfe", "vertices");
    
    assert!(
        result.is_some(),
        "Should find 'vertices' in tfe via subsetting inheritance from Polyhedron::edges"
    );
}
