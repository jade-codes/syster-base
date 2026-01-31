use syster::syntax::file::FileExtension;

fn main() {
    let code = r#"
package Test {
    use case def TransportPassenger;
    use case transportPassenger_1 : TransportPassenger {
        action driverGetInVehicle;
        join join1;
        first start;
        then fork fork1;
            then driverGetInVehicle;
        first driverGetInVehicle then join1;
        first join1 then done;
    }
}
    "#;
    
    let syntax_file = syster::syntax::SyntaxFile::new(code, FileExtension::SysML);
    
    // Extract symbols
    let symbols = syster::hir::extract_symbols_unified(syster::FileId(0), &syntax_file);
    
    // Print all symbols
    println!("=== All Symbols ===");
    for sym in &symbols {
        println!("  {} : {:?}", sym.qualified_name, sym.kind);
        for trk in &sym.type_refs {
            match trk {
                syster::hir::TypeRefKind::Simple(tr) => {
                    println!("    TypeRef: '{}' ({:?}) -> {:?}", tr.target, tr.kind, tr.resolved_target);
                }
                syster::hir::TypeRefKind::Chain(chain) => {
                    for part in &chain.parts {
                        println!("    Chain Part: '{}' ({:?}) -> {:?}", part.target, part.kind, part.resolved_target);
                    }
                }
            }
        }
        // Also print relationships
        if !sym.relationships.is_empty() {
            println!("    Relationships:");
            for rel in &sym.relationships {
                println!("      {:?}: {}", rel.kind, rel.target);
            }
        }
    }
}
