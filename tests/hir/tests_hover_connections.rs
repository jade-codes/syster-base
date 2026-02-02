//! Integration tests for hover on connection and interface endpoints.
//!
//! Tests hover resolution for endpoint refs in connections, interfaces,
//! and cause/effect patterns.

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
fn hover_connection_endpoint_name() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package CauseEffect {
    connection def MultiCauseEffect {
        end causer1;
        end causer2;
        end effected1;
    }
    
    part def System {
        part a;
        part b;
        part c;
        
        connection : MultiCauseEffect
            ( cause1 ::> a, cause2 ::> b, effect1 ::> c );
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Line 14: `( cause1 ::> a, cause2 ::> b, effect1 ::> c );`
    let hover = analysis.hover(file_id, 14, 14);
    assert!(hover.is_some(), "Should hover on 'cause1' endpoint name");

    let hover = analysis.hover(file_id, 14, 28);
    assert!(hover.is_some(), "Should hover on 'cause2' endpoint name");
}

#[test]
fn hover_interface_endpoint_part() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    port def PubPort;
    
    part def Producer {
        port publicationPort : PubPort;
    }
    
    part def Server {
        port publicationPort : PubPort;
    }
    
    part def System {
        part producer_2 : Producer;
        part server_2 : Server;
        
        interface producer_2.publicationPort to server_2.publicationPort;
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Line 16: `interface producer_2.publicationPort to server_2.publicationPort;`
    let hover = analysis.hover(file_id, 16, 48);
    assert!(
        hover.is_some(),
        "Should hover on 'server_2' in interface endpoint"
    );
}

#[test]
fn hover_interface_endpoint_port() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    port def PubPort;
    
    part def Producer {
        port publicationPort : PubPort;
    }
    
    part def Server {
        port publicationPort : PubPort;
    }
    
    part def System {
        part producer_2 : Producer;
        part server_2 : Server;
        
        interface producer_2.publicationPort to server_2.publicationPort;
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Line 16: `interface producer_2.publicationPort to server_2.publicationPort;`
    let hover = analysis.hover(file_id, 16, 58);
    assert!(
        hover.is_some(),
        "Should hover on 'publicationPort' chain member"
    );
}
