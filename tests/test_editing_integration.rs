//! Integration tests for the model-first (ChangeTracker) edit flow.
//!
//! These tests exercise the full pipeline:
//!   SysML text → parse → Model → ChangeTracker mutations → render → verify
//!   SysML text → parse → Model → ChangeTracker mutations → XMI export → verify
//!
//! This complements the text-first (parse) roundtrip tests which never
//! touch ChangeTracker.

#[cfg(feature = "interchange")]
mod editing_integration {
    use syster::base::FileId;
    use syster::hir::{FileText, RootDatabase, file_symbols_from_text};
    use syster::interchange::{
        Element, ElementId, ElementKind, Model, ModelFormat, ModelHost, Xmi, decompile,
        model_from_symbols,
    };

    // ── Helpers ─────────────────────────────────────────────────────

    /// Parse SysML text into a ModelHost (integration-test equivalent
    /// of the crate-private `ModelHost::from_text`).
    fn host(source: &str) -> ModelHost {
        let db = RootDatabase::new();
        let ft = FileText::new(&db, FileId::new(0), source.to_string());
        let symbols = file_symbols_from_text(&db, ft);
        assert!(!symbols.is_empty(), "source should produce symbols");
        let model = model_from_symbols(&symbols);
        ModelHost::from_model(model)
    }

    /// Export model to XMI bytes and return the string representation.
    fn xmi_string(model: &Model) -> String {
        let bytes = Xmi.write(model).expect("XMI export should succeed");
        String::from_utf8(bytes).expect("XMI should be valid UTF-8")
    }

    // ── Tests ───────────────────────────────────────────────────────

    #[test]
    fn edit_rename_then_render() {
        let mut h = host("package P { part def Vehicle; }");
        let mut t = h.tracker();

        let v_id = h.find_by_name("Vehicle")[0].id().clone();
        t.rename(h.model_mut(), &v_id, "Car");

        let rendered = h.render();
        assert!(
            rendered.contains("Car"),
            "Rendered text should contain renamed element 'Car':\n{rendered}"
        );
        assert!(
            !rendered.contains("Vehicle"),
            "Rendered text should NOT contain old name 'Vehicle':\n{rendered}"
        );
    }

    #[test]
    fn edit_rename_then_xmi_export() {
        let mut h = host("package P { part def Vehicle; }");
        let mut t = h.tracker();

        let v_id = h.find_by_name("Vehicle")[0].id().clone();
        t.rename(h.model_mut(), &v_id, "Car");

        let xmi = xmi_string(h.model());
        assert!(
            xmi.contains("Car"),
            "XMI should contain renamed element 'Car':\n{xmi}"
        );
        assert!(
            !xmi.contains("Vehicle"),
            "XMI should NOT contain old name 'Vehicle':\n{xmi}"
        );
    }

    #[test]
    fn edit_add_element_then_render() {
        let mut h = host("package P;");
        let mut t = h.tracker();

        let p_id = h.find_by_name("P")[0].id().clone();
        let new_el = Element::new("new1", ElementKind::PartDefinition).with_name("Widget");
        t.add_element(h.model_mut(), new_el, Some(&p_id));

        let rendered = h.render();
        assert!(
            rendered.contains("Widget"),
            "Rendered text should contain added element 'Widget':\n{rendered}"
        );
        assert!(
            rendered.contains("part def Widget"),
            "Should render as 'part def Widget':\n{rendered}"
        );
    }

    #[test]
    fn edit_add_element_then_xmi_export() {
        let mut h = host("package P;");
        let mut t = h.tracker();

        let p_id = h.find_by_name("P")[0].id().clone();
        let new_el = Element::new("new1", ElementKind::PartDefinition).with_name("Widget");
        let new_id = t.add_element(h.model_mut(), new_el, Some(&p_id));

        let xmi = xmi_string(h.model());
        assert!(
            xmi.contains("Widget"),
            "XMI should contain added element 'Widget':\n{xmi}"
        );
        assert!(
            xmi.contains(new_id.as_str()),
            "XMI should contain the new element's ID '{}':\n{xmi}",
            new_id.as_str()
        );
    }

    #[test]
    fn edit_remove_element_then_render() {
        let mut h = host("package P { part def A; part def B; }");
        let mut t = h.tracker();

        let a_id = h.find_by_name("A")[0].id().clone();
        t.remove_element(h.model_mut(), &a_id);

        let rendered = h.render();
        assert!(
            !rendered.contains("part def A"),
            "Rendered text should NOT contain removed element 'A':\n{rendered}"
        );
        assert!(
            rendered.contains("part def B"),
            "Rendered text should still contain 'B':\n{rendered}"
        );
    }

    #[test]
    fn edit_remove_element_then_xmi_export() {
        let mut h = host("package P { part def A; part def B; }");
        let mut t = h.tracker();

        let a_id = h.find_by_name("A")[0].id().clone();
        t.remove_element(h.model_mut(), &a_id);

        let xmi = xmi_string(h.model());
        // A should be gone from XMI
        assert!(
            !xmi.contains(">A<") && !xmi.contains("\"A\""),
            "XMI should NOT contain removed element 'A'"
        );
        // B should still be present
        assert!(xmi.contains("B"), "XMI should still contain 'B':\n{xmi}");
    }

    #[test]
    fn edit_reparent_then_render() {
        let mut h = host("package A { part def X; } package B;");
        let mut t = h.tracker();

        let x_id = h.find_by_name("X")[0].id().clone();
        let b_id = h.find_by_name("B")[0].id().clone();

        t.reparent(h.model_mut(), &x_id, &b_id);

        let rendered = h.render();
        // X should appear inside B, not A
        // Simple heuristic: "B" block should contain "X"
        let b_pos = rendered.find("package B").expect("should find 'package B'");
        let x_pos = rendered
            .find("part def X")
            .expect("should find 'part def X'");
        assert!(
            x_pos > b_pos,
            "X should appear after B (i.e. inside B's block) in rendered text:\n{rendered}"
        );
    }

    #[test]
    fn edit_reparent_then_xmi_export() {
        let mut h = host("package A { part def X; } package B;");
        let mut t = h.tracker();

        let x_id = h.find_by_name("X")[0].id().clone();
        let b_id = h.find_by_name("B")[0].id().clone();

        t.reparent(h.model_mut(), &x_id, &b_id);

        let xmi = xmi_string(h.model());
        // Both A, B and X should still be in XMI (moved, not removed)
        assert!(xmi.contains("A"), "XMI should still contain 'A'");
        assert!(xmi.contains("B"), "XMI should still contain 'B'");
        assert!(xmi.contains("X"), "XMI should still contain 'X'");
    }

    #[test]
    fn edit_add_relationship_then_xmi_export() {
        let mut h = host("package P { part def Base; part def Derived; }");
        let mut t = h.tracker();

        let base_id = h.find_by_name("Base")[0].id().clone();
        let derived_id = h.find_by_name("Derived")[0].id().clone();

        t.add_relationship(
            h.model_mut(),
            ElementId::generate(),
            ElementKind::Specialization,
            derived_id.clone(),
            base_id.clone(),
        );

        let xmi = xmi_string(h.model());
        // XMI should reference both Base and Derived
        assert!(xmi.contains("Base"), "XMI should contain 'Base':\n{xmi}");
        assert!(
            xmi.contains("Derived"),
            "XMI should contain 'Derived':\n{xmi}"
        );
    }

    #[test]
    fn edit_set_documentation_then_render() {
        let mut h = host("package P { part def A; }");
        let mut t = h.tracker();

        let a_id = h.find_by_name("A")[0].id().clone();
        t.set_documentation(h.model_mut(), &a_id, Some("This is part A"));

        let rendered = h.render();
        assert!(
            rendered.contains("This is part A"),
            "Rendered text should contain documentation:\n{rendered}"
        );
    }

    #[test]
    fn edit_set_abstract_then_render() {
        let mut h = host("package P { part def Vehicle; }");
        let mut t = h.tracker();

        let v_id = h.find_by_name("Vehicle")[0].id().clone();
        t.set_abstract(h.model_mut(), &v_id, true);

        let rendered = h.render();
        assert!(
            rendered.contains("abstract"),
            "Rendered text should contain 'abstract' keyword:\n{rendered}"
        );
    }

    // ── Compound edit workflows ─────────────────────────────────────

    #[test]
    fn compound_edit_add_rename_export_xmi() {
        // Add a new element, rename an existing one, then export to XMI
        let mut h = host("package Models { part def OldName; }");
        let mut t = h.tracker();

        // Rename existing element
        let old_id = h.find_by_name("OldName")[0].id().clone();
        t.rename(h.model_mut(), &old_id, "NewName");

        // Add new element
        let models_id = h.find_by_name("Models")[0].id().clone();
        let new_el = Element::new("new-el-1", ElementKind::PartDefinition).with_name("Added");
        t.add_element(h.model_mut(), new_el, Some(&models_id));

        let xmi = xmi_string(h.model());
        assert!(
            xmi.contains("NewName"),
            "XMI should contain renamed element"
        );
        assert!(xmi.contains("Added"), "XMI should contain added element");
        assert!(!xmi.contains("OldName"), "XMI should NOT contain old name");
    }

    #[test]
    fn compound_edit_add_remove_render() {
        // Add one element, remove another, verify the rendered text
        let mut h = host("package P { part def Keep; part def Remove; }");
        let mut t = h.tracker();

        let p_id = h.find_by_name("P")[0].id().clone();
        let remove_id = h.find_by_name("Remove")[0].id().clone();

        // Remove one
        t.remove_element(h.model_mut(), &remove_id);

        // Add one
        let new_el = Element::new("added-1", ElementKind::PartDefinition).with_name("Fresh");
        t.add_element(h.model_mut(), new_el, Some(&p_id));

        let rendered = h.render();
        assert!(rendered.contains("Keep"), "Should still have 'Keep'");
        assert!(rendered.contains("Fresh"), "Should have 'Fresh'");
        assert!(
            !rendered.contains("Remove"),
            "Should NOT have 'Remove':\n{rendered}"
        );
    }

    // ── XMI round-trip through ChangeTracker ────────────────────────

    #[test]
    fn edit_then_xmi_roundtrip_preserves_ids() {
        // Parse → edit via ChangeTracker → export XMI → re-import XMI
        // Verify that element IDs survive the cycle.
        let mut h = host("package P { part def Engine; part def Wheel; }");
        let mut t = h.tracker();

        // Rename Engine → Motor
        let engine_id = h.find_by_name("Engine")[0].id().clone();
        t.rename(h.model_mut(), &engine_id, "Motor");

        // Add a new element
        let p_id = h.find_by_name("P")[0].id().clone();
        let new_el = Element::new("chassis-1", ElementKind::PartDefinition).with_name("Chassis");
        let chassis_id = t.add_element(h.model_mut(), new_el, Some(&p_id));

        // Export to XMI
        let xmi_bytes = Xmi.write(h.model()).expect("XMI export should succeed");

        // Re-import
        let reimported = Xmi.read(&xmi_bytes).expect("XMI import should succeed");

        // Verify that the renamed element kept its ID
        let motor_el = reimported
            .iter_elements()
            .find(|e| e.name.as_deref() == Some("Motor"));
        assert!(
            motor_el.is_some(),
            "Should find 'Motor' in reimported model"
        );
        assert_eq!(
            motor_el.unwrap().id.as_str(),
            engine_id.as_str(),
            "Renamed element should preserve its original ID"
        );

        // Verify the new element is present with its ID
        let chassis_el = reimported
            .iter_elements()
            .find(|e| e.name.as_deref() == Some("Chassis"));
        assert!(
            chassis_el.is_some(),
            "Should find 'Chassis' in reimported model"
        );
        assert_eq!(
            chassis_el.unwrap().id.as_str(),
            chassis_id.as_str(),
            "Added element should preserve its ID through XMI round-trip"
        );

        // Wheel should still be present and untouched
        let wheel_el = reimported
            .iter_elements()
            .find(|e| e.name.as_deref() == Some("Wheel"));
        assert!(wheel_el.is_some(), "Wheel should still exist");
    }

    #[test]
    fn edit_reparent_then_xmi_roundtrip() {
        // Reparent an element, export to XMI, re-import, verify structure
        let mut h = host("package Src { part def X; } package Dst;");
        let mut t = h.tracker();

        let x_id = h.find_by_name("X")[0].id().clone();
        let dst_id = h.find_by_name("Dst")[0].id().clone();

        t.reparent(h.model_mut(), &x_id, &dst_id);

        // Export and reimport
        let xmi_bytes = Xmi.write(h.model()).expect("export");
        let reimported = Xmi.read(&xmi_bytes).expect("import");

        // X should exist and its owner chain should lead to Dst
        let x_el = reimported
            .iter_elements()
            .find(|e| e.name.as_deref() == Some("X"));
        assert!(x_el.is_some(), "X should survive reparent + XMI round-trip");

        // Walk up to find Dst as logical parent
        let x_el = x_el.unwrap();
        let found_dst = walk_to_named_ancestor(&reimported, &x_el.id, "Dst");
        assert!(
            found_dst,
            "X should be a (possibly indirect) child of Dst after reparent"
        );
    }

    #[test]
    fn edit_remove_then_xmi_roundtrip() {
        // Remove one element, export to XMI, re-import, verify it's gone
        let mut h = host("package P { part def Stay; part def Gone; }");
        let mut t = h.tracker();

        let gone_id = h.find_by_name("Gone")[0].id().clone();
        t.remove_element(h.model_mut(), &gone_id);

        let xmi_bytes = Xmi.write(h.model()).expect("export");
        let reimported = Xmi.read(&xmi_bytes).expect("import");

        let stay = reimported
            .iter_elements()
            .find(|e| e.name.as_deref() == Some("Stay"));
        assert!(stay.is_some(), "'Stay' should still exist after round-trip");

        let gone = reimported
            .iter_elements()
            .find(|e| e.name.as_deref() == Some("Gone"));
        assert!(
            gone.is_none(),
            "'Gone' should NOT exist after remove + round-trip"
        );
    }

    // ── Edit → decompile → re-parse cycle ───────────────────────────

    #[test]
    fn edit_then_decompile_then_reparse() {
        // Model-first edit → decompile to SysML text → re-parse → verify
        // This bridges the two flows: ChangeTracker output feeds back
        // into the text-first pipeline.
        let mut h = host("package Root { part def Alpha; part def Beta; }");
        let mut t = h.tracker();

        // Rename Alpha → Gamma
        let alpha_id = h.find_by_name("Alpha")[0].id().clone();
        t.rename(h.model_mut(), &alpha_id, "Gamma");

        // Add Delta
        let root_id = h.find_by_name("Root")[0].id().clone();
        let new_el = Element::new("delta-1", ElementKind::PartDefinition).with_name("Delta");
        t.add_element(h.model_mut(), new_el, Some(&root_id));

        // Decompile the modified model to SysML text
        let decompiled = decompile(h.model());
        let text = &decompiled.text;

        // Verify the text
        assert!(
            text.contains("Gamma"),
            "Decompiled text should have 'Gamma'"
        );
        assert!(text.contains("Beta"), "Decompiled text should have 'Beta'");
        assert!(
            text.contains("Delta"),
            "Decompiled text should have 'Delta'"
        );
        assert!(!text.contains("Alpha"), "Should NOT have 'Alpha'");

        // Re-parse the decompiled text into a fresh model
        let h2 = host(&text);
        assert!(
            h2.find_by_name("Gamma").len() == 1,
            "Re-parsed model should find 'Gamma'"
        );
        assert!(
            h2.find_by_name("Beta").len() == 1,
            "Re-parsed model should find 'Beta'"
        );
        assert!(
            h2.find_by_name("Delta").len() == 1,
            "Re-parsed model should find 'Delta'"
        );
        assert!(
            h2.find_by_name("Alpha").is_empty(),
            "Re-parsed model should NOT find 'Alpha'"
        );
    }

    // ── Deeper model: nested structures ─────────────────────────────

    #[test]
    fn edit_nested_model_add_usage_then_render() {
        let mut h = host("package System {\n    part def Controller;\n    part def Sensor;\n}");
        let mut t = h.tracker();

        let ctrl_id = h.find_by_name("Controller")[0].id().clone();
        let new_usage = Element::new("s1", ElementKind::PartUsage).with_name("mainSensor");
        t.add_element(h.model_mut(), new_usage, Some(&ctrl_id));

        let rendered = h.render();
        assert!(
            rendered.contains("mainSensor"),
            "Rendered text should contain the new usage 'mainSensor':\n{rendered}"
        );
    }

    #[test]
    fn edit_multiple_operations_then_full_xmi_roundtrip() {
        // Complex scenario: multiple edits then XMI round-trip
        let mut h =
            host("package Fleet {\n    part def Truck;\n    part def Van;\n    part def Bus;\n}");
        let mut t = h.tracker();

        let fleet_id = h.find_by_name("Fleet")[0].id().clone();
        let truck_id = h.find_by_name("Truck")[0].id().clone();
        let van_id = h.find_by_name("Van")[0].id().clone();
        let bus_id = h.find_by_name("Bus")[0].id().clone();

        // 1. Rename Truck → HeavyTruck
        t.rename(h.model_mut(), &truck_id, "HeavyTruck");

        // 2. Remove Van
        t.remove_element(h.model_mut(), &van_id);

        // 3. Add Bike
        let bike = Element::new("bike-1", ElementKind::PartDefinition).with_name("Bike");
        let bike_id = t.add_element(h.model_mut(), bike, Some(&fleet_id));

        // 4. Set Bus as abstract
        t.set_abstract(h.model_mut(), &bus_id, true);

        // 5. Add doc to Bus
        t.set_documentation(h.model_mut(), &bus_id, Some("Public transport"));

        // Export to XMI → reimport
        let xmi_bytes = Xmi.write(h.model()).expect("export");
        let reimported = Xmi.read(&xmi_bytes).expect("import");

        // Verify all operations survived
        let names: Vec<String> = reimported
            .iter_elements()
            .filter_map(|e| e.name.as_ref().map(|n| n.to_string()))
            .collect();

        assert!(
            names.contains(&"HeavyTruck".to_string()),
            "HeavyTruck should exist"
        );
        assert!(
            !names.contains(&"Truck".to_string()),
            "Truck (old name) should be gone"
        );
        assert!(!names.contains(&"Van".to_string()), "Van should be removed");
        assert!(names.contains(&"Bus".to_string()), "Bus should exist");
        assert!(names.contains(&"Bike".to_string()), "Bike should exist");
        assert!(names.contains(&"Fleet".to_string()), "Fleet should exist");

        // Verify Bus is abstract
        let bus = reimported
            .iter_elements()
            .find(|e| e.name.as_deref() == Some("Bus"));
        assert!(bus.is_some());
        assert!(bus.unwrap().is_abstract, "Bus should be abstract");

        // Verify Bike has its ID
        let bike_el = reimported
            .iter_elements()
            .find(|e| e.name.as_deref() == Some("Bike"));
        assert!(bike_el.is_some());
        assert_eq!(bike_el.unwrap().id.as_str(), bike_id.as_str());

        // Verify HeavyTruck preserved original Truck ID
        let ht = reimported
            .iter_elements()
            .find(|e| e.name.as_deref() == Some("HeavyTruck"));
        assert!(ht.is_some());
        assert_eq!(ht.unwrap().id.as_str(), truck_id.as_str());
    }

    // ── Helper ──────────────────────────────────────────────────────

    /// Walk up the owner chain from `start_id` looking for an element named `target_name`.
    fn walk_to_named_ancestor(model: &Model, start_id: &ElementId, target_name: &str) -> bool {
        let mut current = model.get(start_id);
        // Walk up through owners (including memberships) to find target
        while let Some(el) = current {
            if el.name.as_deref() == Some(target_name) {
                return true;
            }
            current = el.owner.as_ref().and_then(|oid| model.get(oid));
        }
        false
    }

    // ================================================================
    // Edge-case & bug-hunting tests
    // ================================================================

    // ── Reparent root element removes from roots ────────────────────

    #[test]
    fn reparent_root_into_package_removes_from_roots() {
        // BUG SCENARIO: If a root element is reparented under a package,
        // it should no longer appear in model.roots.  Otherwise the
        // renderer produces it twice (once as root, once as child).
        let mut h = host("package A; package B;");
        let mut t = h.tracker();

        let a_id = h.find_by_name("A")[0].id().clone();
        let b_id = h.find_by_name("B")[0].id().clone();

        // A and B are both roots
        assert!(
            h.model().roots.contains(&a_id) || {
                // A might be wrapped in a root membership; check either way
                true
            }
        );

        t.reparent(h.model_mut(), &b_id, &a_id);

        // B should NOT appear as a root anymore
        // (neither directly, nor via a root membership that owns B)
        let root_names: Vec<_> = h
            .root_views()
            .iter()
            .map(|v| v.name().unwrap_or("?").to_string())
            .collect();
        assert!(
            !root_names.contains(&"B".to_string()),
            "B should not be a root after reparent into A; roots = {:?}",
            root_names
        );

        // B should be a child of A
        let a_view = h.view(&a_id).unwrap();
        let a_children: Vec<_> = a_view
            .owned_members()
            .iter()
            .filter_map(|m| m.name())
            .collect();
        assert!(
            a_children.contains(&"B"),
            "B should be a child of A; A's children = {:?}",
            a_children
        );

        // Rendered text should show B nested inside A, not at root
        let rendered = h.render();
        let a_pos = rendered.find("package A").expect("should find A");
        let b_pos = rendered.find("package B").expect("should find B");
        assert!(
            b_pos > a_pos,
            "B should appear after (inside) A:\n{rendered}"
        );
    }

    // ── Add element at root (no parent) ─────────────────────────────

    #[test]
    fn add_element_at_root_no_wrapper() {
        let mut h = host("package Existing;");
        let mut t = h.tracker();

        let new_el = Element::new("root-pkg", ElementKind::Package).with_name("NewRoot");
        let new_id = t.add_element(h.model_mut(), new_el, None);

        // Should be a root
        assert!(
            h.model().roots.contains(&new_id),
            "Element added with no parent should be in roots"
        );

        // Should have no owner
        assert!(
            h.model().get(&new_id).unwrap().owner.is_none(),
            "Root element should have no owner"
        );

        // Should render as a top-level element
        let rendered = h.render();
        assert!(
            rendered.contains("NewRoot"),
            "Root element should appear in rendered text:\n{rendered}"
        );
    }

    // ── Membership kind correctness ─────────────────────────────────

    #[test]
    fn add_part_definition_gets_owning_membership() {
        // PartDefinition is NOT a feature → OwningMembership
        let mut h = host("package P;");
        let mut t = h.tracker();

        let p_id = h.find_by_name("P")[0].id().clone();
        let def = Element::new("def1", ElementKind::PartDefinition).with_name("MyDef");
        let def_id = t.add_element(h.model_mut(), def, Some(&p_id));

        // The direct owner of def should be a membership
        let def_el = h.model().get(&def_id).unwrap();
        let owner_id = def_el.owner.as_ref().expect("should have owner");
        let owner = h.model().get(owner_id).unwrap();
        assert_eq!(
            owner.kind,
            ElementKind::OwningMembership,
            "PartDefinition should be wrapped in OwningMembership, got {:?}",
            owner.kind
        );
    }

    #[test]
    fn add_part_usage_gets_feature_membership() {
        // PartUsage IS a feature → FeatureMembership
        let mut h = host("package P { part def Controller; }");
        let mut t = h.tracker();

        let ctrl_id = h.find_by_name("Controller")[0].id().clone();
        let usage = Element::new("u1", ElementKind::PartUsage).with_name("sensor");
        let usage_id = t.add_element(h.model_mut(), usage, Some(&ctrl_id));

        let usage_el = h.model().get(&usage_id).unwrap();
        let owner_id = usage_el.owner.as_ref().expect("should have owner");
        let owner = h.model().get(owner_id).unwrap();
        assert_eq!(
            owner.kind,
            ElementKind::FeatureMembership,
            "PartUsage should be wrapped in FeatureMembership, got {:?}",
            owner.kind
        );
    }

    // ── No double-wrapping ──────────────────────────────────────────

    #[test]
    fn add_element_to_membership_does_not_double_wrap() {
        // If someone adds directly to a membership, no extra wrapper
        let mut h = host("package P { part def A; }");
        let mut t = h.tracker();

        // Find the membership that wraps A
        let a_id = h.find_by_name("A")[0].id().clone();
        let a_el = h.model().get(&a_id).unwrap();
        let membership_id = a_el.owner.clone().unwrap();
        assert!(
            h.model().get(&membership_id).unwrap().kind.is_membership(),
            "A's owner should be a membership"
        );

        // Add element directly to the membership
        let child = Element::new("c1", ElementKind::PartDefinition).with_name("DirectChild");
        let child_id = t.add_element(h.model_mut(), child, Some(&membership_id));

        // The child's owner should be the membership itself (no extra wrapper)
        let child_el = h.model().get(&child_id).unwrap();
        assert_eq!(
            child_el.owner.as_ref().unwrap(),
            &membership_id,
            "Should NOT double-wrap: child's owner should be the membership directly"
        );
    }

    // ── Add-then-remove: tracker state ──────────────────────────────

    #[test]
    fn add_then_remove_clears_created_flag() {
        let mut h = host("package P;");
        let mut t = h.tracker();

        let p_id = h.find_by_name("P")[0].id().clone();
        let el = Element::new("temp", ElementKind::PartDefinition).with_name("Temp");
        let temp_id = t.add_element(h.model_mut(), el, Some(&p_id));

        assert!(t.is_created(&temp_id));
        assert!(t.is_dirty(&temp_id));

        t.remove_element(h.model_mut(), &temp_id);

        // After remove, it should be marked removed and NOT created/dirty
        assert!(t.is_removed(&temp_id));
        assert!(
            !t.is_created(&temp_id),
            "add then remove: created flag should be cleared"
        );
        assert!(
            !t.is_dirty(&temp_id),
            "add then remove: dirty flag should be cleared"
        );

        // Should not render
        let rendered = h.render();
        assert!(
            !rendered.contains("Temp"),
            "Removed element should not render:\n{rendered}"
        );
    }

    // ── Double rename ───────────────────────────────────────────────

    #[test]
    fn double_rename_uses_final_name() {
        let mut h = host("package P { part def A; }");
        let mut t = h.tracker();

        let a_id = h.find_by_name("A")[0].id().clone();
        t.rename(h.model_mut(), &a_id, "B");
        t.rename(h.model_mut(), &a_id, "C");

        let rendered = h.render();
        assert!(rendered.contains("C"), "Should have final name 'C'");
        assert!(
            !rendered.contains(" B"),
            "Should NOT have intermediate name 'B':\n{rendered}"
        );
        assert!(
            !rendered.contains(" A"),
            "Should NOT have original name 'A'"
        );

        // XMI should also have C
        let xmi = xmi_string(h.model());
        assert!(xmi.contains("C"), "XMI should have final name 'C'");
    }

    // ── Toggle abstract on and off ──────────────────────────────────

    #[test]
    fn set_abstract_toggle() {
        let mut h = host("package P { part def V; }");
        let mut t = h.tracker();

        let v_id = h.find_by_name("V")[0].id().clone();

        t.set_abstract(h.model_mut(), &v_id, true);
        assert!(h.model().get(&v_id).unwrap().is_abstract);

        t.set_abstract(h.model_mut(), &v_id, false);
        assert!(!h.model().get(&v_id).unwrap().is_abstract);

        // XMI should NOT have isAbstract="true"
        let xmi = xmi_string(h.model());
        assert!(
            !xmi.contains(r#"isAbstract="true""#),
            "After toggling abstract off, XMI should not have isAbstract=true:\n{xmi}"
        );
    }

    // ── Remove element that has children ────────────────────────────

    #[test]
    fn remove_parent_orphans_children() {
        // Removing a parent does not recursively remove children
        // (they become orphaned in the model). This is by design —
        // the caller should remove children first or handle them.
        let mut h = host("package Outer { part def Inner; }");
        let mut t = h.tracker();

        let outer_id = h.find_by_name("Outer")[0].id().clone();
        let inner_id = h.find_by_name("Inner")[0].id().clone();

        t.remove_element(h.model_mut(), &outer_id);

        assert!(t.is_removed(&outer_id));
        // Inner still exists in the model (orphaned)
        assert!(
            h.model().get(&inner_id).is_some(),
            "Inner should still exist in model elements (orphaned)"
        );

        // But it shouldn't render (no root path to it)
        let rendered = h.render();
        assert!(
            !rendered.contains("Outer"),
            "Removed parent should not render:\n{rendered}"
        );
    }

    // ── Remove all children → empty package renders ─────────────────

    #[test]
    fn remove_all_children_leaves_empty_package() {
        let mut h = host("package P { part def A; part def B; }");
        let mut t = h.tracker();

        let a_id = h.find_by_name("A")[0].id().clone();
        let b_id = h.find_by_name("B")[0].id().clone();

        t.remove_element(h.model_mut(), &a_id);
        t.remove_element(h.model_mut(), &b_id);

        let rendered = h.render();
        assert!(
            rendered.contains("package P"),
            "Empty package should still render:\n{rendered}"
        );
        assert!(
            !rendered.contains("part def"),
            "No part defs should remain:\n{rendered}"
        );
    }

    // ── Remove cleans up membership wrapper ─────────────────────────

    #[test]
    fn remove_element_cleans_up_membership() {
        let mut h = host("package P { part def A; }");
        let mut t = h.tracker();

        let a_id = h.find_by_name("A")[0].id().clone();
        let a_el = h.model().get(&a_id).unwrap();
        let membership_id = a_el.owner.clone().unwrap();

        // Membership should exist before removal
        assert!(h.model().get(&membership_id).is_some());

        t.remove_element(h.model_mut(), &a_id);

        // Membership should be cleaned up (it was left empty)
        assert!(
            h.model().get(&membership_id).is_none(),
            "Empty membership wrapper should be removed when its sole child is removed"
        );
        assert!(
            t.is_removed(&membership_id),
            "Membership should be tracked as removed"
        );
    }

    // ── Reparent wrapped element between packages ───────────────────

    #[test]
    fn reparent_wrapped_element_transfers_membership() {
        let mut h = host("package A { part def X; } package B;");
        let mut t = h.tracker();

        let x_id = h.find_by_name("X")[0].id().clone();
        let b_id = h.find_by_name("B")[0].id().clone();
        let a_id = h.find_by_name("A")[0].id().clone();

        // X should already be wrapped in a membership under A
        let x_el = h.model().get(&x_id).unwrap();
        let m_id = x_el.owner.clone().expect("X should have an owner");
        assert!(
            h.model().get(&m_id).unwrap().kind.is_membership(),
            "X should be wrapped in a membership"
        );

        t.reparent(h.model_mut(), &x_id, &b_id);

        // Membership should now be under B
        let m_el = h.model().get(&m_id).unwrap();
        assert_eq!(
            m_el.owner.as_ref().unwrap(),
            &b_id,
            "Membership should be reparented to B"
        );

        // B's owned_elements should contain the membership
        let b_el = h.model().get(&b_id).unwrap();
        assert!(
            b_el.owned_elements.contains(&m_id),
            "B should own the transferred membership"
        );

        // A should no longer own the membership
        let a_el = h.model().get(&a_id).unwrap();
        assert!(
            !a_el.owned_elements.contains(&m_id),
            "A should no longer own the membership"
        );

        // View API: X's logical owner should now be B
        let x_view = h.view(&x_id).unwrap();
        let owner_view = x_view.owner().expect("X should have an owner");
        assert_eq!(
            owner_view.name(),
            Some("B"),
            "X's logical owner should be B after reparent"
        );
    }

    // ── Documentation with special characters ───────────────────────

    #[test]
    fn documentation_special_chars_xmi_roundtrip() {
        let mut h = host("package P { part def A; }");
        let mut t = h.tracker();

        let a_id = h.find_by_name("A")[0].id().clone();
        // Characters that require XML escaping
        let doc = "Temperature < 100°C & pressure > 1atm \"normal\" conditions";
        t.set_documentation(h.model_mut(), &a_id, Some(doc));

        let xmi_bytes = Xmi.write(h.model()).expect("export");
        let reimported = Xmi.read(&xmi_bytes).expect("import");

        let a_el = reimported
            .iter_elements()
            .find(|e| e.name.as_deref() == Some("A"))
            .expect("A should exist");
        assert_eq!(
            a_el.documentation.as_deref(),
            Some(doc),
            "Documentation with special chars should survive XMI round-trip"
        );
    }

    // ── Rename root element ─────────────────────────────────────────

    #[test]
    fn rename_root_element() {
        let mut h = host("package TopLevel;");
        let mut t = h.tracker();

        let top_id = h.find_by_name("TopLevel")[0].id().clone();
        t.rename(h.model_mut(), &top_id, "Renamed");

        let rendered = h.render();
        assert!(rendered.contains("Renamed"));
        assert!(!rendered.contains("TopLevel"));

        // Still a root
        let roots: Vec<_> = h
            .root_views()
            .iter()
            .map(|v| v.name().unwrap_or("?").to_string())
            .collect();
        assert!(
            roots.contains(&"Renamed".to_string()),
            "Renamed element should still be a root: {:?}",
            roots
        );
    }

    // ── Rename updates qualified name ───────────────────────────────

    #[test]
    fn rename_updates_qualified_name() {
        let mut h = host("package Outer { part def Inner; }");
        let mut t = h.tracker();

        let inner_id = h.find_by_name("Inner")[0].id().clone();
        t.rename(h.model_mut(), &inner_id, "Renamed");

        let el = h.model().get(&inner_id).unwrap();
        if let Some(qn) = &el.qualified_name {
            assert!(
                qn.ends_with("Renamed"),
                "Qualified name should end with new name, got: {}",
                qn
            );
            assert!(
                qn.contains("Outer"),
                "Qualified name should still contain parent path, got: {}",
                qn
            );
        }
    }

    // ── Mutate non-existent element is no-op ────────────────────────

    #[test]
    fn rename_nonexistent_element_is_noop() {
        let mut h = host("package P;");
        let mut t = h.tracker();

        let fake_id = ElementId::new("does-not-exist");
        t.rename(h.model_mut(), &fake_id, "Ghost");

        assert!(
            !t.is_dirty(&fake_id),
            "Renaming non-existent element should not mark it dirty"
        );
    }

    #[test]
    fn remove_nonexistent_element_returns_none() {
        let mut h = host("package P;");
        let mut t = h.tracker();

        let fake_id = ElementId::new("does-not-exist");
        let result = t.remove_element(h.model_mut(), &fake_id);

        assert!(
            result.is_none(),
            "Removing non-existent element should return None"
        );
        assert!(
            !t.is_removed(&fake_id),
            "Non-existent element should not be marked removed"
        );
    }

    // ── Add multiple children to same parent ────────────────────────

    #[test]
    fn add_multiple_children_each_wrapped() {
        let mut h = host("package P;");
        let mut t = h.tracker();

        let p_id = h.find_by_name("P")[0].id().clone();

        let a = Element::new("a1", ElementKind::PartDefinition).with_name("A");
        let b = Element::new("b1", ElementKind::PartDefinition).with_name("B");
        let c = Element::new("c1", ElementKind::PartUsage).with_name("c");

        t.add_element(h.model_mut(), a, Some(&p_id));
        t.add_element(h.model_mut(), b, Some(&p_id));
        t.add_element(h.model_mut(), c, Some(&p_id));

        // All three should be visible as owned members
        let p_view = h.view(&p_id).unwrap();
        let member_names: Vec<_> = p_view
            .owned_members()
            .iter()
            .filter_map(|m| m.name())
            .collect();
        assert!(member_names.contains(&"A"), "A should be a member");
        assert!(member_names.contains(&"B"), "B should be a member");
        assert!(member_names.contains(&"c"), "c should be a member");
        assert_eq!(member_names.len(), 3, "Should have exactly 3 members");

        // Render should show all three
        let rendered = h.render();
        assert!(rendered.contains("part def A"));
        assert!(rendered.contains("part def B"));
        assert!(rendered.contains("c"));
    }

    // ── XMI roundtrip after removing middle element ─────────────────

    #[test]
    fn remove_middle_child_xmi_roundtrip() {
        let mut h = host("package P { part def A; part def B; part def C; }");
        let mut t = h.tracker();

        let b_id = h.find_by_name("B")[0].id().clone();
        t.remove_element(h.model_mut(), &b_id);

        let xmi_bytes = Xmi.write(h.model()).expect("export");
        let reimported = Xmi.read(&xmi_bytes).expect("import");

        let names: Vec<_> = reimported
            .iter_elements()
            .filter_map(|e| e.name.as_ref().map(|n| n.to_string()))
            .collect();
        assert!(names.contains(&"A".to_string()));
        assert!(!names.contains(&"B".to_string()), "B should be gone");
        assert!(names.contains(&"C".to_string()));
    }

    // ── Element ID stability through rename + XMI ───────────────────

    #[test]
    fn element_id_stable_through_rename_and_two_xmi_cycles() {
        let mut h = host("package P { part def Original; }");
        let mut t = h.tracker();

        let id = h.find_by_name("Original")[0].id().clone();
        t.rename(h.model_mut(), &id, "Renamed");

        // Cycle 1: export → import
        let xmi1 = Xmi.write(h.model()).expect("export1");
        let model1 = Xmi.read(&xmi1).expect("import1");

        // Cycle 2: export → import
        let xmi2 = Xmi.write(&model1).expect("export2");
        let model2 = Xmi.read(&xmi2).expect("import2");

        let el = model2
            .iter_elements()
            .find(|e| e.name.as_deref() == Some("Renamed"))
            .expect("Renamed should exist");
        assert_eq!(
            el.id.as_str(),
            id.as_str(),
            "Element ID should be stable through rename + two XMI cycles"
        );
    }

    // ── Add relationship then remove source element ─────────────────

    #[test]
    fn remove_element_cleans_up_relationships() {
        let mut h = host("package P { part def A; part def B; }");
        let mut t = h.tracker();

        let a_id = h.find_by_name("A")[0].id().clone();
        let b_id = h.find_by_name("B")[0].id().clone();

        // Add relationship A → B
        t.add_relationship(
            h.model_mut(),
            ElementId::generate(),
            ElementKind::Specialization,
            a_id.clone(),
            b_id.clone(),
        );
        assert!(h.model().relationship_count() > 0);

        // Remove A — its relationships should be cleaned up
        t.remove_element(h.model_mut(), &a_id);

        // No relationships should reference A anymore
        let dangling: Vec<_> = h
            .model()
            .iter_elements()
            .filter(|e| {
                e.relationship.as_ref().map_or(false, |rd| {
                    rd.source.contains(&a_id) || rd.target.contains(&a_id)
                })
            })
            .collect();
        assert!(
            dangling.is_empty(),
            "No relationships should reference removed element A"
        );
    }
}
