//! Integration tests for hover on metadata annotations.
//!
//! Tests hover resolution for metadata prefixes (#name), metadata refs,
//! and profile base type defaults.

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
fn hover_metadata_prefix_on_definition() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    metadata def service;
    metadata def clouddd;
    
    #service port def ServiceDiscovery {
    }
    
    #clouddd part def ArrowheadCore {
    }
}
"#;
    host.set_file_content("test.sysml", source);

    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Line 5: `#service port def ServiceDiscovery`
    let hover = analysis.hover(file_id, 5, 5);
    assert!(hover.is_some(), "Should hover on 'service' metadata prefix");

    // Line 8: `#clouddd part def ArrowheadCore`
    let hover = analysis.hover(file_id, 8, 5);
    assert!(hover.is_some(), "Should hover on 'clouddd' metadata prefix");
}

#[test]
fn hover_ref_redefines_annotated_element() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    metadata def ClassificationMetadata :> Metaobjects::SemanticMetadata {
        ref :>> annotatedElement : SysML::Usage;
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Line 3: `ref :>> annotatedElement : SysML::Usage;`
    let hover = analysis.hover(file_id, 3, 17);
    assert!(
        hover.is_some(),
        "Should hover on 'annotatedElement' redefines ref"
    );
}

#[test]
fn hover_ref_feature_declaration() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    enum def ClassificationLevel {
        unclassified;
        confidential;
    }
    
    metadata def ClassificationMetadata {
        ref classificationLevel : ClassificationLevel;
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Line 8: `ref classificationLevel : ClassificationLevel;`
    let hover = analysis.hover(file_id, 8, 12);
    assert!(
        hover.is_some(),
        "Should hover on 'classificationLevel' ref declaration"
    );
}

#[test]
fn hover_metadata_def_name() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    metadata def MyMeta;
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Line 2: `metadata def MyMeta;`
    let hover = analysis.hover(file_id, 2, 18);
    assert!(hover.is_some(), "Should hover on 'MyMeta' metadata def name");
}
