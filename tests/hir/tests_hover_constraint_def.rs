//! Integration tests for hover on constraint def members.
//!
//! Tests hover resolution for attributes, parameters, and expression refs
//! inside constraint definitions.

use std::path::PathBuf;
use syster::ide::AnalysisHost;
use syster::project::StdLibLoader;

fn stdlib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sysml.library")
}

fn create_host_with_stdlib() -> AnalysisHost {
    let mut host = AnalysisHost::new();
    let stdlib = stdlib_path();
    if stdlib.exists() {
        let mut stdlib_loader = StdLibLoader::with_path(stdlib);
        let _ = stdlib_loader.ensure_loaded_into_host(&mut host);
    }
    host
}

#[test]
fn hover_attribute_name_in_constraint_def() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    import ISQ::MassValue;
    
    constraint def MassConstraint {
        attribute totalMass: MassValue;
    }
}
"#;
    host.set_file_content("test.sysml", source);

    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Line 5: `attribute totalMass: MassValue;`
    let hover = analysis.hover(file_id, 5, 18);

    assert!(
        hover.is_some(),
        "Should hover on 'totalMass' attribute name in constraint def"
    );
}

#[test]
fn hover_attribute_type_in_constraint_def() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    import ISQ::MassValue;
    
    constraint def MassConstraint {
        attribute totalMass: MassValue;
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Line 5: `attribute totalMass: MassValue;`
    let hover = analysis.hover(file_id, 5, 30);
    assert!(
        hover.is_some(),
        "Should hover on 'MassValue' type in constraint def attribute"
    );
}

#[test]
fn hover_attribute_with_local_type_in_constraint_def() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    attribute def MyValue;
    
    constraint def MyConstraint {
        attribute val: MyValue;
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Line 5: `attribute val: MyValue;`
    let hover = analysis.hover(file_id, 5, 23);
    assert!(
        hover.is_some(),
        "Should hover on 'MyValue' local type in constraint def"
    );
}

#[test]
fn hover_constraint_expression_ref() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    constraint def LimitConstraint {
        attribute max;
        attribute val;
        constraint { val <= max }
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Line 5: `constraint { val <= max }`
    // Hover on 'val'
    let hover = analysis.hover(file_id, 5, 21);
    assert!(
        hover.is_some(),
        "Should hover on 'val' in constraint expression"
    );
}

#[test]
fn hover_nested_constraint_def() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    part def Container {
        constraint def InnerConstraint {
            attribute x;
        }
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Line 4: `attribute x;`
    let hover = analysis.hover(file_id, 4, 22);
    assert!(
        hover.is_some(),
        "Should hover on 'x' in nested constraint def"
    );
}
