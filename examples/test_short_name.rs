use std::path::PathBuf;
use syster::ide::AnalysisHost;
use syster::project::StdLibLoader;

fn main() {
    let stdlib = PathBuf::from("sysml.library");
    let mut host = AnalysisHost::new();
    
    // Load stdlib
    let loader = StdLibLoader::with_path(stdlib);
    loader.load_into_host(&mut host).expect("stdlib should load");
    
    // Snapshot for queries
    let analysis = host.analysis();
    let index = analysis.symbol_index();
    
    // Look for kg in simple name lookup
    let results = index.lookup_simple("kg");
    println!("lookup_simple('kg') found {} results:", results.len());
    for sym in &results {
        println!("  {} (short_name: {:?})", sym.qualified_name, sym.short_name);
    }
    
    // Check direct lookup by short_name index
    let by_short = index.lookup_by_short_name("kg");
    println!("\nlookup_by_short_name('kg') found {} results:", by_short.len());
    for sym in &by_short {
        println!("  {} (short_name: {:?})", sym.qualified_name, sym.short_name);
    }
    
    // Look for kilogram
    let kilo = index.lookup_simple("kilogram");
    println!("\nlookup_simple('kilogram') found {} results:", kilo.len());
    for sym in &kilo {
        println!("  {} (short_name: {:?})", sym.qualified_name, sym.short_name);
    }
}
