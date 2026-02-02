//! Bottom-up triage for OTHER hover failures
//! Line 893: status = StatusKind::closed;
//! Line 937: #refinement dependency engine4Cyl to ...

use syster::parser::{parse_sysml, ast::*, SyntaxNode, SyntaxKind};
use syster::syntax::normalized::{NormalizedElement, NormalizedRelKind};

/// Test: Enum member access StatusKind::closed
/// This is inside a metadata annotation block @StatusInfo { status = StatusKind::closed; }
#[test]
fn test_enum_member_access_bottom_up() {
    let source = r#"
package Test {
    enum def StatusKind {
        enum open;
        enum closed;
        enum tbd;
    }
    metadata def StatusInfo {
        attribute status : StatusKind;
    }
    part vehicle {
        @StatusInfo {
            status = StatusKind::closed;
        }
    }
}
"#;

    // LAYER 1: Parser
    println!("=== LAYER 1: Parser ===");
    let parsed = parse_sysml(source);
    println!("{:#?}", parsed.syntax());
    
    // Check that StatusKind::closed is parsed
    let source_text = parsed.syntax().text().to_string();
    assert!(source_text.contains("StatusKind::closed"), "Source should contain StatusKind::closed");
    
    println!("\n=== LAYER 1 PASS ===");
}

/// Test: #refinement prefix metadata on dependency
#[test]
fn test_refinement_prefix_metadata_bottom_up() {
    let source = r#"
package Test {
    metadata def refinement;
    part engine1;
    part engine2;
    #refinement dependency engine1 to engine2;
}
"#;

    // LAYER 1: Parser
    println!("=== LAYER 1: Parser ===");
    let parsed = parse_sysml(source);
    println!("{:#?}", parsed.syntax());
    
    // LAYER 2: AST - find the dependency and check prefix metadata
    println!("\n=== LAYER 2: AST ===");
    let root = SourceFile::cast(parsed.syntax()).unwrap();
    
    fn find_dependency(node: SyntaxNode) -> Option<Dependency> {
        if let Some(d) = Dependency::cast(node.clone()) {
            return Some(d);
        }
        for child in node.children() {
            if let Some(found) = find_dependency(child) {
                return Some(found);
            }
        }
        None
    }
    
    let dependency = find_dependency(root.syntax().clone());
    assert!(dependency.is_some(), "Should find dependency");
    let dependency = dependency.unwrap();
    
    // Check prefix metadata via the new method
    let prefix_metas = dependency.prefix_metadata();
    println!("dependency prefix_metadata count: {}", prefix_metas.len());
    for pm in &prefix_metas {
        println!("  prefix_metadata name: {:?}", pm.name());
    }
    assert!(!prefix_metas.is_empty(), "dependency should have prefix metadata");
    assert_eq!(prefix_metas[0].name().as_deref(), Some("refinement"), "prefix metadata should be 'refinement'");
    
    println!("\n=== LAYER 2 PASS - AST correctly finds prefix metadata ===");
    
    // LAYER 3: Extraction - check if NormalizedElement handles Dependency
    println!("\n=== LAYER 3: Extraction ===");
    if let Some(member) = NamespaceMember::cast(dependency.syntax().clone()) {
        let elem = NormalizedElement::from_rowan(&member);
        println!("Extracted element: {:?}", std::mem::discriminant(&elem));
        match &elem {
            NormalizedElement::Usage(nu) => {
                println!("Extracted as Usage, relationships:");
                for rel in &nu.relationships {
                    println!("  {:?} -> {}", rel.kind, rel.target.as_str());
                }
                let has_meta_refinement = nu.relationships.iter()
                    .any(|r| matches!(r.kind, NormalizedRelKind::Meta) && r.target.as_str() == "refinement");
                assert!(has_meta_refinement, "Should have Meta->refinement relationship");
            }
            NormalizedElement::Dependency(nd) => {
                println!("Extracted as Dependency, relationships:");
                for rel in &nd.relationships {
                    println!("  {:?} -> {}", rel.kind, rel.target.as_str());
                }
                let has_meta_refinement = nd.relationships.iter()
                    .any(|r| matches!(r.kind, NormalizedRelKind::Meta) && r.target.as_str() == "refinement");
                assert!(has_meta_refinement, "Should have Meta->refinement relationship");
            }
            _ => {
                panic!("Expected Usage or Dependency element, got {:?}", std::mem::discriminant(&elem));
            }
        }
    } else {
        panic!("Dependency should cast to NamespaceMember");
    }
    
    println!("\n=== All layers pass! ===");
}
