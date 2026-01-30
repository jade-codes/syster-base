fn main() {
    let source = r#"package Test {
    use case transportPassenger:TransportPassenger{
        first start; 
        then action a{
            action driverGetInVehicle subsets getInVehicle_a[1];
            action passenger1GetInVehicle subsets getInVehicle_a[1];
        }
        then action trigger accept ignitionCmd:IgnitionCmd;
        then action b{
            action driveVehicleToDestination;
            action providePower;   
        }
        then action c{
            action driverGetOutOfVehicle subsets getOutOfVehicle_a[1];
            action passenger1GetOutOfVehicle subsets getOutOfVehicle_a[1];
        }
        then done;
    }
}"#;
    
    use syster::syntax::file::FileExtension;
    let syntax_file = syster::syntax::SyntaxFile::new(source, FileExtension::SysML);
    
    // Extract symbols
    let symbols = syster::hir::extract_symbols_unified(syster::FileId(0), &syntax_file);
    
    println!("=== EXTRACTED SYMBOLS ===");
    for sym in &symbols {
        println!("  {} ({:?})", sym.qualified_name, sym.kind);
    }
}
