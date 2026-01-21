use pest::Parser;
use syster::parser::sysml::{Rule, SysMLParser};

fn main() {
    let source = r#"use case transportPassenger_1:TransportPassenger{
    action driverGetInVehicle subsets getInVehicle_a[1];
    action driveVehicleToDestination;
    action providePower;
    item def VehicleOnSignal;
    join join1;
    first start;
    then fork fork1;
    first join1 then trigger;
}"#;

    println!("Parsing use case...");
    match SysMLParser::parse(Rule::use_case_usage, source) {
        Ok(pairs) => {
            println!("SUCCESS!");
            for pair in pairs {
                print_pair(&pair, 0);
            }
        }
        Err(e) => {
            println!("PARSE ERROR: {}", e);
        }
    }
}

fn print_pair(pair: &pest::iterators::Pair<Rule>, depth: usize) {
    let indent = "  ".repeat(depth);
    let span = pair.as_span();
    let text: String = span.as_str().chars().take(50).collect();
    println!("{}{:?} @ {}..{}: {:?}...", indent, pair.as_rule(), span.start(), span.end(), text);
    
    for inner in pair.clone().into_inner() {
        print_pair(&inner, depth + 1);
    }
}
