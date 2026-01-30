use syster::ide::AnalysisHost;

fn main() {
    let source = "package P {
    part def FuelTank {
        item fuel { attribute fuelMass = 0; }
    }
    part fuelTank : FuelTank {
        ref item redefines fuel {
            attribute redefines fuelMass = 50;
        }
    }
}";
    
    let mut host = AnalysisHost::new();
    let _errors = host.set_file_content("test.sysml", source);
    
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").expect("file not found");
    let index = analysis.symbol_index();
    
    println!("=== SYMBOLS ===");
    for sym in index.symbols_in_file(file_id) {
        println!("{} ({:?})", sym.qualified_name, sym.kind);
        println!("  supertypes: {:?}", sym.supertypes);
        for (i, tr) in sym.type_refs.iter().enumerate() {
            println!("  type_ref[{}]: {:?}", i, tr);
        }
    }
}
