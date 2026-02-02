//! Test for transition reference extraction
//!
//! Checks that transition source/target references and feature chains
//! are being extracted properly at the symbol level.

use std::path::Path;
use syster::base::FileId;
use syster::hir::extract_symbols_unified;
use syster::syntax::parser::parse_content;

fn get_all_ref_targets(source: &str) -> Vec<String> {
    let parse = parse_content(source, Path::new("test.sysml")).unwrap();
    let syntax = parse;
    let symbols = extract_symbols_unified(FileId::new(0), &syntax);

    let mut targets = Vec::new();
    for sym in &symbols {
        for tr in &sym.type_refs {
            for r in tr.as_refs() {
                targets.push(r.target.to_string());
            }
        }
    }
    targets
}

#[test]
fn test_transition_targets_in_symbols() {
    let source = r#"
        state def VehicleState {
            state off;
            state on;
            transition initial then off;
            transition off_to_on first off then on;
        }
    "#;

    let targets = get_all_ref_targets(source);

    assert!(
        targets.iter().any(|t| t == "initial"),
        "Should have 'initial' as target"
    );
    assert!(
        targets.iter().any(|t| t == "off"),
        "Should have 'off' as target"
    );
    assert!(
        targets.iter().any(|t| t == "on"),
        "Should have 'on' as target"
    );
}

#[test]
fn test_perform_chain_in_symbols() {
    let source = r#"
        action def ProvidePower {
            action distributeTorque;
            action generateTorque;
        }
        part vehicle {
            perform providePower.distributeTorque;
        }
    "#;

    let targets = get_all_ref_targets(source);

    let has_chain = targets
        .iter()
        .any(|t| t.contains("distributeTorque") || t.contains("providePower"));
    assert!(
        has_chain,
        "Should have feature chain reference to providePower.distributeTorque"
    );
}

#[test]
fn test_constraint_has_name_in_symbols() {
    let source = r#"
        part def FuelTank {
            attribute fuel;
            assert constraint fuelConstraint { fuel > 0 }
        }
    "#;

    let parse = parse_content(source, Path::new("test.sysml")).unwrap();
    let syntax = parse;
    let symbols = extract_symbols_unified(FileId::new(0), &syntax);

    let has_constraint = symbols
        .iter()
        .any(|s| s.qualified_name.contains("fuelConstraint"));

    assert!(has_constraint, "Should have symbol for 'fuelConstraint'");
}
