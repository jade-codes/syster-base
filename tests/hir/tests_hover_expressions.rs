//! Integration tests for hover on multiplicity and expression contexts.
//!
//! Tests hover resolution for variable refs in multiplicity bounds,
//! tuple expressions, and return statements.

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
fn hover_variable_in_multiplicity_single() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    part def Container {
        attribute n : ScalarValues::Integer;
        
        part e[n];
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Line 5: `part e[n];`
    let hover = analysis.hover(file_id, 5, 15);
    assert!(
        hover.is_some(),
        "Should hover on 'n' in multiplicity bounds"
    );
}

#[test]
fn hover_variable_in_multiplicity_lower_bound() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    part def Container {
        attribute n : ScalarValues::Integer;
        
        part f[n..*];
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Line 5: `part f[n..*];`
    let hover = analysis.hover(file_id, 5, 15);
    assert!(hover.is_some(), "Should hover on 'n' in lower bound");
}

#[test]
fn hover_variable_in_multiplicity_upper_bound() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    part def Container {
        attribute n : ScalarValues::Integer;
        
        part g[1..n];
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Line 5: `part g[1..n];`
    let hover = analysis.hover(file_id, 5, 18);
    assert!(hover.is_some(), "Should hover on 'n' in upper bound");
}

#[test]
fn hover_variable_in_attribute_multiplicity() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    part def Container {
        attribute i : ScalarValues::Integer;
        
        attribute x : ScalarValues::Integer[i];
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Line 5: `attribute x : ScalarValues::Integer[i];`
    let hover = analysis.hover(file_id, 5, 44);
    assert!(
        hover.is_some(),
        "Should hover on 'i' in attribute multiplicity"
    );
}

#[test]
fn hover_ref_in_tuple_expression() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    part def Engine4Cyl;
    part def Engine6Cyl;
    
    calc def SelectEngine {
        in engine4cyl : Engine4Cyl;
        in engine6cyl : Engine6Cyl;
        
        subject alternatives :> engine4cyl [2] = (engine4cyl, engine6cyl);
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Line 9: `        subject alternatives :> engine4cyl [2] = (engine4cyl, engine6cyl);`
    //          01234567890123456789012345678901234567890123456789012345678901234567890123
    //          0         1         2         3         4         5         6         7
    // engine4cyl in tuple: cols 50-59, engine6cyl in tuple: cols 62-71
    let hover = analysis.hover(file_id, 9, 50);
    assert!(hover.is_some(), "Should hover on 'engine4cyl' in tuple");

    let hover = analysis.hover(file_id, 9, 62);
    assert!(hover.is_some(), "Should hover on 'engine6cyl' in tuple");
}

#[test]
fn hover_return_ref_simple() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    part def Engine;
    
    calc def SelectEngine {
        in engine : Engine[2];
        
        return selectedEngine :> engine;
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Line 7: `return selectedEngine :> engine;`
    let hover = analysis.hover(file_id, 7, 15);
    assert!(
        hover.is_some(),
        "Should hover on 'selectedEngine' in return"
    );

    let hover = analysis.hover(file_id, 7, 33);
    assert!(hover.is_some(), "Should hover on 'engine' ref in return");
}
