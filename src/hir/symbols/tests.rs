use super::*;
use crate::base::FileId;
use crate::parser::{Direction, Multiplicity};

#[test]
fn test_symbol_kind_display() {
    assert_eq!(SymbolKind::PartDefinition.display(), "Part def");
    assert_eq!(SymbolKind::PartUsage.display(), "Part");
}

#[test]
fn test_strip_quotes() {
    use super::context::strip_quotes;
    assert_eq!(strip_quotes("'hello'"), "hello");
    assert_eq!(strip_quotes("hello"), "hello");
    assert_eq!(strip_quotes("'"), "'");
}

#[test]
fn test_extraction_context() {
    use super::context::ExtractionContext;
    let mut ctx = ExtractionContext {
        file: FileId::new(0),
        prefix: String::new(),
        anon_counter: 0,
        scope_stack: Vec::new(),
        line_index: crate::base::LineIndex::new(""),
    };

    assert_eq!(ctx.qualified_name("Foo"), "Foo");

    ctx.push_scope("Outer");
    assert_eq!(ctx.qualified_name("Inner"), "Outer::Inner");

    ctx.push_scope("Deep");
    assert_eq!(ctx.qualified_name("Leaf"), "Outer::Deep::Leaf");

    ctx.pop_scope();
    assert_eq!(ctx.qualified_name("Sibling"), "Outer::Sibling");

    ctx.pop_scope();
    assert_eq!(ctx.qualified_name("Root"), "Root");
}

#[test]
fn test_direction_and_multiplicity_extraction() {
    use crate::syntax::parser::parse_content;

    let source = r#"part def Vehicle {
            in port fuelIn : FuelType[1];
            out port exhaust : GasType[0..*];
            inout port control : ControlType[1..5];
            part wheels[4];
        }"#;

    let syntax = parse_content(source, std::path::Path::new("test.sysml")).unwrap();
    let symbols = extract_symbols_unified(FileId::new(0), &syntax);

    // Find the ports and verify direction
    let fuel_in = symbols
        .iter()
        .find(|s| s.name.as_ref() == "fuelIn")
        .unwrap();
    assert_eq!(fuel_in.direction, Some(Direction::In));
    assert_eq!(
        fuel_in.multiplicity,
        Some(Multiplicity {
            lower: Some(1),
            upper: Some(1)
        })
    );

    let exhaust = symbols
        .iter()
        .find(|s| s.name.as_ref() == "exhaust")
        .unwrap();
    assert_eq!(exhaust.direction, Some(Direction::Out));
    assert_eq!(
        exhaust.multiplicity,
        Some(Multiplicity {
            lower: Some(0),
            upper: None
        })
    );

    let control = symbols
        .iter()
        .find(|s| s.name.as_ref() == "control")
        .unwrap();
    assert_eq!(control.direction, Some(Direction::InOut));
    assert_eq!(
        control.multiplicity,
        Some(Multiplicity {
            lower: Some(1),
            upper: Some(5)
        })
    );

    let wheels = symbols
        .iter()
        .find(|s| s.name.as_ref() == "wheels")
        .unwrap();
    assert_eq!(wheels.direction, None);
    assert_eq!(
        wheels.multiplicity,
        Some(Multiplicity {
            lower: Some(4),
            upper: Some(4)
        })
    );
}

#[cfg(test)]
mod test_package_span {
    use super::super::*;
    use crate::base::FileId;
    use crate::syntax::parser::parse_content;

    #[test]
    fn test_hir_simple_package_span() {
        let source = "package SimpleVehicleModel { }";
        let syntax = parse_content(source, std::path::Path::new("test.sysml")).unwrap();

        let symbols = extract_symbols_unified(FileId(1), &syntax);

        let pkg_sym = symbols
            .iter()
            .find(|s| s.name.as_ref() == "SimpleVehicleModel")
            .unwrap();

        println!(
            "Package symbol: name='{}' start=({},{}), end=({},{})",
            pkg_sym.name, pkg_sym.start_line, pkg_sym.start_col, pkg_sym.end_line, pkg_sym.end_col
        );

        // The name "SimpleVehicleModel" starts at column 8 (after "package ")
        assert_eq!(pkg_sym.start_col, 8, "start_col should be 8");
        assert_eq!(pkg_sym.end_col, 26, "end_col should be 26");
    }

    #[test]
    fn test_hir_nested_package_span() {
        // Match the structure of VehicleIndividuals.sysml
        let source = r#"package VehicleIndividuals {
	package IndividualDefinitions {
	}
}"#;
        let syntax = parse_content(source, std::path::Path::new("test.sysml")).unwrap();

        let symbols = extract_symbols_unified(FileId(1), &syntax);

        for sym in &symbols {
            println!(
                "Symbol: name='{}' kind={:?} start=({},{}), end=({},{})",
                sym.name, sym.kind, sym.start_line, sym.start_col, sym.end_line, sym.end_col
            );
        }

        // Top-level package: "VehicleIndividuals" starts at column 8
        let outer = symbols
            .iter()
            .find(|s| s.name.as_ref() == "VehicleIndividuals")
            .unwrap();
        assert_eq!(outer.start_col, 8, "outer start_col should be 8");
        assert_eq!(
            outer.end_col, 26,
            "outer end_col should be 26 (8 + 18 = 26)"
        );

        // Nested package: "IndividualDefinitions" on line 2, with a tab prefix
        // "package IndividualDefinitions" - tab is 1 char, "package " is 8 chars = 9
        let nested = symbols
            .iter()
            .find(|s| s.name.as_ref() == "IndividualDefinitions")
            .unwrap();
        println!(
            "Nested package: start_col={}, end_col={}",
            nested.start_col, nested.end_col
        );
        // After tab (1) and "package " (8) = 9
        assert_eq!(nested.start_col, 9, "nested start_col should be 9");
        // "IndividualDefinitions" is 21 chars, so 9 + 21 = 30
        assert_eq!(nested.end_col, 30, "nested end_col should be 30");
    }
}

// === Regression tests for false positive fixes ===

/// Regression: nested `attribute def` inside another def should be extractable.
/// Previously, shorthand redefines `:>> samples : TimeStateRecord` was anonymous,
/// attaching TypedBy refs to the parent and causing false "undefined reference" errors.
#[test]
fn test_shorthand_redefines_names_usage() {
    use crate::syntax::parser::parse_content;
    let source = r#"package P {
    attribute def NominalScenario {
        attribute def TimeStateRecord {
            t : Real;
        }
        :>> samples : TimeStateRecord;
    }
}"#;
    let syntax = parse_content(source, std::path::Path::new("test.sysml")).unwrap();
    let symbols = extract_symbols_unified(FileId::new(0), &syntax);
    // TimeStateRecord should be extracted
    assert!(symbols.iter().any(|s| s.name.as_ref() == "TimeStateRecord"));
    // The `:>> samples` shorthand should name the usage "samples"
    assert!(
        symbols.iter().any(|s| s.name.as_ref() == "samples"),
        "Shorthand redefines ':>> samples' should create a named usage 'samples'"
    );
}

/// Regression: `action aa accept S` should scope the payload child under `aa`.
/// Previously, the payload `S` got qname `...::a1::S` instead of `...::a1::aa::S`,
/// colliding with `accept S via x` and causing a false duplicate definition error.
#[test]
fn test_accept_action_payload_scoping() {
    use crate::syntax::parser::parse_content;
    let source = r#"package PartTest {
    item def S;
    abstract part def B {
        action a1 {
            accept S via x;
            action aa accept S;
        }
    }
}"#;
    let syntax = parse_content(source, std::path::Path::new("test.sysml")).unwrap();
    let symbols = extract_symbols_unified(FileId::new(0), &syntax);
    let s_symbols: Vec<_> = symbols.iter().filter(|s| s.name.as_ref() == "S").collect();
    // Should have exactly 1 ItemDefinition of S
    assert_eq!(
        s_symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::ItemDefinition)
            .count(),
        1
    );
    // The payload S from `action aa accept S` should be scoped under aa
    let aa_payload = s_symbols
        .iter()
        .find(|s| s.qualified_name.as_ref().contains("aa::S"));
    assert!(
        aa_payload.is_some(),
        "Payload S from 'action aa accept S' should have qname containing 'aa::S'"
    );
}
