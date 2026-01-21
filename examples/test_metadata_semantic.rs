//! Semantic test for metadata annotation references
//!
//! Tests that StatusKind::closed inside @StatusInfo { } is indexed as a reference

use std::path::PathBuf;
use syster::semantic::Workspace;
use syster::syntax::file::SyntaxFile;
use syster::syntax::sysml::ast::parse_file;
use syster::parser::sysml::{SysMLParser, Rule};
use pest::Parser;

fn main() {
    let source = r#"package ModelingMetadata {
    enum def StatusKind {
        enum open;
        enum closed;
    }
    
    metadata def StatusInfo {
        attribute status : StatusKind;
    }
}

package Test {
    import ModelingMetadata::*;
    
    part myPart {
        @StatusInfo {
            status = StatusKind::closed;
        }
    }
}"#;

    println!("=== SEMANTIC TEST: metadata annotation ===\n");
    println!("SOURCE:\n{}\n", source);

    let mut workspace: Workspace<SyntaxFile> = Workspace::new();
    let mut pairs = SysMLParser::parse(Rule::file, source).expect("Parse should succeed");
    let sysml_file = parse_file(&mut pairs).expect("AST parse should succeed");
    let syntax_file = SyntaxFile::SysML(sysml_file);
    workspace.add_file(PathBuf::from("/test.sysml"), syntax_file);
    workspace.populate_all().expect("Should populate");

    println!("SYMBOLS:");
    for sym in workspace.symbol_table().iter_symbols() {
        println!("  {} at {:?}", sym.qualified_name(), sym.span());
    }

    // Print all references with more detail
    println!("\nALL REFERENCES:");
    let refs = workspace.reference_index().get_references_in_file("/test.sysml");
    for r in &refs {
        println!("  line={} source={} span={:?} chain_context={:?}", 
            r.span.start.line, r.source_qname, r.span, r.chain_context);
    }

    // Check if there's a reference on line 16 that comes from status usage
    // The reference to StatusKind::closed should be indexed from the metadata_body_usage "status"
    println!("\nLooking for reference on line 16 from status usage...");
    let found = refs.iter().any(|r| {
        r.span.start.line == 16 && r.source_qname.contains("status")
    });
    println!("Found reference on line 16: {}", found);
}
