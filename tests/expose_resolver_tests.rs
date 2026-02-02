//! Tests for SysML v2 View expose relationship resolution.

use std::sync::Arc;
use syster::hir::{ExposeRelationship, WildcardKind};

#[test]
fn test_member_expose_found() {
    let expose = ExposeRelationship::new(Arc::from("Model::Vehicle"), WildcardKind::None);
    let symbols = vec![
        "Model::Vehicle",
        "Model::Vehicle::engine",
        "Model::Aircraft",
    ];

    let result = expose.resolve(symbols.into_iter());
    assert_eq!(result.len(), 1, "Should find exactly one member");
    assert_eq!(result[0].as_ref(), "Model::Vehicle");
}

#[test]
fn test_member_expose_not_found() {
    let expose = ExposeRelationship::new(Arc::from("Model::Boat"), WildcardKind::None);
    let symbols = vec!["Model::Vehicle", "Model::Aircraft"];

    let result = expose.resolve(symbols.into_iter());
    assert_eq!(
        result.len(),
        0,
        "Should find nothing when target doesn't exist"
    );
}

#[test]
fn test_namespace_expose_direct_children() {
    let expose = ExposeRelationship::new(Arc::from("Model::Vehicle"), WildcardKind::Direct);
    let symbols = vec![
        "Model::Vehicle",
        "Model::Vehicle::engine",
        "Model::Vehicle::wheels",
        "Model::Vehicle::wheels::tire",
        "Model::Vehicle::body",
        "Model::Aircraft",
    ];

    let result = expose.resolve(symbols.into_iter());
    assert_eq!(result.len(), 3, "Should find exactly 3 direct children");

    let result_strs: Vec<&str> = result.iter().map(|s: &Arc<str>| s.as_ref()).collect();
    assert!(result_strs.contains(&"Model::Vehicle::engine"));
    assert!(result_strs.contains(&"Model::Vehicle::wheels"));
    assert!(result_strs.contains(&"Model::Vehicle::body"));
    assert!(
        !result_strs.contains(&"Model::Vehicle::wheels::tire"),
        "Should not include nested children"
    );
    assert!(
        !result_strs.contains(&"Model::Vehicle"),
        "Should not include parent"
    );
}

#[test]
fn test_namespace_expose_no_children() {
    let expose = ExposeRelationship::new(Arc::from("Model::Empty"), WildcardKind::Direct);
    let symbols = vec!["Model::Empty", "Model::Vehicle"];

    let result = expose.resolve(symbols.into_iter());
    assert_eq!(
        result.len(),
        0,
        "Should find no children for empty namespace"
    );
}

#[test]
fn test_recursive_expose_all_descendants() {
    let expose = ExposeRelationship::new(Arc::from("Model::Vehicle"), WildcardKind::Recursive);
    let symbols = vec![
        "Model::Vehicle",
        "Model::Vehicle::engine",
        "Model::Vehicle::engine::cylinder",
        "Model::Vehicle::wheels",
        "Model::Vehicle::wheels::tire",
        "Model::Vehicle::wheels::tire::tread",
        "Model::Aircraft",
    ];

    let result = expose.resolve(symbols.into_iter());
    assert_eq!(result.len(), 5, "Should find all 5 descendants");

    let result_strs: Vec<&str> = result.iter().map(|s: &Arc<str>| s.as_ref()).collect();
    assert!(result_strs.contains(&"Model::Vehicle::engine"));
    assert!(result_strs.contains(&"Model::Vehicle::engine::cylinder"));
    assert!(result_strs.contains(&"Model::Vehicle::wheels"));
    assert!(result_strs.contains(&"Model::Vehicle::wheels::tire"));
    assert!(result_strs.contains(&"Model::Vehicle::wheels::tire::tread"));
    assert!(
        !result_strs.contains(&"Model::Vehicle"),
        "Should not include parent"
    );
    assert!(
        !result_strs.contains(&"Model::Aircraft"),
        "Should not include unrelated"
    );
}

#[test]
fn test_recursive_expose_deep_nesting() {
    let expose = ExposeRelationship::new(Arc::from("A"), WildcardKind::Recursive);
    let symbols = vec!["A", "A::B", "A::B::C", "A::B::C::D", "A::B::C::D::E"];

    let result = expose.resolve(symbols.into_iter());
    assert_eq!(result.len(), 4, "Should find all nested descendants");
}

#[test]
fn test_expose_with_similar_names() {
    let expose = ExposeRelationship::new(Arc::from("Model::Vehicle"), WildcardKind::Direct);
    let symbols = vec![
        "Model::Vehicle",
        "Model::Vehicle::part",
        "Model::VehicleType",
        "Model::VehicleType::subtype",
    ];

    let result = expose.resolve(symbols.into_iter());
    assert_eq!(result.len(), 1, "Should only match exact prefix");
    assert_eq!(result[0].as_ref(), "Model::Vehicle::part");
}

#[test]
fn test_member_expose_is_member() {
    let expose = ExposeRelationship::new(Arc::from("Model::Vehicle"), WildcardKind::None);
    assert!(expose.is_member(), "Should be a member expose");
    assert!(!expose.is_namespace(), "Should not be a namespace expose");
    assert!(!expose.is_recursive(), "Should not be recursive");
}

#[test]
fn test_namespace_expose_is_namespace() {
    let expose = ExposeRelationship::new(Arc::from("Model::Vehicle"), WildcardKind::Direct);
    assert!(!expose.is_member(), "Should not be a member expose");
    assert!(expose.is_namespace(), "Should be a namespace expose");
    assert!(!expose.is_recursive(), "Should not be recursive");
}

#[test]
fn test_recursive_expose_is_recursive() {
    let expose = ExposeRelationship::new(Arc::from("Model::Vehicle"), WildcardKind::Recursive);
    assert!(!expose.is_member(), "Should not be a member expose");
    assert!(!expose.is_namespace(), "Should not be a namespace expose");
    assert!(expose.is_recursive(), "Should be recursive");
}

#[test]
fn test_expose_target_getter() {
    let target: Arc<str> = Arc::from("Model::Vehicle");
    let expose = ExposeRelationship::new(target.clone(), WildcardKind::None);
    assert_eq!(expose.target(), &target, "Should return the target");
}

#[test]
fn test_multiple_levels_direct_expose() {
    let expose = ExposeRelationship::new(Arc::from("A::B::C"), WildcardKind::Direct);
    let symbols = vec![
        "A",
        "A::B",
        "A::B::C",
        "A::B::C::D",
        "A::B::C::E",
        "A::B::C::D::F",
    ];

    let result = expose.resolve(symbols.into_iter());
    assert_eq!(
        result.len(),
        2,
        "Should find only direct children of A::B::C"
    );

    let result_strs: Vec<&str> = result.iter().map(|s: &Arc<str>| s.as_ref()).collect();
    assert!(result_strs.contains(&"A::B::C::D"));
    assert!(result_strs.contains(&"A::B::C::E"));
    assert!(!result_strs.contains(&"A::B::C::D::F"));
}

#[test]
fn test_empty_symbol_list() {
    let expose = ExposeRelationship::new(Arc::from("Model::Vehicle"), WildcardKind::Recursive);
    let symbols: Vec<&str> = vec![];

    let result = expose.resolve(symbols.into_iter());
    assert_eq!(result.len(), 0, "Should return empty for no symbols");
}

#[test]
fn test_single_level_names() {
    let expose = ExposeRelationship::new(Arc::from("Vehicle"), WildcardKind::Direct);
    let symbols = vec!["Vehicle", "Vehicle::engine", "Vehicle::wheels"];

    let result = expose.resolve(symbols.into_iter());
    assert_eq!(
        result.len(),
        2,
        "Should work with single-level qualified names"
    );
}

#[test]
fn test_case_sensitive_matching() {
    let expose = ExposeRelationship::new(Arc::from("Model::Vehicle"), WildcardKind::Direct);
    let symbols = vec![
        "Model::vehicle", // lowercase
        "Model::Vehicle::engine",
        "model::Vehicle::wheels", // wrong case in parent
    ];

    let result = expose.resolve(symbols.into_iter());
    assert_eq!(result.len(), 1, "Should be case-sensitive");
    assert_eq!(result[0].as_ref(), "Model::Vehicle::engine");
}

#[test]
fn test_expose_from_path() {
    use syster::hir::ImportPath;

    let path = ImportPath {
        target: Arc::from("Model::Vehicle"),
        wildcard: WildcardKind::Recursive,
    };
    let expose = ExposeRelationship::from_path(path);

    assert!(expose.is_recursive());
    assert_eq!(expose.target(), &Arc::from("Model::Vehicle"));
}
