fn main() {
    let sysml_cases = [
        "package P { attribute x = collection->exists {in x; x}; }",
        "package P { attribute x = seq2->forAll {in x; seq1->exists{in y; x == y}}; }",
        "package P { attribute x = exists a, b : a == b; }",
        "package P { attribute x = exists a : a == a; }",
    ];
    for src in sysml_cases {
        let parsed = syster::parser::parse_sysml(src);
        println!("--- {src}");
        println!("errors (sysml): {:?}\n", parsed.errors);
    }

    let kerml_cases = [
        "function exists { in x; return : Boolean[1]; }",
        "package P { private import ControlFunctions::exists; }",
    ];
    for src in kerml_cases {
        let parsed = syster::parser::parse_kerml(src);
        println!("--- {src}");
        println!("errors (kerml): {:?}\n", parsed.errors);
    }
}
