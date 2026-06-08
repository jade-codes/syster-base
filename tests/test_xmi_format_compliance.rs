//! Tests for XMI format compliance with OMG standard.
//!
//! These tests verify that our XMI writer produces output that matches
//! the official OMG SysML v2 XMI format as closely as possible.
#![cfg(feature = "interchange")]

use syster::interchange::{ModelFormat, Xmi, model::*};

/// Test that XML declaration uses ASCII encoding.
#[test]
fn test_xml_declaration_encoding() {
    let mut model = Model::new();
    let elem = Element::new("test-id", ElementKind::Package);
    model.add_element(elem);

    let xmi = Xmi;
    let output = xmi.write(&model).expect("write");
    let output_str = String::from_utf8_lossy(&output);

    assert!(
        output_str.contains(r#"encoding="ASCII""#),
        "Expected ASCII encoding, got: {}",
        output_str.lines().next().unwrap_or("")
    );
}

/// Test that single-root documents use the element as root (no xmi:XMI wrapper).
#[test]
fn test_single_root_no_wrapper() {
    let mut model = Model::new();
    let elem = Element::new("test-id", ElementKind::Package);
    model.add_element(elem);

    let xmi = Xmi;
    let output = xmi.write(&model).expect("write");
    let output_str = String::from_utf8_lossy(&output);

    // Should NOT have xmi:XMI wrapper for single root
    assert!(
        !output_str.contains("<xmi:XMI"),
        "Single root should not have xmi:XMI wrapper"
    );
    // Should start with the actual element type
    assert!(
        output_str.contains("<sysml:Package"),
        "Should start with element type, got: {}",
        &output_str[..200.min(output_str.len())]
    );
}

/// Test that root element has xmi:version="2.0".
#[test]
fn test_xmi_version_attribute() {
    let mut model = Model::new();
    let elem = Element::new("test-id", ElementKind::Package);
    model.add_element(elem);

    let xmi = Xmi;
    let output = xmi.write(&model).expect("write");
    let output_str = String::from_utf8_lossy(&output);

    assert!(
        output_str.contains(r#"xmi:version="2.0""#),
        "Expected xmi:version=\"2.0\", got: {}",
        output_str.lines().nth(1).unwrap_or("")
    );
}

/// Test that elements have both xmi:id and elementId attributes.
#[test]
fn test_element_id_duplication() {
    let mut model = Model::new();
    let elem = Element::new("test-uuid-123", ElementKind::Package);
    model.add_element(elem);

    let xmi = Xmi;
    let output = xmi.write(&model).expect("write");
    let output_str = String::from_utf8_lossy(&output);

    assert!(
        output_str.contains(r#"xmi:id="test-uuid-123""#),
        "Expected xmi:id attribute"
    );
    assert!(
        output_str.contains(r#"elementId="test-uuid-123""#),
        "Expected elementId attribute with same value"
    );
}

/// Test that SysML elements use declaredName instead of name.
#[test]
fn test_declared_name_for_sysml() {
    let mut model = Model::new();
    let mut elem = Element::new("test-id", ElementKind::PartDefinition);
    elem.name = Some("Vehicle".into());
    model.add_element(elem);

    let xmi = Xmi;
    let output = xmi.write(&model).expect("write");
    let output_str = String::from_utf8_lossy(&output);

    assert!(
        output_str.contains(r#"declaredName="Vehicle""#),
        "SysML elements should use declaredName, got: {}",
        output_str
    );
    // Should NOT have plain name= for SysML elements
    assert!(
        !output_str.contains(r#" name="Vehicle""#),
        "SysML elements should not use plain name attribute"
    );
}

/// Test that namespace URIs match the 2025 spec version.
#[test]
fn test_namespace_uris() {
    let mut model = Model::new();
    let elem = Element::new("test-id", ElementKind::Package);
    model.add_element(elem);

    let xmi = Xmi;
    let output = xmi.write(&model).expect("write");
    let output_str = String::from_utf8_lossy(&output);

    assert!(
        output_str.contains("https://www.omg.org/spec/SysML/20250201"),
        "Expected SysML 2025 namespace URI"
    );
    assert!(
        output_str.contains("https://www.omg.org/spec/KerML/20250201"),
        "Expected KerML 2025 namespace URI"
    );
}

/// Test that xsi namespace is declared.
#[test]
fn test_xsi_namespace_declared() {
    let mut model = Model::new();
    let elem = Element::new("test-id", ElementKind::Package);
    model.add_element(elem);

    let xmi = Xmi;
    let output = xmi.write(&model).expect("write");
    let output_str = String::from_utf8_lossy(&output);

    assert!(
        output_str.contains("xmlns:xsi="),
        "Expected xsi namespace declaration"
    );
}

/// Test that child relationships use xsi:type attribute.
#[test]
fn test_owned_relationship_xsi_type() {
    let mut model = Model::new();

    // Create parent package
    let mut pkg = Element::new("pkg-id", ElementKind::Package);
    pkg.name = Some("TestPackage".into());

    // Create child membership relationship
    let membership = Element::new("mem-id", ElementKind::OwningMembership);
    pkg.owned_elements.push(ElementId::new("mem-id"));

    model.add_element(pkg);
    model.add_element(membership);

    let xmi = Xmi;
    let output = xmi.write(&model).expect("write");
    let output_str = String::from_utf8_lossy(&output);

    // Should use xsi:type on ownedRelationship
    assert!(
        output_str.contains(r#"<ownedRelationship xsi:type="#)
            || output_str.contains(r#"xsi:type="sysml:OwningMembership"#)
            || output_str.contains(r#"xsi:type="kerml:OwningMembership"#),
        "Expected xsi:type on ownedRelationship, got: {}",
        output_str
    );
}

/// Test that isComposite is written even when false.
#[test]
fn test_is_composite_written_when_false() {
    let mut model = Model::new();
    let mut elem = Element::new("test-id", ElementKind::AttributeUsage);
    elem.name = Some("attr".into());
    elem.properties
        .insert("isComposite".into(), PropertyValue::Boolean(false));
    model.add_element(elem);

    let xmi = Xmi;
    let output = xmi.write(&model).expect("write");
    let output_str = String::from_utf8_lossy(&output);

    assert!(
        output_str.contains(r#"isComposite="false""#),
        "isComposite should be written even when false"
    );
}

/// Test that perform/include/exhibit/assert use specialized usage elements plus
/// ReferenceSubsetting, while assume/require keep their current membership shape.
#[test]
fn test_special_usage_relationship_xmi_types_and_generic_targets() {
    let mut model = Model::new();

    let mut pkg = Element::new("pkg", ElementKind::Package);
    pkg.name = Some("Pkg".into());

    let mut perform_src =
        Element::new("perform-src", ElementKind::PerformActionUsage).with_owner("pkg");
    perform_src.name = Some("performSrc".into());
    let mut perform_tgt = Element::new("perform-tgt", ElementKind::ActionUsage).with_owner("pkg");
    perform_tgt.name = Some("performTgt".into());

    let mut exhibit_src =
        Element::new("exhibit-src", ElementKind::ExhibitStateUsage).with_owner("pkg");
    exhibit_src.name = Some("exhibitSrc".into());
    let mut exhibit_tgt = Element::new("exhibit-tgt", ElementKind::StateUsage).with_owner("pkg");
    exhibit_tgt.name = Some("exhibitTgt".into());

    let mut include_src =
        Element::new("include-src", ElementKind::IncludeUseCaseUsage).with_owner("pkg");
    include_src.name = Some("includeSrc".into());
    let mut include_tgt = Element::new("include-tgt", ElementKind::UseCaseUsage).with_owner("pkg");
    include_tgt.name = Some("includeTgt".into());

    let mut assert_src =
        Element::new("assert-src", ElementKind::AssertConstraintUsage).with_owner("pkg");
    assert_src.name = Some("assertSrc".into());
    let mut assert_tgt = Element::new("assert-tgt", ElementKind::ConstraintUsage).with_owner("pkg");
    assert_tgt.name = Some("assertTgt".into());

    let mut assume_src = Element::new("assume-src", ElementKind::ConstraintUsage).with_owner("pkg");
    assume_src.name = Some("assumeSrc".into());
    let mut assume_tgt = Element::new("assume-tgt", ElementKind::ConstraintUsage).with_owner("pkg");
    assume_tgt.name = Some("assumeTgt".into());

    let mut require_src = Element::new("require-src", ElementKind::ConstraintUsage).with_owner("pkg");
    require_src.name = Some("requireSrc".into());
    let mut require_tgt = Element::new("require-tgt", ElementKind::ConstraintUsage).with_owner("pkg");
    require_tgt.name = Some("requireTgt".into());

    let child_ids = [
        "perform-src",
        "perform-tgt",
        "exhibit-src",
        "exhibit-tgt",
        "include-src",
        "include-tgt",
        "assert-src",
        "assert-tgt",
        "assume-src",
        "assume-tgt",
        "require-src",
        "require-tgt",
    ];
    for child_id in child_ids {
        pkg.owned_elements.push(ElementId::new(child_id));
    }

    let perform_rel = Element::new_relationship(
        "perform-rel",
        ElementKind::ReferenceSubsetting,
        "perform-src",
        "perform-tgt",
    )
    .with_owner("perform-src");
    perform_src.owned_elements.push(ElementId::new("perform-rel"));

    let exhibit_rel = Element::new_relationship(
        "exhibit-rel",
        ElementKind::ReferenceSubsetting,
        "exhibit-src",
        "exhibit-tgt",
    )
    .with_owner("exhibit-src");
    exhibit_src.owned_elements.push(ElementId::new("exhibit-rel"));

    let include_rel = Element::new_relationship(
        "include-rel",
        ElementKind::ReferenceSubsetting,
        "include-src",
        "include-tgt",
    )
    .with_owner("include-src");
    include_src.owned_elements.push(ElementId::new("include-rel"));

    let assert_rel = Element::new_relationship(
        "assert-rel",
        ElementKind::ReferenceSubsetting,
        "assert-src",
        "assert-tgt",
    )
    .with_owner("assert-src");
    assert_src.owned_elements.push(ElementId::new("assert-rel"));

    let mut assume_rel = Element::new_relationship(
        "assume-rel",
        ElementKind::RequirementConstraintMembership,
        "assume-src",
        "assume-tgt",
    )
    .with_owner("assume-src");
    assume_rel
        .properties
        .insert("kind".into(), PropertyValue::String("assumption".into()));
    assume_src.owned_elements.push(ElementId::new("assume-rel"));

    let mut require_rel = Element::new_relationship(
        "require-rel",
        ElementKind::RequirementConstraintMembership,
        "require-src",
        "require-tgt",
    )
    .with_owner("require-src");
    require_rel
        .properties
        .insert("kind".into(), PropertyValue::String("requirement".into()));
    require_src.owned_elements.push(ElementId::new("require-rel"));

    model.add_element(pkg);
    model.add_element(perform_src);
    model.add_element(perform_tgt);
    model.add_element(exhibit_src);
    model.add_element(exhibit_tgt);
    model.add_element(include_src);
    model.add_element(include_tgt);
    model.add_element(assert_src);
    model.add_element(assert_tgt);
    model.add_element(assume_src);
    model.add_element(assume_tgt);
    model.add_element(require_src);
    model.add_element(require_tgt);
    model.add_element(perform_rel);
    model.add_element(exhibit_rel);
    model.add_element(include_rel);
    model.add_element(assert_rel);
    model.add_element(assume_rel);
    model.add_element(require_rel);

    let xmi = Xmi;
    let output = xmi.write(&model).expect("write");
    let output_str = String::from_utf8_lossy(&output);

    assert!(output_str.contains(r#"xsi:type="sysml:PerformActionUsage""#));
    assert!(output_str.contains(r#"xsi:type="kerml:ReferenceSubsetting""#));
    assert!(output_str.contains(r#"target="perform-tgt""#));
    assert!(output_str.contains(r#"xsi:type="sysml:ExhibitStateUsage""#));
    assert!(output_str.contains(r#"xsi:type="kerml:ReferenceSubsetting""#));
    assert!(output_str.contains(r#"target="exhibit-tgt""#));
    assert!(output_str.contains(r#"xsi:type="sysml:IncludeUseCaseUsage""#));
    assert!(output_str.contains(r#"xsi:type="kerml:ReferenceSubsetting""#));
    assert!(output_str.contains(r#"target="include-tgt""#));
    assert!(output_str.contains(r#"xsi:type="sysml:AssertConstraintUsage""#));
    assert!(output_str.contains(r#"xsi:type="kerml:ReferenceSubsetting""#));
    assert!(output_str.contains(r#"target="assert-tgt""#));
    assert!(output_str.contains(r#"xsi:type="sysml:RequirementConstraintMembership""#));
    assert!(output_str.contains(r#"target="assume-tgt""#));
    assert!(output_str.contains(r#"kind="assumption""#));
    assert!(output_str.contains(r#"kind="requirement""#));
    assert!(!output_str.contains("performedAction="));
    assert!(!output_str.contains("exhibitedState="));
    assert!(!output_str.contains("useCaseIncluded="));
    assert!(!output_str.contains("assertedConstraint="));
    assert!(!output_str.contains("referencedConstraint="));

    let roundtrip = xmi.read(&output).expect("read");

    let perform_rel_rt = roundtrip
        .get(&ElementId::new("perform-rel"))
        .expect("perform relationship should round-trip");
    assert_eq!(perform_rel_rt.kind, ElementKind::ReferenceSubsetting);
    assert_eq!(
        perform_rel_rt.relationship.as_ref().and_then(|rel| rel.target()),
        Some(&ElementId::new("perform-tgt"))
    );

    let assume_rel_rt = roundtrip
        .get(&ElementId::new("assume-rel"))
        .expect("assume relationship should round-trip");
    assert_eq!(assume_rel_rt.kind, ElementKind::RequirementConstraintMembership);
    assert_eq!(
        assume_rel_rt.properties.get("kind"),
        Some(&PropertyValue::String("assumption".into()))
    );
    assert_eq!(
        assume_rel_rt.relationship.as_ref().and_then(|rel| rel.target()),
        Some(&ElementId::new("assume-tgt"))
    );
}

/// Test roundtrip with official XMI file preserves format.
#[test]
fn test_roundtrip_preserves_key_attributes() {
    use std::process::Command;

    // Get test file
    let tmp_dir = std::env::temp_dir().join("syster-test-sysml-release");
    if !tmp_dir.exists() {
        let status = Command::new("git")
            .args([
                "clone",
                "--depth=1",
                "https://github.com/Systems-Modeling/SysML-v2-Release.git",
                tmp_dir.to_str().unwrap(),
            ])
            .status();
        if status.is_err() || !status.unwrap().success() {
            println!("Skipping test - could not clone repo");
            return;
        }
    }

    let test_file =
        tmp_dir.join("sysml.library.xmi/Domain Libraries/Quantities and Units/Quantities.sysmlx");
    if !test_file.exists() {
        println!("Skipping test - file not found");
        return;
    }

    let original = std::fs::read(&test_file).expect("read file");
    let original_str = String::from_utf8_lossy(&original);

    let xmi = Xmi;
    let model = xmi.read(&original).expect("parse");
    let output = xmi.write(&model).expect("write");
    let output_str = String::from_utf8_lossy(&output);

    // Check key format aspects are preserved
    if original_str.contains("declaredName=") {
        assert!(
            output_str.contains("declaredName="),
            "declaredName should be preserved"
        );
    }
    if original_str.contains("elementId=") {
        assert!(
            output_str.contains("elementId="),
            "elementId should be preserved"
        );
    }
    if original_str.contains(r#"encoding="ASCII""#) {
        assert!(
            output_str.contains(r#"encoding="ASCII""#),
            "ASCII encoding should be preserved"
        );
    }
}
