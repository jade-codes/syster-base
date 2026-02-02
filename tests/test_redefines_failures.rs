//! Bottom-up triage for REDEFINES hover failures
//! Line 653: requirement torqueGenerationRequirement :>> torqueGenerationRequirement
//! Line 656: requirement drivePowerOuputRequirement :>> drivePowerOutputRequirement

use syster::parser::{parse_sysml, ast::*, SyntaxNode};
use syster::syntax::normalized::{NormalizedElement, NormalizedRelKind};
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

/// Test: Redefines where name matches target
/// `requirement torqueGenerationRequirement :>> torqueGenerationRequirement`
/// The second `torqueGenerationRequirement` (after :>>) should resolve to the parent's member
#[test]
fn test_redefines_same_name_bottom_up() {
    let source = r#"
package Test {
    requirement def TorqueReq {
        subject torque;
    }
    requirement group {
        requirement torqueGenerationRequirement : TorqueReq;
    }
    satisfy group {
        requirement torqueGenerationRequirement :>> torqueGenerationRequirement {
            subject torque redefines torque;
        }
    }
}
"#;

    // LAYER 1: Parser
    println!("=== LAYER 1: Parser ===");
    let parsed = parse_sysml(source);
    println!("{:#?}", parsed.syntax());
    
    // LAYER 2: AST - find the inner requirement with :>>
    println!("\n=== LAYER 2: AST ===");
    let root = SourceFile::cast(parsed.syntax()).unwrap();
    
    fn find_redefining_req(node: SyntaxNode) -> Option<Usage> {
        if let Some(u) = Usage::cast(node.clone()) {
            // Check if it has a :>> specialization
            let specs: Vec<_> = u.specializations().collect();
            for spec in &specs {
                if spec.kind() == Some(SpecializationKind::Redefines) {
                    let name = u.name().and_then(|n| n.text());
                    if name.as_deref() == Some("torqueGenerationRequirement") {
                        return Some(u);
                    }
                }
            }
        }
        for child in node.children() {
            if let Some(found) = find_redefining_req(child) {
                return Some(found);
            }
        }
        None
    }
    
    let req = find_redefining_req(root.syntax().clone());
    assert!(req.is_some(), "Should find requirement with :>>");
    let req = req.unwrap();
    
    println!("Found requirement: {:?}", req.name().and_then(|n| n.text()));
    let specs: Vec<_> = req.specializations().collect();
    println!("Specializations: {}", specs.len());
    for spec in &specs {
        println!("  kind={:?} target={:?}", spec.kind(), spec.target().map(|t| t.to_string()));
    }
    
    // LAYER 3: Extraction
    println!("\n=== LAYER 3: Extraction ===");
    if let Some(member) = NamespaceMember::cast(req.syntax().clone()) {
        let elem = NormalizedElement::from_rowan(&member);
        if let NormalizedElement::Usage(nu) = &elem {
            println!("Extracted relationships:");
            for rel in &nu.relationships {
                println!("  {:?} -> {} range={:?}", rel.kind, rel.target.as_str(), rel.range);
            }
            let has_redefines = nu.relationships.iter()
                .any(|r| matches!(r.kind, NormalizedRelKind::Redefines));
            assert!(has_redefines, "Should have Redefines relationship");
        }
    }
    
    println!("\n=== LAYER 3 PASS ===");
    
    // LAYER 3b: Extraction of the SATISFY BLOCK itself
    println!("\n=== LAYER 3b: Satisfy Block Extraction ===");
    fn find_satisfy_block(node: SyntaxNode) -> Option<Usage> {
        if let Some(u) = Usage::cast(node.clone()) {
            // Check if it has requirement_verification (satisfy/verify)
            if u.requirement_verification().is_some() {
                return Some(u);
            }
        }
        for child in node.children() {
            if let Some(found) = find_satisfy_block(child) {
                return Some(found);
            }
        }
        None
    }
    
    let satisfy_usage = find_satisfy_block(root.syntax().clone());
    assert!(satisfy_usage.is_some(), "Should find satisfy block");
    let satisfy_usage = satisfy_usage.unwrap();
    
    println!("Satisfy block AST:");
    println!("  requirement_verification: {:?}", satisfy_usage.requirement_verification().map(|rv| rv.requirement().map(|t| t.to_string())));
    println!("  name: {:?}", satisfy_usage.name().and_then(|n| n.text()));
    println!("  children count: {}", satisfy_usage.syntax().children().count());
    
    // Extract the satisfy block as NormalizedElement
    if let Some(member) = NamespaceMember::cast(satisfy_usage.syntax().clone()) {
        let elem = NormalizedElement::from_rowan(&member);
        if let NormalizedElement::Usage(nu) = &elem {
            println!("\nSatisfy block normalized:");
            println!("  name: {:?}", nu.name);
            println!("  kind: {:?}", nu.kind);
            println!("  relationships:");
            for rel in &nu.relationships {
                println!("    {:?} -> {} range={:?}", rel.kind, rel.target.as_str(), rel.range);
            }
            println!("  children count: {}", nu.children.len());
            
            // Log children
            for (i, child) in nu.children.iter().enumerate() {
                println!("\n  Child {}:", i);
                match child {
                    NormalizedElement::Usage(cu) => {
                        println!("    kind: Usage({:?})", cu.kind);
                        println!("    name: {:?}", cu.name);
                        println!("    relationships:");
                        for rel in &cu.relationships {
                            println!("      {:?} -> {} range={:?}", rel.kind, rel.target.as_str(), rel.range);
                        }
                    }
                    NormalizedElement::Definition(cd) => {
                        println!("    kind: Definition({:?})", cd.kind);
                        println!("    name: {:?}", cd.name);
                    }
                    _ => println!("    kind: {:?}", child),
                }
            }
        }
    }
    
    // LAYER 4: Symbol extraction with full resolution
    println!("\n=== LAYER 4: Symbol Extraction ===");
    let mut host = AnalysisHost::new();
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();
    
    println!("All symbols:");
    for sym in analysis.symbol_index().symbols_in_file(file_id) {
        println!("  L{}: {} kind={:?}", sym.start_line, sym.qualified_name, sym.kind);
        if !sym.supertypes.is_empty() {
            println!("    supertypes: {:?}", sym.supertypes);
        }
        if !sym.type_refs.is_empty() {
            println!("    type_refs:");
            for tr in &sym.type_refs {
                println!("      {:?}", tr);
            }
        }
    }
    
    // Check that the nested requirement has the redefines type_ref
    let nested_req_sym = analysis.symbol_index().symbols_in_file(file_id)
        .into_iter()
        .find(|s| s.qualified_name.contains("satisfy") && s.qualified_name.contains("torqueGenerationRequirement"));
    assert!(nested_req_sym.is_some(), "Should have nested requirement symbol in satisfy block");
    let nested_req_sym = nested_req_sym.unwrap();
    
    // The type_refs should include the redefines target
    let redefines_ref = nested_req_sym.type_refs.iter().find(|tr| {
        match tr {
            syster::hir::TypeRefKind::Simple(r) => r.kind == syster::hir::RefKind::Redefines,
            _ => false,
        }
    });
    assert!(redefines_ref.is_some(), "Nested requirement should have Redefines type_ref");
    
    // Now check the resolution - it should resolve to the PARENT's torqueGenerationRequirement, 
    // not to itself!
    if let Some(syster::hir::TypeRefKind::Simple(r)) = redefines_ref {
        println!("\nRedefines resolution check:");
        println!("  target: {}", r.target);
        println!("  resolved_target: {:?}", r.resolved_target);
        
        // The redefines target should resolve to Test::group::torqueGenerationRequirement
        // NOT to Test::<satisfy:group#1@L8>::torqueGenerationRequirement (itself)
        assert!(
            r.resolved_target.as_ref().map_or(false, |rt| rt.as_ref() == "Test::group::torqueGenerationRequirement"),
            "Redefines should resolve to parent's member, not to itself. Got: {:?}",
            r.resolved_target
        );
    }
}

/// Test: Hover on redefines target in real file context
#[test]
fn test_redefines_hover_line_653() {
    let file_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/sysml-examples/Vehicle Example/SysML v2 Spec Annex A SimpleVehicleModel.sysml");
    
    let content = std::fs::read_to_string(&file_path).expect("Failed to read file");
    let path_str = file_path.to_string_lossy().to_string();

    let mut host = create_host_with_stdlib();
    let _parse_errors = host.set_file_content(&path_str, &content);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id(&path_str).expect("File not in index");

    let target_line = 652u32; // 0-indexed (line 653 in editor)
    
    let lines: Vec<&str> = content.lines().collect();
    println!("Line 653: {}", lines[target_line as usize]);
    
    // Find the column for the second "torqueGenerationRequirement" (after :>>)
    let line = lines[target_line as usize];
    // Line: "requirement torqueGenerationRequirement :>> torqueGenerationRequirement{"
    // First occurrence at ~24, second at ~56
    let first_pos = line.find("torqueGenerationRequirement").unwrap();
    let after_first = first_pos + "torqueGenerationRequirement".len();
    let second_pos = line[after_first..].find("torqueGenerationRequirement").map(|p| p + after_first);
    
    println!("First 'torqueGenerationRequirement' at col {}", first_pos);
    if let Some(pos) = second_pos {
        println!("Second 'torqueGenerationRequirement' at col {}", pos);
        
        // Check what symbols exist at this line
        println!("\nSymbols near line {}:", target_line);
        for sym in analysis.symbol_index().symbols_in_file(file_id) {
            if sym.start_line >= target_line.saturating_sub(3) && sym.start_line <= target_line + 5 {
                println!("  L{}: {} kind={:?}", sym.start_line, sym.qualified_name, sym.kind);
                if !sym.type_refs.is_empty() {
                    println!("    type_refs:");
                    for tr in &sym.type_refs {
                        println!("      {:?}", tr);
                    }
                }
            }
        }
        
        // Also search for symbols containing "torqueGenerationRequirement"
        println!("\nSymbols containing 'torqueGenerationRequirement':");
        for sym in analysis.symbol_index().all_symbols() {
            if sym.qualified_name.contains("torqueGenerationRequirement") {
                println!("  L{}: {} kind={:?}", sym.start_line, sym.qualified_name, sym.kind);
                println!("    type_refs:");
                for tr in &sym.type_refs {
                    println!("      {:?}", tr);
                }
            }
        }
        
        // Check the satisfy block symbol
        println!("\nSatisfy block symbol details:");
        for sym in analysis.symbol_index().symbols_in_file(file_id) {
            if sym.qualified_name.contains("satisfy") && sym.start_line == 651 {
                println!("  {}", sym.qualified_name);
                println!("  span: L{}:{} - L{}:{}", sym.start_line, sym.start_col, sym.end_line, sym.end_col);
            }
        }
        
        // Check for children of the satisfy block (look for symbols that start with the satisfy block QN)
        println!("\nChildren of satisfy block:");
        for sym in analysis.symbol_index().symbols_in_file(file_id) {
            if sym.qualified_name.contains("<satisfy:Requirements::engineSpecification") {
                println!("  L{}: {} kind={:?}", sym.start_line, sym.qualified_name, sym.kind);
                if !sym.type_refs.is_empty() {
                    println!("    type_refs:");
                    for tr in &sym.type_refs {
                        println!("      {:?}", tr);
                    }
                }
            }
        }
        
        // Test hover on the second occurrence (the redefines target)
        let col = pos as u32 + 5; // somewhere in the middle of the identifier
        println!("\nTesting hover at line {} col {}:", target_line, col);
        
        if let Some(hover) = analysis.hover(file_id, target_line, col) {
            println!("  HOVER FOUND: {:?}", hover.qualified_name);
            assert!(hover.qualified_name.is_some(), "Should have qualified name");
        } else {
            println!("  NO HOVER - this is the bug!");
            // This assertion should FAIL until we fix the bug
            panic!("Hover should work on redefines target");
        }
    } else {
        panic!("Could not find second occurrence of torqueGenerationRequirement");
    }
}
