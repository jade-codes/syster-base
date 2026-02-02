//! Tests for SysML v2 View filter evaluation.

use std::sync::Arc;
use syster::hir::{FilterCondition, ViewDefinition, ViewUsage};

#[test]
fn test_metadata_filter_exact_match() {
    let filter = FilterCondition::metadata(Arc::from("SysML::PartUsage"));
    let metadata = vec![Arc::from("SysML::PartUsage")];

    assert!(
        filter.matches(&metadata),
        "Should match exact metadata annotation"
    );
}

#[test]
fn test_metadata_filter_suffix_match() {
    let filter = FilterCondition::metadata(Arc::from("PartUsage"));
    let metadata = vec![Arc::from("SysML::PartUsage")];

    assert!(filter.matches(&metadata), "Should match by suffix");
}

#[test]
fn test_metadata_filter_multiple_annotations() {
    let filter = FilterCondition::metadata(Arc::from("SysML::PartUsage"));
    let metadata = vec![
        Arc::from("Doc::note"),
        Arc::from("SysML::PartUsage"),
        Arc::from("Custom::annotation"),
    ];

    assert!(
        filter.matches(&metadata),
        "Should find match among multiple annotations"
    );
}

#[test]
fn test_metadata_filter_no_match() {
    let filter = FilterCondition::metadata(Arc::from("SysML::PartUsage"));
    let metadata = vec![Arc::from("SysML::ActionUsage")];

    assert!(
        !filter.matches(&metadata),
        "Should not match wrong annotation"
    );
}

#[test]
fn test_metadata_filter_empty_metadata() {
    let filter = FilterCondition::metadata(Arc::from("SysML::PartUsage"));
    let metadata = vec![];

    assert!(
        !filter.matches(&metadata),
        "Should not match when no metadata present"
    );
}

#[test]
fn test_expression_filter_unimplemented() {
    let filter = FilterCondition::expression("element.type == 'PartDef'".to_string());
    let metadata = vec![];

    // Expression filters not yet implemented, should return true (pass-through)
    assert!(
        filter.matches(&metadata),
        "Unimplemented expression filter should pass"
    );
}

#[test]
fn test_view_definition_no_filters() {
    let view = ViewDefinition::new();
    let metadata = vec![Arc::from("SysML::PartUsage")];

    assert!(
        view.passes_filters(&metadata),
        "View with no filters should pass all elements"
    );
}

#[test]
fn test_view_definition_single_filter_passes() {
    let mut view = ViewDefinition::new();
    view.add_filter(FilterCondition::metadata(Arc::from("SysML::PartUsage")));

    let metadata = vec![Arc::from("SysML::PartUsage")];
    assert!(
        view.passes_filters(&metadata),
        "Element with matching metadata should pass"
    );
}

#[test]
fn test_view_definition_single_filter_fails() {
    let mut view = ViewDefinition::new();
    view.add_filter(FilterCondition::metadata(Arc::from("SysML::PartUsage")));

    let metadata = vec![Arc::from("SysML::ActionUsage")];
    assert!(
        !view.passes_filters(&metadata),
        "Element with non-matching metadata should fail"
    );
}

#[test]
fn test_view_definition_multiple_filters_and_logic() {
    let mut view = ViewDefinition::new();
    view.add_filter(FilterCondition::metadata(Arc::from("SysML::PartUsage")));
    view.add_filter(FilterCondition::metadata(Arc::from("Doc::important")));

    // Element with both annotations should pass
    let metadata_both = vec![Arc::from("SysML::PartUsage"), Arc::from("Doc::important")];
    assert!(
        view.passes_filters(&metadata_both),
        "Element with all required metadata should pass"
    );

    // Element with only one annotation should fail
    let metadata_partial = vec![Arc::from("SysML::PartUsage")];
    assert!(
        !view.passes_filters(&metadata_partial),
        "Element missing required metadata should fail"
    );
}

#[test]
fn test_view_definition_multiple_filters_all_fail() {
    let mut view = ViewDefinition::new();
    view.add_filter(FilterCondition::metadata(Arc::from("SysML::PartUsage")));
    view.add_filter(FilterCondition::metadata(Arc::from("Doc::important")));

    let metadata = vec![Arc::from("SysML::ActionUsage")];
    assert!(
        !view.passes_filters(&metadata),
        "Element with no matching metadata should fail"
    );
}

#[test]
fn test_view_usage_no_filters() {
    let view = ViewUsage::new(None);
    let metadata = vec![Arc::from("SysML::PartUsage")];

    assert!(
        view.passes_filters(&metadata),
        "ViewUsage with no filters should pass all elements"
    );
}

#[test]
fn test_view_usage_single_filter_passes() {
    let mut view = ViewUsage::new(Some(Arc::from("MyViewDef")));
    view.add_filter(FilterCondition::metadata(Arc::from("SysML::PartUsage")));

    let metadata = vec![Arc::from("SysML::PartUsage")];
    assert!(
        view.passes_filters(&metadata),
        "Element with matching metadata should pass"
    );
}

#[test]
fn test_view_usage_single_filter_fails() {
    let mut view = ViewUsage::new(Some(Arc::from("MyViewDef")));
    view.add_filter(FilterCondition::metadata(Arc::from("SysML::PartUsage")));

    let metadata = vec![Arc::from("SysML::ActionUsage")];
    assert!(
        !view.passes_filters(&metadata),
        "Element with non-matching metadata should fail"
    );
}

#[test]
fn test_complex_qualified_names() {
    let filter = FilterCondition::metadata(Arc::from("Systems::Modeling::PartUsage"));

    // Exact match
    let metadata_exact = vec![Arc::from("Systems::Modeling::PartUsage")];
    assert!(
        filter.matches(&metadata_exact),
        "Should match exact qualified name"
    );

    // Suffix match
    let filter_short = FilterCondition::metadata(Arc::from("PartUsage"));
    assert!(
        filter_short.matches(&metadata_exact),
        "Should match by suffix even with long qualified name"
    );
}

#[test]
fn test_case_sensitive_matching() {
    let filter = FilterCondition::metadata(Arc::from("SysML::PartUsage"));
    let metadata = vec![Arc::from("SysML::partusage")]; // different case

    assert!(
        !filter.matches(&metadata),
        "Matching should be case-sensitive"
    );
}

#[test]
fn test_partial_name_no_match() {
    let filter = FilterCondition::metadata(Arc::from("Part"));
    let metadata = vec![Arc::from("SysML::PartUsage")];

    // "Part" should NOT match "PartUsage" - must be exact or suffix match
    assert!(!filter.matches(&metadata), "Partial name should not match");
}
