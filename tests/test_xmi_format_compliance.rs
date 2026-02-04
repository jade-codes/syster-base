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
        output_str.contains(r#"<ownedRelationship xsi:type="#) || 
        output_str.contains(r#"xsi:type="sysml:OwningMembership"#) ||
        output_str.contains(r#"xsi:type="kerml:OwningMembership"#),
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
    elem.properties.insert("isComposite".into(), PropertyValue::Boolean(false));
    model.add_element(elem);

    let xmi = Xmi;
    let output = xmi.write(&model).expect("write");
    let output_str = String::from_utf8_lossy(&output);

    assert!(
        output_str.contains(r#"isComposite="false""#),
        "isComposite should be written even when false"
    );
}

/// Test roundtrip with official XMI file preserves format.
#[test]
fn test_roundtrip_preserves_key_attributes() {
    use std::path::PathBuf;
    use std::process::Command;
    
    // Get test file
    let tmp_dir = std::env::temp_dir().join("syster-test-sysml-release");
    if !tmp_dir.exists() {
        let status = Command::new("git")
            .args([
                "clone", "--depth=1",
                "https://github.com/Systems-Modeling/SysML-v2-Release.git",
                tmp_dir.to_str().unwrap(),
            ])
            .status();
        if status.is_err() || !status.unwrap().success() {
            println!("Skipping test - could not clone repo");
            return;
        }
    }
    
    let test_file = tmp_dir.join("sysml.library.xmi/Domain Libraries/Quantities and Units/Quantities.sysmlx");
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
        assert!(output_str.contains("declaredName="), "declaredName should be preserved");
    }
    if original_str.contains("elementId=") {
        assert!(output_str.contains("elementId="), "elementId should be preserved");
    }
    if original_str.contains(r#"encoding="ASCII""#) {
        assert!(output_str.contains(r#"encoding="ASCII""#), "ASCII encoding should be preserved");
    }
}
