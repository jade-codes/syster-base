//! Temporary debug test for connect parsing
use syster::parser::parse_sysml;

#[test]
fn debug_connect_forms() {
    let input1 = "package Test {
    #multicausation connection : MultiCauseEffect connect
        ( cause1 ::> causer1, cause2 ::> causer2,
          effect1 ::> effected1, effect2 ::> effected2 );
}";

    let input2 = "package Test {
    #multicausation connect
        ( cause1 ::> causer1, cause2 ::> causer2,
          effect1 ::> effected1, effect2 ::> effected2 );
}";

    let parse1 = parse_sysml(input1);
    let parse2 = parse_sysml(input2);

    println!("\n=== FORM 1: connection : Type connect ===");
    println!("{:#?}", parse1.syntax());
    for err in &parse1.errors {
        println!("ERROR: {:?}", err);
    }

    println!("\n=== FORM 2: #multicausation connect ===");
    println!("{:#?}", parse2.syntax());
    for err in &parse2.errors {
        println!("ERROR: {:?}", err);
    }
}
