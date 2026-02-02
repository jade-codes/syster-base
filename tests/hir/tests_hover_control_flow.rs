//! Integration tests for hover on control flow constructs.
//!
//! Tests hover resolution for action refs in decision/merge, for loops,
//! assign statements, and succession statements.

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
fn hover_action_ref_in_if_then() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    action def DecisionAction {
        action A1;
        action A2;
        attribute x : ScalarValues::Integer;
        
        if x == 1 then A1;
        if x > 1 then A2;
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Line 7: `if x == 1 then A1;`
    let hover = analysis.hover(file_id, 7, 23);
    assert!(hover.is_some(), "Should hover on 'A1' in if-then");

    // Line 8: `if x > 1 then A2;`
    let hover = analysis.hover(file_id, 8, 22);
    assert!(hover.is_some(), "Should hover on 'A2' in if-then");
}

#[test]
fn hover_for_loop_variable() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    import ScalarValues::*;
    
    action def TestAction {
        attribute i : Integer = 1;
        
        for n : Integer in (1, 2, 3) {
            assign i := i * n;
        }
    }
}
"#;
    host.set_file_content("test.sysml", source);

    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Line 7: `for n : Integer in (1, 2, 3) {`
    let hover = analysis.hover(file_id, 7, 12);
    assert!(hover.is_some(), "Should hover on 'n' loop variable");
}

#[test]
fn hover_assign_statement_refs() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    import ScalarValues::*;
    
    action def TestAction {
        attribute i : Integer = 1;
        attribute n : Integer = 2;
        
        assign i := i * n;
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Line 8: `assign i := i * n;`
    let hover = analysis.hover(file_id, 8, 15);
    assert!(hover.is_some(), "Should hover on 'i' in assign target");

    let hover = analysis.hover(file_id, 8, 20);
    assert!(hover.is_some(), "Should hover on 'i' in assign expression");

    let hover = analysis.hover(file_id, 8, 24);
    assert!(hover.is_some(), "Should hover on 'n' in assign expression");
}

#[test]
fn hover_succession_refs() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    action def FlowAction {
        action step1;
        action step2;
        
        first step1 then step2;
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Line 6: `first step1 then step2;`
    let hover = analysis.hover(file_id, 6, 14);
    assert!(hover.is_some(), "Should hover on 'step1' in succession");

    let hover = analysis.hover(file_id, 6, 25);
    assert!(hover.is_some(), "Should hover on 'step2' in succession");
}

#[test]
fn hover_transition_refs() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    state def MyState {
        state idle;
        state active;
        
        transition idle then active;
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Line 6: `transition idle then active;`
    let hover = analysis.hover(file_id, 6, 19);
    assert!(hover.is_some(), "Should hover on 'idle' in transition");

    let hover = analysis.hover(file_id, 6, 29);
    assert!(hover.is_some(), "Should hover on 'active' in transition");
}
