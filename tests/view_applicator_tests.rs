//! Tests for SysML v2 View application (expose + filter combined).

#![allow(clippy::useless_vec)]

use std::sync::Arc;
use syster::hir::{ExposeRelationship, FilterCondition, ViewDefinition, ViewUsage, WildcardKind};

#[test]
fn test_apply_member_expose_no_filter() {
    let mut view = ViewDefinition::new();
    view.add_expose(ExposeRelationship::new(
        Arc::from("Model::Vehicle"),
        WildcardKind::None,
    ));

    let symbols = vec![
        ("Model::Vehicle", vec![Arc::from("SysML::PartDef")]),
        (
            "Model::Vehicle::engine",
            vec![Arc::from("SysML::PartUsage")],
        ),
        ("Model::Aircraft", vec![Arc::from("SysML::PartDef")]),
    ];

    let result = view.apply(symbols.iter().map(|(qn, meta)| (*qn, meta.as_slice())));
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].as_ref(), "Model::Vehicle");
}

#[test]
fn test_apply_namespace_expose_no_filter() {
    let mut view = ViewDefinition::new();
    view.add_expose(ExposeRelationship::new(
        Arc::from("Model::Vehicle"),
        WildcardKind::Direct,
    ));

    let symbols = vec![
        ("Model::Vehicle", vec![]),
        ("Model::Vehicle::engine", vec![]),
        ("Model::Vehicle::wheels", vec![]),
        ("Model::Vehicle::wheels::tire", vec![]),
    ];

    let result = view.apply(symbols.iter().map(|(qn, meta)| (*qn, meta.as_slice())));
    assert_eq!(result.len(), 2); // Only direct children

    let result_strs: Vec<&str> = result.iter().map(|s| s.as_ref()).collect();
    assert!(result_strs.contains(&"Model::Vehicle::engine"));
    assert!(result_strs.contains(&"Model::Vehicle::wheels"));
}

#[test]
fn test_apply_recursive_expose_no_filter() {
    let mut view = ViewDefinition::new();
    view.add_expose(ExposeRelationship::new(
        Arc::from("Model::Vehicle"),
        WildcardKind::Recursive,
    ));

    let symbols = vec![
        ("Model::Vehicle", vec![]),
        ("Model::Vehicle::engine", vec![]),
        ("Model::Vehicle::engine::cylinder", vec![]),
        ("Model::Vehicle::wheels", vec![]),
        ("Model::Aircraft", vec![]),
    ];

    let result = view.apply(symbols.iter().map(|(qn, meta)| (*qn, meta.as_slice())));
    assert_eq!(result.len(), 3); // All descendants

    let result_strs: Vec<&str> = result.iter().map(|s| s.as_ref()).collect();
    assert!(result_strs.contains(&"Model::Vehicle::engine"));
    assert!(result_strs.contains(&"Model::Vehicle::engine::cylinder"));
    assert!(result_strs.contains(&"Model::Vehicle::wheels"));
}

#[test]
fn test_apply_namespace_expose_with_filter() {
    let mut view = ViewDefinition::new();
    view.add_expose(ExposeRelationship::new(
        Arc::from("Model::Vehicle"),
        WildcardKind::Direct,
    ));
    view.add_filter(FilterCondition::metadata(Arc::from("SysML::PartUsage")));

    let symbols = vec![
        (
            "Model::Vehicle::engine",
            vec![Arc::from("SysML::PartUsage")],
        ),
        (
            "Model::Vehicle::wheels",
            vec![Arc::from("SysML::PartUsage")],
        ),
        (
            "Model::Vehicle::name",
            vec![Arc::from("SysML::AttributeUsage")],
        ),
        (
            "Model::Vehicle::speed",
            vec![Arc::from("SysML::AttributeUsage")],
        ),
    ];

    let result = view.apply(symbols.iter().map(|(qn, meta)| (*qn, meta.as_slice())));
    assert_eq!(result.len(), 2); // Only PartUsage elements

    let result_strs: Vec<&str> = result.iter().map(|s| s.as_ref()).collect();
    assert!(result_strs.contains(&"Model::Vehicle::engine"));
    assert!(result_strs.contains(&"Model::Vehicle::wheels"));
}

#[test]
fn test_apply_multiple_expose_relationships() {
    let mut view = ViewDefinition::new();
    view.add_expose(ExposeRelationship::new(
        Arc::from("Model::Vehicle::engine"),
        WildcardKind::None,
    ));
    view.add_expose(ExposeRelationship::new(
        Arc::from("Model::Vehicle::wheels"),
        WildcardKind::None,
    ));

    let symbols = vec![
        ("Model::Vehicle::engine", vec![]),
        ("Model::Vehicle::wheels", vec![]),
        ("Model::Vehicle::body", vec![]),
    ];

    let result = view.apply(symbols.iter().map(|(qn, meta)| (*qn, meta.as_slice())));
    assert_eq!(result.len(), 2);

    let result_strs: Vec<&str> = result.iter().map(|s| s.as_ref()).collect();
    assert!(result_strs.contains(&"Model::Vehicle::engine"));
    assert!(result_strs.contains(&"Model::Vehicle::wheels"));
}

#[test]
fn test_apply_multiple_filters_and_logic() {
    let mut view = ViewDefinition::new();
    view.add_expose(ExposeRelationship::new(
        Arc::from("Model"),
        WildcardKind::Recursive,
    ));
    view.add_filter(FilterCondition::metadata(Arc::from("SysML::PartUsage")));
    view.add_filter(FilterCondition::metadata(Arc::from("Doc::important")));

    let symbols = vec![
        (
            "Model::A",
            vec![Arc::from("SysML::PartUsage"), Arc::from("Doc::important")],
        ),
        ("Model::B", vec![Arc::from("SysML::PartUsage")]),
        ("Model::C", vec![Arc::from("Doc::important")]),
        ("Model::D", vec![]),
    ];

    let result = view.apply(symbols.iter().map(|(qn, meta)| (*qn, meta.as_slice())));
    assert_eq!(result.len(), 1); // Only A has both annotations
    assert_eq!(result[0].as_ref(), "Model::A");
}

#[test]
fn test_apply_no_expose_returns_empty() {
    let view = ViewDefinition::new(); // No expose relationships

    let symbols = vec![("Model::Vehicle", vec![]), ("Model::Aircraft", vec![])];

    let result = view.apply(symbols.iter().map(|(qn, meta)| (*qn, meta.as_slice())));
    assert_eq!(result.len(), 0, "No expose = nothing visible");
}

#[test]
fn test_apply_expose_nonexistent_element() {
    let mut view = ViewDefinition::new();
    view.add_expose(ExposeRelationship::new(
        Arc::from("Model::Nonexistent"),
        WildcardKind::Direct,
    ));

    let symbols = vec![("Model::Vehicle", vec![]), ("Model::Aircraft", vec![])];

    let result = view.apply(symbols.iter().map(|(qn, meta)| (*qn, meta.as_slice())));
    assert_eq!(result.len(), 0, "Nonexistent target = nothing visible");
}

#[test]
fn test_apply_filter_excludes_all() {
    let mut view = ViewDefinition::new();
    view.add_expose(ExposeRelationship::new(
        Arc::from("Model"),
        WildcardKind::Recursive,
    ));
    view.add_filter(FilterCondition::metadata(Arc::from("NonexistentMetadata")));

    let symbols = vec![
        ("Model::A", vec![Arc::from("SysML::PartUsage")]),
        ("Model::B", vec![Arc::from("SysML::ActionUsage")]),
    ];

    let result = view.apply(symbols.iter().map(|(qn, meta)| (*qn, meta.as_slice())));
    assert_eq!(result.len(), 0, "Filter excludes everything");
}

#[test]
fn test_apply_empty_symbol_list() {
    let mut view = ViewDefinition::new();
    view.add_expose(ExposeRelationship::new(
        Arc::from("Model"),
        WildcardKind::Recursive,
    ));

    let symbols: Vec<(&str, Vec<Arc<str>>)> = vec![];

    let result = view.apply(symbols.iter().map(|(qn, meta)| (*qn, meta.as_slice())));
    assert_eq!(result.len(), 0);
}

#[test]
fn test_apply_overlapping_expose_relationships() {
    let mut view = ViewDefinition::new();
    // Add overlapping expose relationships
    view.add_expose(ExposeRelationship::new(
        Arc::from("Model::Vehicle"),
        WildcardKind::Recursive,
    ));
    view.add_expose(ExposeRelationship::new(
        Arc::from("Model::Vehicle::engine"),
        WildcardKind::None,
    ));

    let symbols = vec![
        ("Model::Vehicle::engine", vec![]),
        ("Model::Vehicle::wheels", vec![]),
    ];

    let result = view.apply(symbols.iter().map(|(qn, meta)| (*qn, meta.as_slice())));
    assert_eq!(result.len(), 2); // Both exposed (HashSet deduplicates)
}

#[test]
fn test_apply_suffix_metadata_matching() {
    let mut view = ViewDefinition::new();
    view.add_expose(ExposeRelationship::new(
        Arc::from("Model"),
        WildcardKind::Recursive,
    ));
    view.add_filter(FilterCondition::metadata(Arc::from("PartUsage"))); // Short form

    let symbols = vec![
        ("Model::A", vec![Arc::from("SysML::PartUsage")]), // Long form
        ("Model::B", vec![Arc::from("SysML::ActionUsage")]),
    ];

    let result = view.apply(symbols.iter().map(|(qn, meta)| (*qn, meta.as_slice())));
    assert_eq!(result.len(), 1); // Suffix matching works
    assert_eq!(result[0].as_ref(), "Model::A");
}

#[test]
fn test_view_usage_apply() {
    let mut view_usage = ViewUsage::new(Some(Arc::from("MyViewDef")));
    view_usage.add_expose(ExposeRelationship::new(
        Arc::from("Model"),
        WildcardKind::Direct,
    ));
    view_usage.add_filter(FilterCondition::metadata(Arc::from("SysML::PartUsage")));

    let symbols = vec![
        ("Model::A", vec![Arc::from("SysML::PartUsage")]),
        ("Model::B", vec![Arc::from("SysML::ActionUsage")]),
    ];

    let result = view_usage.apply(symbols.iter().map(|(qn, meta)| (*qn, meta.as_slice())));
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].as_ref(), "Model::A");
}

#[test]
fn test_apply_elements_without_metadata() {
    let mut view = ViewDefinition::new();
    view.add_expose(ExposeRelationship::new(
        Arc::from("Model"),
        WildcardKind::Direct,
    ));
    view.add_filter(FilterCondition::metadata(Arc::from("SysML::PartUsage")));

    let symbols = vec![
        ("Model::A", vec![Arc::from("SysML::PartUsage")]),
        ("Model::B", vec![]), // No metadata
    ];

    let result = view.apply(symbols.iter().map(|(qn, meta)| (*qn, meta.as_slice())));
    assert_eq!(result.len(), 1); // B filtered out (no matching metadata)
    assert_eq!(result[0].as_ref(), "Model::A");
}

#[test]
fn test_apply_complex_hierarchy_with_filters() {
    let mut view = ViewDefinition::new();
    view.add_expose(ExposeRelationship::new(
        Arc::from("Systems"),
        WildcardKind::Recursive,
    ));
    view.add_filter(FilterCondition::metadata(Arc::from("SysML::PartUsage")));

    let symbols = vec![
        ("Systems::Vehicle", vec![Arc::from("SysML::PartDef")]),
        (
            "Systems::Vehicle::engine",
            vec![Arc::from("SysML::PartUsage")],
        ),
        (
            "Systems::Vehicle::engine::cylinder",
            vec![Arc::from("SysML::PartUsage")],
        ),
        (
            "Systems::Vehicle::name",
            vec![Arc::from("SysML::AttributeUsage")],
        ),
        (
            "Systems::Aircraft::wing",
            vec![Arc::from("SysML::PartUsage")],
        ),
    ];

    let result = view.apply(symbols.iter().map(|(qn, meta)| (*qn, meta.as_slice())));
    assert_eq!(result.len(), 3); // engine, cylinder, wing

    let result_strs: Vec<&str> = result.iter().map(|s| s.as_ref()).collect();
    assert!(result_strs.contains(&"Systems::Vehicle::engine"));
    assert!(result_strs.contains(&"Systems::Vehicle::engine::cylinder"));
    assert!(result_strs.contains(&"Systems::Aircraft::wing"));
}
