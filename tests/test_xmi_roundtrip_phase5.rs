//! Phase 5: End-to-end round-trip integrity tests.
//!
//! Tests the complete cycle:
//! - SysML → XMI → edit → XMI (element IDs stable)
//! - XMI → decompile → edit → recompile (IDs preserved)
//! - Workspace reload preserves IDs
//! - Import → edit → export preserves imported IDs

#[cfg(feature = "interchange")]
mod roundtrip_tests {
    use std::sync::Arc;
    use syster::base::FileId;
    use syster::hir::{FileText, RootDatabase, file_symbols_from_text};
    use syster::ide::AnalysisHost;
    use syster::interchange::{
        ModelFormat, Xmi, apply_metadata_to_host, decompile, model_from_symbols, symbols_from_model,
    };

    #[test]
    fn test_sysml_to_xmi_multiple_export_cycles() {
        // Test: SysML → symbols → XMI → symbols → XMI → symbols → XMI
        // Element IDs should be stable across all exports

        let db = RootDatabase::new();
        let sysml = r#"
package Vehicle {
    part def Engine;
    part def Car;
}
"#;
        let file_text = FileText::new(&db, FileId::new(1), sysml.to_string());

        // First cycle: SysML → symbols → XMI
        let symbols_v1 = file_symbols_from_text(&db, file_text);
        let model_v1 = model_from_symbols(&symbols_v1);
        let xmi_v1 = Xmi.write(&model_v1).expect("Should write XMI");

        // Second cycle: XMI → symbols → XMI
        let model_v2 = Xmi.read(&xmi_v1).expect("Should read XMI");
        let symbols_v2 = symbols_from_model(&model_v2);
        let model_v2_rebuilt = model_from_symbols(&symbols_v2);
        let xmi_v2 = Xmi.write(&model_v2_rebuilt).expect("Should write XMI");

        // Third cycle: XMI → symbols → XMI
        let model_v3 = Xmi.read(&xmi_v2).expect("Should read XMI");
        let symbols_v3 = symbols_from_model(&model_v3);
        let model_v3_rebuilt = model_from_symbols(&symbols_v3);
        let xmi_v3 = Xmi.write(&model_v3_rebuilt).expect("Should write XMI");

        // Element IDs should be identical across all cycles
        assert_eq!(
            symbols_v1.len(),
            symbols_v2.len(),
            "Symbol count should match"
        );
        assert_eq!(
            symbols_v2.len(),
            symbols_v3.len(),
            "Symbol count should match"
        );

        for i in 0..symbols_v1.len() {
            let s1 = &symbols_v1[i];
            let s2 = &symbols_v2[i];
            let s3 = &symbols_v3[i];

            assert_eq!(
                s1.qualified_name, s2.qualified_name,
                "Qualified names should match across cycles"
            );
            assert_eq!(
                s2.qualified_name, s3.qualified_name,
                "Qualified names should match across cycles"
            );

            // Element IDs should be stable
            assert_eq!(
                s1.element_id, s2.element_id,
                "Element ID for {} should be stable after round-trip",
                s1.qualified_name
            );
            assert_eq!(
                s2.element_id, s3.element_id,
                "Element ID for {} should be stable after multiple round-trips",
                s1.qualified_name
            );
        }

        // XMI output should be identical (except whitespace/formatting)
        // At minimum, element IDs should appear in the same order
        let xmi_v1_str = String::from_utf8_lossy(&xmi_v1);
        let xmi_v3_str = String::from_utf8_lossy(&xmi_v3);

        // Extract element IDs from both XMIs and compare
        for symbol in &symbols_v1 {
            let id = symbol.element_id.as_ref();
            assert!(xmi_v1_str.contains(id), "First XMI should contain {}", id);
            assert!(xmi_v3_str.contains(id), "Third XMI should contain {}", id);
        }
    }

    #[test]
    fn test_xmi_decompile_edit_recompile_preserves_ids() {
        // Test: XMI → decompile → edit SysML → parse → export XMI
        // Original element IDs should be preserved

        // Start with SysML and export to XMI
        let db = RootDatabase::new();
        let original_sysml = r#"
package MyModel {
    part def Component;
}
"#;
        let file_text = FileText::new(&db, FileId::new(1), original_sysml.to_string());
        let original_symbols = file_symbols_from_text(&db, file_text);
        let original_model = model_from_symbols(&original_symbols);
        let xmi_bytes = Xmi.write(&original_model).expect("Should write XMI");

        // Decompile XMI to SysML + metadata
        let model = Xmi.read(&xmi_bytes).expect("Should read XMI");
        let decompile_result = decompile(&model);

        // Load decompiled SysML into new workspace
        let mut host = AnalysisHost::new();
        host.set_file_content("/model.sysml", &decompile_result.text);

        // Apply metadata to restore element IDs
        apply_metadata_to_host(&mut host, &decompile_result.metadata);

        // Edit the SysML (add a new element)
        let edited_sysml = r#"
package MyModel {
    part def Component;
    part def NewComponent;
}
"#;
        host.set_file_content("/model.sysml", edited_sysml);

        // Export to XMI again
        let analysis = host.analysis();
        let all_symbols_refs: Vec<_> = analysis.symbol_index().all_symbols().collect();
        let all_symbols: Vec<_> = all_symbols_refs.iter().map(|s| (*s).clone()).collect();
        let new_model = model_from_symbols(&all_symbols);
        let new_xmi_bytes = Xmi.write(&new_model).expect("Should write new XMI");

        // Original elements should have same IDs
        for orig_symbol in &original_symbols {
            let found = all_symbols
                .iter()
                .find(|s| s.qualified_name == orig_symbol.qualified_name);

            if let Some(found) = found {
                assert_eq!(
                    found.element_id, orig_symbol.element_id,
                    "Element ID for {} should be preserved after edit",
                    orig_symbol.qualified_name
                );
            }
        }

        // New element should have a different ID
        let new_component = all_symbols
            .iter()
            .find(|s| s.name.as_ref() == "NewComponent");
        assert!(new_component.is_some(), "Should have new component");

        let new_id = &new_component.unwrap().element_id;
        for orig_symbol in &original_symbols {
            assert_ne!(
                new_id, &orig_symbol.element_id,
                "New element should have different ID from originals"
            );
        }
    }

    #[test]
    fn test_workspace_reload_preserves_ids() {
        // Test: Load workspace → export XMI → reload workspace → export XMI
        // Element IDs should be identical

        let sysml = r#"
package System {
    part def Controller;
    part def Sensor;
}
"#;

        // First load
        let mut host1 = AnalysisHost::new();
        host1.set_file_content("/system.sysml", sysml);

        let analysis1 = host1.analysis();
        let symbols1_refs: Vec<_> = analysis1.symbol_index().all_symbols().collect();
        let symbols1: Vec<_> = symbols1_refs.iter().map(|s| (*s).clone()).collect();
        let model1 = model_from_symbols(&symbols1);
        let xmi1 = Xmi.write(&model1).expect("Should write XMI");

        // Extract element IDs from first export
        let element_ids1: Vec<Arc<str>> = symbols1.iter().map(|s| s.element_id.clone()).collect();

        // Decompile to get metadata
        let decompile_result = decompile(&model1);

        // Second load (simulating workspace reload)
        let mut host2 = AnalysisHost::new();
        host2.set_file_content("/system.sysml", sysml);

        // Apply metadata to restore IDs
        apply_metadata_to_host(&mut host2, &decompile_result.metadata);

        let analysis2 = host2.analysis();
        let symbols2_refs: Vec<_> = analysis2.symbol_index().all_symbols().collect();
        let symbols2: Vec<_> = symbols2_refs.iter().map(|s| (*s).clone()).collect();
        let model2 = model_from_symbols(&symbols2);
        let xmi2 = Xmi.write(&model2).expect("Should write XMI");

        // Element IDs should match (sort both by qualified_name for comparison)
        assert_eq!(symbols1.len(), symbols2.len(), "Symbol count should match");

        let mut s1_sorted = symbols1.clone();
        let mut s2_sorted = symbols2.clone();
        s1_sorted.sort_by(|a, b| a.qualified_name.cmp(&b.qualified_name));
        s2_sorted.sort_by(|a, b| a.qualified_name.cmp(&b.qualified_name));

        for (s1, s2) in s1_sorted.iter().zip(s2_sorted.iter()) {
            assert_eq!(
                s1.qualified_name, s2.qualified_name,
                "Qualified names should match"
            );
            assert_eq!(
                s1.element_id, s2.element_id,
                "Element ID for {} should survive workspace reload",
                s1.qualified_name
            );
        }

        // XMI should be equivalent
        let xmi1_str = String::from_utf8_lossy(&xmi1);
        let xmi2_str = String::from_utf8_lossy(&xmi2);

        for id in &element_ids1 {
            assert!(
                xmi1_str.contains(id.as_ref()),
                "First XMI should contain {}",
                id
            );
            assert!(
                xmi2_str.contains(id.as_ref()),
                "Second XMI should contain {}",
                id
            );
        }
    }

    #[test]
    fn test_xmi_import_edit_export_preserves_imported_ids() {
        // Test: Import XMI → modify workspace → export XMI
        // Imported element IDs preserved, new elements get new IDs

        // Create an XMI model
        let db = RootDatabase::new();
        let sysml = r#"
package ImportedModel {
    part def OriginalPart;
}
"#;
        let file_text = FileText::new(&db, FileId::new(1), sysml.to_string());
        let original_symbols = file_symbols_from_text(&db, file_text);
        let original_model = model_from_symbols(&original_symbols);
        let xmi_bytes = Xmi.write(&original_model).expect("Should write XMI");

        // Import into workspace
        let imported_model = Xmi.read(&xmi_bytes).expect("Should read XMI");

        let mut host = AnalysisHost::new();
        host.add_model(&imported_model, "imported.sysml");

        // Add new SysML file with additional elements
        let new_sysml = r#"
package NewModel {
    part def NewPart;
}
"#;
        host.set_file_content("/new.sysml", new_sysml);

        // Export combined model
        let analysis = host.analysis();
        let all_symbols_refs: Vec<_> = analysis.symbol_index().all_symbols().collect();
        let all_symbols: Vec<_> = all_symbols_refs.iter().map(|s| (*s).clone()).collect();
        let combined_model = model_from_symbols(&all_symbols);
        let combined_xmi = Xmi
            .write(&combined_model)
            .expect("Should write combined XMI");

        // Verify imported IDs are preserved (check against original model's elements)
        for element in imported_model.iter_elements() {
            if let Some(ref name) = element.name {
                let found = all_symbols
                    .iter()
                    .find(|s| s.name.as_ref() == name.as_ref());

                assert!(found.is_some(), "Imported symbol {} should exist", name);
                assert_eq!(
                    found.unwrap().element_id.as_ref(),
                    element.id.as_str(),
                    "Imported element ID for {} should be preserved",
                    name
                );
            }
        }

        // New elements should have different IDs
        let new_part = all_symbols.iter().find(|s| s.name.as_ref() == "NewPart");

        if let Some(new_part) = new_part {
            for element in imported_model.iter_elements() {
                assert_ne!(
                    new_part.element_id.as_ref(),
                    element.id.as_str(),
                    "New element should have different ID from imported elements"
                );
            }
        }

        // Combined XMI should contain all element IDs
        let combined_xmi_str = String::from_utf8_lossy(&combined_xmi);
        for element in imported_model.iter_elements() {
            assert!(
                combined_xmi_str.contains(element.id.as_str()),
                "Combined XMI should contain imported ID {}",
                element.id
            );
        }
    }

    #[test]
    fn test_full_round_trip_file_workflow() {
        // Test complete workflow: SysML file → XMI export → decompile → reload → re-export
        // Simulates real user workflow
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();

        // Step 1: Create original SysML file
        let original_sysml = r#"
package RealWorld {
    part def Robot {
        part actuator : Actuator;
    }
    part def Actuator;
}
"#;
        let sysml_path = temp_dir.path().join("robot.sysml");
        fs::write(&sysml_path, original_sysml).unwrap();

        // Step 2: Parse and export to XMI
        let mut host1 = AnalysisHost::new();
        host1.set_file_content(sysml_path.to_str().unwrap(), original_sysml);

        let analysis1 = host1.analysis();
        let symbols1_refs: Vec<_> = analysis1.symbol_index().all_symbols().collect();
        let symbols1: Vec<_> = symbols1_refs.iter().map(|s| (*s).clone()).collect();
        let model1 = model_from_symbols(&symbols1);
        let xmi_bytes = Xmi.write(&model1).expect("Should export to XMI");

        let xmi_path = temp_dir.path().join("robot.xmi");
        fs::write(&xmi_path, &xmi_bytes).unwrap();

        // Step 3: Decompile XMI to SysML + metadata
        let model_imported = Xmi.read(&xmi_bytes).expect("Should import XMI");
        let decompile_result = decompile(&model_imported);

        let sysml_path2 = temp_dir.path().join("robot_decompiled.sysml");
        let metadata_path = temp_dir.path().join("robot_decompiled.metadata.json");

        fs::write(&sysml_path2, &decompile_result.text).unwrap();
        decompile_result
            .metadata
            .write_to_file(&metadata_path)
            .unwrap();

        // Step 4: Reload from decompiled files
        let mut host2 = AnalysisHost::new();
        host2.set_file_content(sysml_path2.to_str().unwrap(), &decompile_result.text);

        // Apply metadata
        apply_metadata_to_host(&mut host2, &decompile_result.metadata);

        // Step 5: Re-export to XMI
        let analysis2 = host2.analysis();
        let symbols2_refs: Vec<_> = analysis2.symbol_index().all_symbols().collect();
        let symbols2: Vec<_> = symbols2_refs.iter().map(|s| (*s).clone()).collect();
        let model2 = model_from_symbols(&symbols2);
        let xmi_bytes2 = Xmi.write(&model2).expect("Should re-export to XMI");

        // Verify element IDs are preserved (sort by qualified_name for comparison)
        assert_eq!(
            symbols1.len(),
            symbols2.len(),
            "Should have same number of symbols after full round-trip"
        );

        let mut s1_sorted = symbols1.clone();
        let mut s2_sorted = symbols2.clone();
        s1_sorted.sort_by(|a, b| a.qualified_name.cmp(&b.qualified_name));
        s2_sorted.sort_by(|a, b| a.qualified_name.cmp(&b.qualified_name));

        for (s1, s2) in s1_sorted.iter().zip(s2_sorted.iter()) {
            assert_eq!(
                s1.qualified_name, s2.qualified_name,
                "Qualified name should match: {} vs {}",
                s1.qualified_name, s2.qualified_name
            );
            assert_eq!(
                s1.element_id, s2.element_id,
                "Element ID for {} should survive full round-trip workflow",
                s1.qualified_name
            );
        }

        // XMI files should contain same element IDs
        let xmi1_str = String::from_utf8_lossy(&xmi_bytes);
        let xmi2_str = String::from_utf8_lossy(&xmi_bytes2);

        for symbol in &symbols1 {
            let id = symbol.element_id.as_ref();
            assert!(xmi1_str.contains(id), "Original XMI should contain {}", id);
            assert!(
                xmi2_str.contains(id),
                "Re-exported XMI should contain {}",
                id
            );
        }
    }
}
