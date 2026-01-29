//! Test JSON-LD roundtrip with attribute preservation.

use std::sync::Arc;
use syster::interchange::JsonLd;
use syster::interchange::ModelFormat;
use syster::interchange::model::{Element, ElementId, ElementKind, Model, PropertyValue};

fn main() {
    println!("Testing JSON-LD roundtrip with attributes...\n");

    // Create a model with various attributes
    let mut model = Model::new();

    // Add an abstract package
    let mut pkg = Element::new("pkg-001", ElementKind::Package);
    pkg.name = Some(Arc::from("TestPackage"));
    pkg.short_name = Some(Arc::from("TP"));
    model.add_element(pkg);

    // Add an abstract classifier with various properties
    let mut cls = Element::new("cls-001", ElementKind::Class);
    cls.name = Some(Arc::from("TestClassifier"));
    cls.is_abstract = true;
    cls.documentation = Some(Arc::from("This is a documented classifier"));
    cls.properties
        .insert(Arc::from("isStandard"), PropertyValue::Boolean(true));
    cls.properties
        .insert(Arc::from("someNumber"), PropertyValue::Integer(42));
    cls.properties.insert(
        Arc::from("someString"),
        PropertyValue::String(Arc::from("hello")),
    );
    cls.owner = Some("pkg-001".into());
    model.add_element(cls);

    // Add a feature
    let mut feat = Element::new("feat-001", ElementKind::Feature);
    feat.name = Some(Arc::from("TestFeature"));
    feat.properties
        .insert(Arc::from("isComposite"), PropertyValue::Boolean(true));
    feat.owner = Some("cls-001".into());
    model.add_element(feat);

    println!("Original model: {} elements", model.element_count());

    // Write to JSON-LD
    let jsonld = JsonLd::default();
    let bytes = jsonld.write(&model).expect("Failed to export JSON-LD");
    let json_str = String::from_utf8_lossy(&bytes);
    println!("JSON-LD size: {} bytes", bytes.len());
    println!(
        "\n=== JSON-LD Output ===\n{}\n",
        &json_str[..json_str.len().min(2000)]
    );

    // Read back
    let model2: Model = jsonld.read(&bytes).expect("Failed to import JSON-LD");
    println!("Imported model: {} elements", model2.element_count());

    // Verify attributes
    let mut passed = 0;
    let mut failed = 0;

    // Check package
    let pkg2 = model2
        .get(&ElementId::new("pkg-001"))
        .expect("Package missing");
    check(
        "Package name",
        pkg2.name.as_deref() == Some("TestPackage"),
        &mut passed,
        &mut failed,
    );
    check(
        "Package shortName",
        pkg2.short_name.as_deref() == Some("TP"),
        &mut passed,
        &mut failed,
    );

    // Check classifier
    let cls2 = model2
        .get(&ElementId::new("cls-001"))
        .expect("Classifier missing");
    check(
        "Classifier name",
        cls2.name.as_deref() == Some("TestClassifier"),
        &mut passed,
        &mut failed,
    );
    check(
        "Classifier isAbstract",
        cls2.is_abstract,
        &mut passed,
        &mut failed,
    );
    check(
        "Classifier documentation",
        cls2.documentation.as_deref() == Some("This is a documented classifier"),
        &mut passed,
        &mut failed,
    );
    check(
        "Classifier isStandard property",
        cls2.properties.get(&Arc::from("isStandard")) == Some(&PropertyValue::Boolean(true)),
        &mut passed,
        &mut failed,
    );
    check(
        "Classifier someNumber property",
        cls2.properties.get(&Arc::from("someNumber")) == Some(&PropertyValue::Integer(42)),
        &mut passed,
        &mut failed,
    );
    check(
        "Classifier someString property",
        cls2.properties.get(&Arc::from("someString"))
            == Some(&PropertyValue::String(Arc::from("hello"))),
        &mut passed,
        &mut failed,
    );

    // Check feature
    let feat2 = model2
        .get(&ElementId::new("feat-001"))
        .expect("Feature missing");
    check(
        "Feature name",
        feat2.name.as_deref() == Some("TestFeature"),
        &mut passed,
        &mut failed,
    );
    check(
        "Feature isComposite property",
        feat2.properties.get(&Arc::from("isComposite")) == Some(&PropertyValue::Boolean(true)),
        &mut passed,
        &mut failed,
    );

    println!("\n=== Results ===");
    println!("Passed: {}", passed);
    println!("Failed: {}", failed);

    if failed == 0 {
        println!("\n✓ All JSON-LD roundtrip tests passed!");
    } else {
        println!("\n✗ Some tests failed!");
        std::process::exit(1);
    }
}

fn check(name: &str, condition: bool, passed: &mut i32, failed: &mut i32) {
    if condition {
        println!("  ✓ {}", name);
        *passed += 1;
    } else {
        println!("  ✗ {}", name);
        *failed += 1;
    }
}
