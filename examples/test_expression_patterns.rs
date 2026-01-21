//! Test for all expression reference patterns
//!
//! Tests various patterns to find which ones are missing expression ref extraction

use std::path::PathBuf;
use syster::core::Position;
use syster::semantic::Workspace;
use syster::syntax::file::SyntaxFile;
use syster::syntax::sysml::ast::parse_file;
use syster::parser::sysml::{SysMLParser, Rule};
use pest::Parser;

fn main() {
    let source = r#"package Test {
    // Base definitions
    attribute def MassValue;
    attribute def PowerValue;
    attribute def CostValue;
    part def Engine;
    
    // Usage with redefines and value
    part baseEngine : Engine {
        attribute mass : MassValue;
        attribute peakHorsePower : PowerValue;
        attribute cost : CostValue;
    }
    
    // Test various patterns
    part derivedEngine :> baseEngine {
        // Pattern 1: redefines with simple value
        attribute mass redefines mass = 180;
        
        // Pattern 2: redefines with expression referencing another attribute
        attribute cost redefines cost = mass * 10;
        
        // Pattern 3: simple assignment
        attribute simpleAttr = mass;
        
        // Pattern 4: chained reference  
        attribute chainedAttr = baseEngine.mass;
    }
}"#;
    
    println!("=== EXPRESSION REFERENCE PATTERNS TEST ===\n");
    println!("SOURCE:\n{}\n", source);
    
    let mut workspace: Workspace<SyntaxFile> = Workspace::new();
    let mut pairs = SysMLParser::parse(Rule::file, source).expect("Parse should succeed");
    let sysml_file = parse_file(&mut pairs).expect("AST parse should succeed");
    let syntax_file = SyntaxFile::SysML(sysml_file);
    workspace.add_file(PathBuf::from("/test.sysml"), syntax_file);
    workspace.populate_all().expect("Should populate");
    
    let ref_index = workspace.reference_index();
    
    println!("ALL TARGETS in reference index:");
    for target in ref_index.targets() {
        let refs = ref_index.get_references(target);
        for r in refs {
            println!("  line={:2} col={:2}-{:2} target={:<30} source={}", 
                r.span.start.line, r.span.start.column, r.span.end.column, 
                target, r.source_qname);
        }
    }
    
    println!("\n=== CHECKING HOVER AT SPECIFIC POSITIONS ===\n");
    
    // Line 17: attribute mass redefines mass = 180;
    // "mass" redefines target is at col 33-37
    println!("Line 17 col 33 - 'redefines mass':");
    let hover_target = ref_index.get_reference_at_position("/test.sysml", Position::new(17, 33));
    println!("  Hover target: {:?}", hover_target);
    
    // Line 20: attribute cost redefines cost = mass * 10;
    // "cost" redefines target is at col 33-37
    // "mass" expression ref is at col 40-44
    println!("\nLine 20 col 33 - 'redefines cost':");
    let hover_target = ref_index.get_reference_at_position("/test.sysml", Position::new(20, 33));
    println!("  Hover target: {:?}", hover_target);
    
    println!("\nLine 20 col 40 - 'mass' in expression:");
    let hover_target = ref_index.get_reference_at_position("/test.sysml", Position::new(20, 40));
    println!("  Hover target: {:?}", hover_target);
    
    // Line 23: attribute simpleAttr = mass;
    // "mass" is at col 31-35
    println!("\nLine 23 col 31 - 'mass' in simple assignment:");
    let hover_target = ref_index.get_reference_at_position("/test.sysml", Position::new(23, 31));
    println!("  Hover target: {:?}", hover_target);
    
    // Line 26: attribute chainedAttr = baseEngine.mass;
    // "baseEngine" is at col 32-42
    // "mass" is at col 43-47
    println!("\nLine 26 col 32 - 'baseEngine' in chained ref:");
    let hover_target = ref_index.get_reference_at_position("/test.sysml", Position::new(26, 32));
    println!("  Hover target: {:?}", hover_target);
    
    println!("\nLine 26 col 43 - 'mass' in chained ref:");
    let hover_target = ref_index.get_reference_at_position("/test.sysml", Position::new(26, 43));
    println!("  Hover target: {:?}", hover_target);
}
