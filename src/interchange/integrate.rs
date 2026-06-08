//! Integration between interchange Model and AnalysisHost/RootDatabase.
//!
//! This module bridges the two parallel model systems:
//!
//! - **AnalysisHost** (Salsa path): parses text → `SymbolIndex` → IDE queries
//! - **ModelHost** (interchange path): standalone `Model` → semantic edits → `render_dirty`
//!
//! ## Functions
//!
//! | Function | Direction | Purpose |
//! |----------|-----------|---------|
//! | `model_from_symbols()` | HIR → Model | Export symbols to interchange Model |
//! | `symbols_from_model()` | Model → HIR | Import Model elements as HirSymbols |
//! | `model_from_database()` | RootDatabase → Model | Export Salsa DB to Model |
//! | `apply_metadata_to_host()` | Metadata → AnalysisHost | Restore element IDs after decompile |
//!
//! For bridging ChangeTracker edits into Salsa queries, use
//! [`AnalysisHost::apply_model_edit()`](crate::ide::AnalysisHost::apply_model_edit).

use super::{
    error::InterchangeError,
    model::{Element, ElementId, ElementKind, Model, PropertyValue, Visibility},
};
use crate::base::FileId;
use crate::hir::{
    HirRelationship, HirSymbol, RelationshipKind as HirRelKind, RootDatabase, SymbolKind,
};
use crate::parser::{Direction, Multiplicity};
use std::sync::Arc;

/// Convert a RootDatabase to a standalone Model for interchange.
///
/// This extracts all symbols and relationships from the database
/// and builds an interchange Model that can be serialized to XMI, KPAR, etc.
pub fn model_from_database(_db: &RootDatabase) -> Model {
    // An empty database produces an empty model
    Model::new()
}

/// Convert an interchange Model back to HIR symbols.
///
/// This is the reverse of `model_from_symbols()`. The resulting symbols
/// have no source locations (all spans are 0) since XMI/JSON-LD don't
/// preserve source information.
///
/// Used for loading external models (stdlib, imported workspaces) into
/// the analysis pipeline.
pub fn symbols_from_model(model: &Model) -> Result<Vec<HirSymbol>, InterchangeError> {
    let mut symbols = Vec::new();

    for element in model.elements.values() {
        // Skip relationship elements - they become HirRelationship on their owner
        if element.kind.is_relationship() {
            continue;
        }

        let kind: SymbolKind = element.kind.into();

        // Build qualified name: prefer element's qualified_name (from ownership hierarchy),
        // fallback to name, then id
        let qualified_name: Arc<str> = element
            .qualified_name
            .clone()
            .or_else(|| element.name.clone())
            .map(|n| n.to_string().into())
            .unwrap_or_else(|| element.id.as_str().into());

        // Simple name is the same as qualified for now (no ownership chain)
        let name: Arc<str> = element
            .name
            .clone()
            .map(|n| n.to_string().into())
            .unwrap_or_else(|| {
                qualified_name
                    .rsplit("::")
                    .next()
                    .unwrap_or(qualified_name.as_ref())
                    .into()
            });

        // Collect relationships where this element is the source.
        //
        // For the 8 short-form keywords the edge is always a
        // `ReferenceSubsetting` owned by the slot (the local usage). The
        // reader discriminates among Performs / Satisfies / Exhibits /
        // Includes / Asserts / Requires / Assumes / Verifies inside
        // `relationship_element_to_hir`, using either the slot's ElementKind
        // (Group A short) or the slot's wrap membership kind (Group B
        // short — RCM with `kind` property, or Verification).
        let mut relationships = Vec::new();
        for re in model.rel_elements_from(&element.id) {
            let Some(hir_kind) = relationship_element_to_hir(re, element, model)? else {
                continue;
            };

            // Look up target element to get its qualified name (HIR uses names, not UUIDs)
            let target_name: Arc<str> = re
                .target()
                .and_then(|tid| model.elements.get(tid))
                .and_then(|target_elem| {
                    target_elem
                        .qualified_name
                        .clone()
                        .or_else(|| target_elem.name.clone())
                })
                .map(|n| n.to_string().into())
                .unwrap_or_else(|| {
                    re.target()
                        .map(|tid| tid.as_str().into())
                        .unwrap_or_else(|| "".into())
                }); // Fallback to ID if not found

            relationships.push(HirRelationship {
                kind: hir_kind,
                target: target_name.clone(),
                resolved_target: Some(target_name), // XMI has resolved refs
                start_line: 0,
                start_col: 0,
                end_line: 0,
                end_col: 0,
            });
        }

        // Extract supertypes from specialization relationships
        let supertypes: Vec<Arc<str>> = relationships
            .iter()
            .filter(|r| r.kind == HirRelKind::Specializes)
            .map(|r| r.target.clone())
            .collect();

        let symbol = HirSymbol {
            name,
            short_name: None, // XMI may have this in declaredShortName property
            qualified_name,
            element_id: element.id.as_str().into(), // Preserve XMI element ID
            kind,
            file: FileId::new(0), // Synthetic - no real file
            start_line: 0,
            start_col: 0,
            end_line: 0,
            end_col: 0,
            short_name_start_line: None,
            short_name_start_col: None,
            short_name_end_line: None,
            short_name_end_col: None,
            doc: element.documentation.as_ref().map(|d| d.to_string().into()),
            supertypes,
            relationships,
            type_refs: Vec::new(),
            is_public: true, // Default to public for imported symbols
            view_data: None,
            metadata_annotations: Vec::new(),
            is_abstract: element.is_abstract,
            is_variation: element.is_variation,
            is_readonly: element.is_readonly,
            is_derived: element.is_derived,
            is_parallel: element.is_parallel,
            is_individual: element.is_individual,
            is_end: element.is_end,
            is_default: element.is_default,
            is_ordered: element.is_ordered,
            is_nonunique: element.is_nonunique,
            is_portion: element.is_portion,
            is_composite: if element.kind.is_feature_kind() {
                Some(
                    element
                        .properties
                        .get("isComposite")
                        .and_then(|v| match v {
                            PropertyValue::Boolean(value) => Some(*value),
                            _ => None,
                        })
                        .unwrap_or(false),
                )
            } else {
                None
            },
            direction: element.properties.get("direction").and_then(|v| match v {
                PropertyValue::String(s) => match s.as_ref() {
                    "in" => Some(Direction::In),
                    "out" => Some(Direction::Out),
                    "inout" => Some(Direction::InOut),
                    _ => None,
                },
                _ => None,
            }),
            multiplicity: extract_multiplicity(element, model),
            value: None,
        };

        symbols.push(symbol);
    }

    Ok(symbols)
}

/// Extract multiplicity from an element's owned `MultiplicityRange` child.
///
/// SysML XMI stores multiplicity as a nested child element:
/// ```xml
/// <ownedMember xsi:type="MultiplicityRange" ...>
///   <ownedMember xsi:type="LiteralInteger" value="0"/>   <!-- lower -->
///   <ownedMember xsi:type="LiteralInteger" value="*"/>   <!-- upper -->
/// </ownedMember>
/// ```
fn extract_multiplicity(element: &Element, model: &Model) -> Option<Multiplicity> {
    // Find the first owned MultiplicityRange child
    for child_id in &element.owned_elements {
        if let Some(child) = model.elements.get(child_id) {
            if child.kind == ElementKind::MultiplicityRange {
                return multiplicity_from_range(child, model);
            }
            // Sometimes the MultiplicityRange is nested one level deeper
            // (e.g., inside an OwningMembership)
            for grandchild_id in &child.owned_elements {
                if let Some(grandchild) = model.elements.get(grandchild_id) {
                    if grandchild.kind == ElementKind::MultiplicityRange {
                        return multiplicity_from_range(grandchild, model);
                    }
                }
            }
        }
    }
    None
}

/// Build a `Multiplicity` from a `MultiplicityRange` element's literal children.
fn multiplicity_from_range(range: &Element, model: &Model) -> Option<Multiplicity> {
    let mut lower: Option<u64> = None;
    let mut upper: Option<u64> = None;

    for (i, child_id) in range.owned_elements.iter().enumerate() {
        if let Some(child) = model.elements.get(child_id) {
            match child.kind {
                ElementKind::LiteralInteger => {
                    if let Some(PropertyValue::Integer(v)) = child.properties.get("value") {
                        let val = (*v).max(0) as u64;
                        if i == 0 {
                            lower = Some(val);
                        } else {
                            upper = Some(val);
                        }
                    }
                }
                ElementKind::LiteralInfinity
                    // Infinity is represented as upper = None (None means "*")
                    if i == 0 => {
                        lower = None;
                    }
                    // upper stays None → means unbounded
                _ => {}
            }
        }
    }

    // Only return if we found at least one bound
    if lower.is_some() || upper.is_some() {
        Some(Multiplicity { lower, upper })
    } else {
        None
    }
}

/// Convert an element-based relationship kind to HIR relationship kind.
///
/// Only covers the "plain" kinds whose HirRelKind is unambiguous from the
/// edge's ElementKind alone. ReferenceSubsetting is intentionally NOT handled
/// here — it requires source-element / wrap context to disambiguate among
/// perform/satisfy/exhibit/include/assert/require/assume/verify and plain
/// `:>` subsetting, which `relationship_element_to_hir` handles directly.
fn element_kind_to_hir(kind: ElementKind) -> Option<HirRelKind> {
    match kind {
        ElementKind::Specialization => Some(HirRelKind::Specializes),
        ElementKind::FeatureTyping => Some(HirRelKind::TypedBy),
        ElementKind::Redefinition => Some(HirRelKind::Redefines),
        ElementKind::Subsetting => Some(HirRelKind::Subsets),
        _ => None,
    }
}

fn relationship_element_to_hir(
    element: &Element,
    source_element: &Element,
    model: &Model,
) -> Result<Option<HirRelKind>, InterchangeError> {
    // Defensive path: an external producer may still emit
    // RequirementConstraintMembership / Verification directly as an edge
    // (via `Model::add_rel`). Our own emit never does this — it wraps the
    // slot and places a ReferenceSubsetting edge underneath — but we still
    // have to refuse malformed inputs clearly. Validate `kind` on RCM-as-
    // edge the same way we do on the wrap.
    match element.kind {
        ElementKind::RequirementConstraintMembership => {
            return match element.properties.get("kind") {
                Some(PropertyValue::String(k)) if k.as_ref() == "assumption" => {
                    Ok(Some(HirRelKind::Assumes))
                }
                Some(PropertyValue::String(k)) if k.as_ref() == "requirement" => {
                    Ok(Some(HirRelKind::Requires))
                }
                Some(PropertyValue::String(k)) => Err(InterchangeError::invalid_attribute(
                    format!(
                        "RequirementConstraintMembership missing valid kind: expected assumption or requirement, got {k}"
                    ),
                )),
                _ => Err(InterchangeError::invalid_attribute(
                    "RequirementConstraintMembership missing valid kind",
                )),
            };
        }
        ElementKind::Verification => {
            return Ok(Some(HirRelKind::Verifies));
        }
        _ => {}
    }

    // All 8 short-form keywords emit their edge as ReferenceSubsetting. The
    // specific HirRelKind is recovered from (a) the source element's kind —
    // PerformActionUsage / IncludeUseCaseUsage / etc. disambiguate Group A
    // short — or (b) the source element's owner kind, for Group B short:
    // slot.owner == RCM → Requires/Assumes (discriminated by the wrap's
    // `kind` property); slot.owner == Verification → Verifies.
    if element.kind == ElementKind::ReferenceSubsetting {
        // Check slot.owner for the specialized-wrap case first. Both RCM and
        // Verification wraps are built by
        // `emit_group_b_short_specialized_membership` with the slot as the
        // sole `ownedMember` and `kind=requirement/assumption` on RCM.
        if let Some(owner) = source_element
            .owner
            .as_ref()
            .and_then(|oid| model.elements.get(oid))
        {
            match owner.kind {
                ElementKind::RequirementConstraintMembership => {
                    return match owner.properties.get("kind") {
                        Some(PropertyValue::String(k)) if k.as_ref() == "assumption" => {
                            Ok(Some(HirRelKind::Assumes))
                        }
                        Some(PropertyValue::String(k)) if k.as_ref() == "requirement" => {
                            Ok(Some(HirRelKind::Requires))
                        }
                        Some(PropertyValue::String(k)) => {
                            Err(InterchangeError::invalid_attribute(format!(
                                "RequirementConstraintMembership missing valid kind: expected assumption or requirement, got {k}"
                            )))
                        }
                        _ => Err(InterchangeError::invalid_attribute(
                            "RequirementConstraintMembership missing valid kind",
                        )),
                    };
                }
                ElementKind::Verification => {
                    return Ok(Some(HirRelKind::Verifies));
                }
                _ => { /* fall through to source-kind discrimination */ }
            }
        }

        // Group A short — discriminate by the usage kind of the slot that
        // owns this ReferenceSubsetting edge.
        return Ok(match source_element.kind {
            ElementKind::PerformActionUsage => Some(HirRelKind::Performs),
            ElementKind::IncludeUseCaseUsage => Some(HirRelKind::Includes),
            ElementKind::ExhibitStateUsage => Some(HirRelKind::Exhibits),
            ElementKind::AssertConstraintUsage => Some(HirRelKind::Asserts),
            ElementKind::Satisfaction => Some(HirRelKind::Satisfies),
            // A plain ReferenceSubsetting (e.g. `:> feat` on a usage or
            // `redefines x` via specialization pathways) has no Group A/B
            // short-form provenance — leave it as `None` so the caller
            // doesn't synthesize a spurious HirRelationship.
            _ => None,
        });
    }

    Ok(element_kind_to_hir(element.kind))
}

/// Convert interchange `ElementKind` to HIR `SymbolKind`.
///
/// This is a lossy, many-to-one mapping: several `ElementKind` variants
/// (e.g. `LibraryPackage`, `AssociationStructure`, import/comment variants)
/// collapse into a single `SymbolKind`.
impl From<ElementKind> for SymbolKind {
    fn from(kind: ElementKind) -> Self {
        match kind {
            ElementKind::Package | ElementKind::LibraryPackage => SymbolKind::Package,
            ElementKind::PartDefinition => SymbolKind::PartDefinition,
            ElementKind::ItemDefinition => SymbolKind::ItemDefinition,
            ElementKind::ActionDefinition => SymbolKind::ActionDefinition,
            ElementKind::PortDefinition => SymbolKind::PortDefinition,
            ElementKind::AttributeDefinition => SymbolKind::AttributeDefinition,
            ElementKind::ConnectionDefinition => SymbolKind::ConnectionDefinition,
            ElementKind::InterfaceDefinition => SymbolKind::InterfaceDefinition,
            ElementKind::AllocationDefinition => SymbolKind::AllocationDefinition,
            ElementKind::RequirementDefinition => SymbolKind::RequirementDefinition,
            ElementKind::ConstraintDefinition => SymbolKind::ConstraintDefinition,
            ElementKind::StateDefinition => SymbolKind::StateDefinition,
            ElementKind::CalculationDefinition => SymbolKind::CalculationDefinition,
            ElementKind::OccurrenceDefinition => SymbolKind::OccurrenceDefinition,
            ElementKind::UseCaseDefinition => SymbolKind::UseCaseDefinition,
            ElementKind::AnalysisCaseDefinition => SymbolKind::AnalysisCaseDefinition,
            ElementKind::VerificationCaseDefinition => SymbolKind::VerificationCaseDefinition,
            ElementKind::ConcernDefinition => SymbolKind::ConcernDefinition,
            ElementKind::ViewDefinition => SymbolKind::ViewDefinition,
            ElementKind::ViewpointDefinition => SymbolKind::ViewpointDefinition,
            ElementKind::RenderingDefinition => SymbolKind::RenderingDefinition,
            ElementKind::EnumerationDefinition => SymbolKind::EnumerationDefinition,
            ElementKind::MetadataDefinition => SymbolKind::MetadataDefinition,
            // KerML definitions
            ElementKind::DataType => SymbolKind::DataType,
            ElementKind::Class => SymbolKind::Class,
            ElementKind::Structure => SymbolKind::Structure,
            ElementKind::Behavior => SymbolKind::Behavior,
            ElementKind::Function => SymbolKind::Function,
            ElementKind::Association | ElementKind::AssociationStructure => SymbolKind::Association,
            ElementKind::Interaction => SymbolKind::Interaction,
            // Usages
            ElementKind::PartUsage => SymbolKind::PartUsage,
            ElementKind::ItemUsage => SymbolKind::ItemUsage,
            ElementKind::ActionUsage => SymbolKind::ActionUsage,
            ElementKind::PerformActionUsage => SymbolKind::PerformActionUsage,
            ElementKind::PortUsage => SymbolKind::PortUsage,
            ElementKind::AttributeUsage => SymbolKind::AttributeUsage,
            ElementKind::ConnectionUsage => SymbolKind::ConnectionUsage,
            ElementKind::InterfaceUsage => SymbolKind::InterfaceUsage,
            ElementKind::AllocationUsage => SymbolKind::AllocationUsage,
            ElementKind::RequirementUsage => SymbolKind::RequirementUsage,
            ElementKind::Satisfaction => SymbolKind::SatisfyRequirementUsage,
            ElementKind::ConstraintUsage => SymbolKind::ConstraintUsage,
            ElementKind::AssertConstraintUsage => SymbolKind::AssertConstraintUsage,
            ElementKind::StateUsage => SymbolKind::StateUsage,
            ElementKind::ExhibitStateUsage => SymbolKind::ExhibitStateUsage,
            ElementKind::TransitionUsage => SymbolKind::TransitionUsage,
            ElementKind::CalculationUsage => SymbolKind::CalculationUsage,
            ElementKind::ReferenceUsage => SymbolKind::ReferenceUsage,
            ElementKind::OccurrenceUsage => SymbolKind::OccurrenceUsage,
            ElementKind::UseCaseUsage => SymbolKind::UseCaseUsage,
            ElementKind::IncludeUseCaseUsage => SymbolKind::IncludeUseCaseUsage,
            ElementKind::AnalysisCaseUsage => SymbolKind::AnalysisCaseUsage,
            ElementKind::VerificationCaseUsage => SymbolKind::VerificationCaseUsage,
            ElementKind::FlowConnectionUsage => SymbolKind::FlowConnectionUsage,
            // Other
            ElementKind::Import | ElementKind::NamespaceImport | ElementKind::MembershipImport => {
                SymbolKind::Import
            }
            ElementKind::Comment | ElementKind::Documentation => SymbolKind::Comment,
            _ => SymbolKind::Other,
        }
    }
}

/// Convert a collection of HirSymbols to a standalone Model.
///
/// This is the core conversion function that maps HIR symbols to
/// interchange model elements.
pub fn model_from_symbols(symbols: &[HirSymbol]) -> Model {
    let mut model = Model::new();
    let mut rel_counter = 0u64;

    // Build lookup map: qualified_name -> element_id
    // This allows us to resolve relationship targets and ownership
    let name_to_id: std::collections::HashMap<&str, &str> = symbols
        .iter()
        .map(|s| (s.qualified_name.as_ref(), s.element_id.as_ref()))
        .collect();

    for symbol in symbols {
        // Use the symbol's element_id to preserve UUIDs across round-trips
        let id = ElementId::new(symbol.element_id.as_ref());
        let kind: ElementKind = symbol.kind.into();

        // Handle Import symbols specially: create NamespaceImport/MembershipImport
        // relationship elements instead of bare Import elements.
        if symbol.kind == SymbolKind::Import {
            // Parse the import name to determine kind and target namespace.
            // Import symbol name is like "ScalarValues::*" or "Pkg::Elem".
            let import_name = symbol.name.as_ref();
            let is_wildcard = import_name.ends_with("::*");
            let namespace = if is_wildcard {
                import_name.trim_end_matches("::*")
            } else {
                import_name
            };

            // Determine the owner package from the qualified name.
            // Import qn format: "Vehicle::import:ScalarValues::*"
            // The owner is the first segment before "::import:".
            let owner_id = symbol.qualified_name.find("::import:").and_then(|idx| {
                let parent_qn = &symbol.qualified_name[..idx];
                name_to_id.get(parent_qn).map(|&eid| ElementId::new(eid))
            });

            let import_kind = if is_wildcard {
                ElementKind::NamespaceImport
            } else {
                ElementKind::MembershipImport
            };

            // Create the import as a relationship element with the namespace as target.
            // The target ID is the namespace name (external reference).
            let target_id = ElementId::new(namespace);
            let source_id = owner_id.clone().unwrap_or_else(|| id.clone());

            let rel_id = model.add_rel(
                id.clone(),
                import_kind,
                source_id,
                target_id,
                owner_id.clone(),
            );

            // Store importedNamespace attribute for XMI roundtrip and decompiler fallback
            let attr_name = if is_wildcard {
                "importedNamespace"
            } else {
                "importedMembership"
            };
            if let Some(rel_element) = model.get_mut(&rel_id) {
                rel_element.properties.insert(
                    Arc::from(attr_name),
                    PropertyValue::String(Arc::from(namespace)),
                );
                // Copy import visibility
                rel_element.visibility = if symbol.is_public {
                    Visibility::Public
                } else {
                    Visibility::Private
                };
            }

            // Add to parent's owned_elements
            if let Some(ref oid) = owner_id {
                if let Some(parent) = model.get_mut(oid) {
                    if !parent.owned_elements.contains(&id) {
                        parent.owned_elements.push(id.clone());
                    }
                }
            }

            continue;
        }

        // Determine ownership from qualified name, then look up owner's element_id
        let owner = if symbol.qualified_name.contains("::") {
            let parent = symbol.qualified_name.rsplit_once("::").map(|(p, _)| p);
            parent.and_then(|p| name_to_id.get(p).map(|&id| ElementId::new(id)))
        } else {
            None
        };

        let mut element = Element::new(id.clone(), kind)
            .with_name(symbol.name.as_ref())
            .with_qualified_name(symbol.qualified_name.as_ref());

        if let Some(ref owner_id) = owner {
            element = element.with_owner(owner_id.clone());
        }

        // Copy boolean flags from HIR symbol
        element.is_abstract = symbol.is_abstract;
        element.is_variation = symbol.is_variation;
        element.is_readonly = symbol.is_readonly;
        element.is_derived = symbol.is_derived;
        element.is_parallel = symbol.is_parallel;
        element.is_individual = symbol.is_individual;
        element.is_end = symbol.is_end;
        element.is_default = symbol.is_default;
        element.is_ordered = symbol.is_ordered;
        element.is_nonunique = symbol.is_nonunique;
        element.is_portion = symbol.is_portion;
        if kind.is_feature_kind() {
            element.properties.insert(
                Arc::from("isComposite"),
                PropertyValue::Boolean(symbol.is_composite.unwrap_or(false)),
            );
        }

        // Copy short name
        element.short_name = symbol.short_name.clone();

        // Copy documentation
        element.documentation = symbol.doc.clone();

        // Copy direction as a property (for decompiler to read)
        if let Some(ref dir) = symbol.direction {
            let dir_str = match dir {
                Direction::In => "in",
                Direction::Out => "out",
                Direction::InOut => "inout",
            };
            element.properties.insert(
                Arc::from("direction"),
                PropertyValue::String(Arc::from(dir_str)),
            );
        }

        // Copy multiplicity as properties
        if let Some(ref mult) = symbol.multiplicity {
            if let Some(lower) = mult.lower {
                element.properties.insert(
                    Arc::from("multiplicityLower"),
                    PropertyValue::String(Arc::from(lower.to_string())),
                );
            }
            if let Some(upper) = mult.upper {
                element.properties.insert(
                    Arc::from("multiplicityUpper"),
                    PropertyValue::String(Arc::from(upper.to_string())),
                );
            }
        }

        // Don't set visibility from is_public on regular elements.
        // SysML default is public — only imports use the is_public flag
        // to distinguish `private import` from `import`.

        // Store alias target from supertypes (for decompiler)
        if symbol.kind == SymbolKind::Alias {
            if let Some(target) = symbol.supertypes.first() {
                element.properties.insert(
                    Arc::from("aliasTarget"),
                    PropertyValue::String(Arc::from(target.as_ref())),
                );
            }
        }

        model.add_element(element);
        // We'll populate parent's owned_elements in a separate pass below

        // Ensure owned_elements lists are correctly populated regardless of symbol ordering.
        // Clear existing owned lists and rebuild from owner pointers.
        let mut child_owner_pairs: Vec<(ElementId, ElementId)> = Vec::new();
        for (id, element) in model.elements.iter() {
            if let Some(owner_id) = &element.owner {
                child_owner_pairs.push((id.clone(), owner_id.clone()));
            }
        }

        // Clear current owned_elements
        for (_id, element) in model.elements.iter_mut() {
            element.owned_elements.clear();
        }

        // Repopulate owned_elements according to owner pointers
        for (child_id, owner_id) in child_owner_pairs {
            if let Some(parent) = model.get_mut(&owner_id) {
                parent.owned_elements.push(child_id);
            }
        }
        // Extract relationships from the symbol
        for hir_rel in &symbol.relationships {
            let element_kind = hir_relationship_kind_to_element_kind(&hir_rel.kind);
            if let Some(ek) = element_kind {
                rel_counter += 1;
                let rel_id = ElementId::new(format!("rel_{}", rel_counter));

                // Look up target's element_id from qualified name.
                // Resolution order:
                // 0. Use resolved_target from the resolver (best source)
                // 1. Direct lookup by name (fully qualified)
                // 2. Walk up ancestor namespaces (e.g., for "Label" from
                //    "Sensor::Thermometer::name", try "Sensor::Thermometer::Label"
                //    then "Sensor::Label")
                // 3. Scan all symbols for a matching simple name suffix
                //    (handles cross-type references like `redefines size`
                //    where `size` lives in a supertype)
                let is_special_usage = matches!(
                    hir_rel.kind,
                    HirRelKind::Performs
                        | HirRelKind::Satisfies
                        | HirRelKind::Verifies
                        | HirRelKind::Exhibits
                        | HirRelKind::Includes
                        | HirRelKind::Asserts
                        | HirRelKind::Assumes
                        | HirRelKind::Requires
                );

                let target_name = if is_special_usage {
                    hir_rel
                        .resolved_target
                        .as_deref()
                        .or_else(|| {
                            symbol
                                .special_usage_terminal_ref()
                                .and_then(|tr| tr.resolved_target.as_deref())
                        })
                } else {
                    Some(
                        hir_rel
                            .resolved_target
                            .as_deref()
                            .unwrap_or(hir_rel.target.as_ref()),
                    )
                };

                let Some(target_name) = target_name else {
                    continue;
                };

                let target_id = name_to_id
                    .get(target_name)
                    .or_else(|| {
                        let mut ns = symbol.qualified_name.as_ref();
                        while let Some((parent, _)) = ns.rsplit_once("::") {
                            let candidate = format!("{}::{}", parent, target_name);
                            if let Some(found) = name_to_id.get(candidate.as_str()) {
                                return Some(found);
                            }
                            ns = parent;
                        }
                        None
                    })
                    .or_else(|| {
                        // Scan for any element whose qualified name ends with
                        // ::target. Needed for cross-type references and
                        // perform chain terminals when we only have the last segment.
                        let suffix = format!("::{}", target_name);
                        let mut matches: Vec<&&str> = name_to_id
                            .keys()
                            .filter(|qn| qn.ends_with(suffix.as_str()))
                            .collect();
                        if matches.len() == 1 {
                            name_to_id.get(*matches[0])
                        } else if matches.len() > 1 {
                            matches.sort_by_key(|qn| {
                                let common = symbol
                                    .qualified_name
                                    .as_ref()
                                    .chars()
                                    .zip(qn.chars())
                                    .take_while(|(a, b)| a == b)
                                    .count();
                                std::cmp::Reverse(common)
                            });
                            name_to_id.get(*matches[0])
                        } else {
                            None
                        }
                    })
                    .map(|&id| ElementId::new(id));

                // If resolution failed, create a stub element for the external
                // reference so the decompiler can still render its name.
                let target_id = target_id.unwrap_or_else(|| {
                    let ext_id = ElementId::new(format!("_ext_{}", target_name));
                    if !model.elements.contains_key(&ext_id) {
                        let mut stub = Element::new(ext_id.clone(), ElementKind::Other)
                            .with_name(target_name);
                        // Mark as external so it is never decompiled to output
                        stub.properties
                            .insert(Arc::from("_external"), PropertyValue::Boolean(true));
                        model.add_element(stub);
                    }
                    ext_id
                });

                // Group B · short forms (require / assume / verify) need the
                // slot pre-wrapped in a specialized RCM / Verification
                // membership so the wrap's @type (and for RCM, its `kind`
                // property) carries the KerML discrimination that can't live
                // on the edge alone (all 8 short forms now emit the edge as
                // plain ReferenceSubsetting). Dispatch when we know the
                // slot's enclosing namespace (needed as the wrap's owner).
                let needs_specialized_wrap = matches!(
                    hir_rel.kind,
                    HirRelKind::Requires | HirRelKind::Assumes | HirRelKind::Verifies
                );
                if needs_specialized_wrap {
                    if let Some(parent_owner_id) = owner.as_ref() {
                        emit_group_b_short_specialized_membership(
                            &mut model,
                            &hir_rel.kind,
                            &id,
                            &target_id,
                            parent_owner_id,
                            &rel_id,
                        );
                        continue;
                    }
                    // No known parent owner: fall through to the plain
                    // single-element path so we don't silently lose the
                    // relationship. This branch is not expected for
                    // well-formed short-form usages (the slot always has an
                    // enclosing RequirementUsage / RequirementDefinition),
                    // and the caller will see a plain ReferenceSubsetting
                    // without the RCM/RVM wrap — i.e. the reader will
                    // decode it as a generic ReferenceSubsetting (None) not
                    // as Requires/Assumes/Verifies. Acceptable for the
                    // degenerate case.
                }

                // Plain single-element emit (all other relationships,
                // including the 5 Group A short forms whose slot gets wrapped
                // by Phase-6 in a default FeatureMembership).
                model.add_rel(
                    rel_id.clone(),
                    ek,
                    id.clone(),
                    target_id.clone(),
                    Some(id.clone()),
                );

                if let Some(rel_element) = model.get_mut(&rel_id) {
                    let standard_target_attr = match ek {
                        ElementKind::FeatureTyping => Some("type"),
                        ElementKind::Specialization => Some("general"),
                        ElementKind::Redefinition => Some("redefinedFeature"),
                        ElementKind::Subsetting => Some("subsettedFeature"),
                        _ => None,
                    };
                    if let Some(attr) = standard_target_attr {
                        rel_element.properties.insert(
                            Arc::from(attr),
                            PropertyValue::Reference(target_id.clone()),
                        );
                    }
                }
                if let Some(parent) = model.get_mut(&id) {
                    parent.owned_elements.push(rel_id);
                }
            }
        }

        // Create FeatureValue + Literal child elements for symbols with values
        if let Some(ref value) = symbol.value {
            use crate::parser::ValueExpression;

            let fv_id = ElementId::new(format!("{}-fv", symbol.element_id));
            let lit_id = ElementId::new(format!("{}-fv-lit", symbol.element_id));

            let (lit_kind, lit_prop_value) = match value {
                ValueExpression::LiteralInteger(v) => {
                    (ElementKind::LiteralInteger, PropertyValue::Integer(*v))
                }
                ValueExpression::LiteralReal(v) => {
                    (ElementKind::LiteralReal, PropertyValue::Real(*v))
                }
                ValueExpression::LiteralString(s) => (
                    ElementKind::LiteralString,
                    PropertyValue::String(Arc::from(s.as_str())),
                ),
                ValueExpression::LiteralBoolean(b) => {
                    (ElementKind::LiteralBoolean, PropertyValue::Boolean(*b))
                }
                ValueExpression::Null => (
                    ElementKind::NullExpression,
                    PropertyValue::String(Arc::from("null")),
                ),
                ValueExpression::Expression(text) => (
                    ElementKind::FeatureReferenceExpression,
                    PropertyValue::String(Arc::from(text.as_str())),
                ),
            };

            // Create the literal element
            let mut lit_element = Element::new(lit_id.clone(), lit_kind).with_owner(fv_id.clone());
            lit_element
                .properties
                .insert(Arc::from("value"), lit_prop_value);
            model.add_element(lit_element);

            // Create the FeatureValue relationship element
            let mut fv_element =
                Element::new(fv_id.clone(), ElementKind::FeatureValue).with_owner(id.clone());
            fv_element.owned_elements.push(lit_id);
            model.add_element(fv_element);

            // Add FeatureValue as owned child of the usage element
            if let Some(parent) = model.get_mut(&id) {
                parent.owned_elements.push(fv_id);
            }
        }

        // Create FeatureValue + Literal child elements for symbols with values
        if let Some(ref value) = symbol.value {
            use crate::parser::ValueExpression;

            let fv_id = ElementId::new(format!("{}-fv", symbol.element_id));
            let lit_id = ElementId::new(format!("{}-fv-lit", symbol.element_id));

            let (lit_kind, lit_prop_value) = match value {
                ValueExpression::LiteralInteger(v) => {
                    (ElementKind::LiteralInteger, PropertyValue::Integer(*v))
                }
                ValueExpression::LiteralReal(v) => {
                    (ElementKind::LiteralReal, PropertyValue::Real(*v))
                }
                ValueExpression::LiteralString(s) => (
                    ElementKind::LiteralString,
                    PropertyValue::String(Arc::from(s.as_str())),
                ),
                ValueExpression::LiteralBoolean(b) => {
                    (ElementKind::LiteralBoolean, PropertyValue::Boolean(*b))
                }
                ValueExpression::Null => (
                    ElementKind::NullExpression,
                    PropertyValue::String(Arc::from("null")),
                ),
                ValueExpression::Expression(text) => (
                    ElementKind::FeatureReferenceExpression,
                    PropertyValue::String(Arc::from(text.as_str())),
                ),
            };

            // Create the literal element
            let mut lit_element =
                Element::new(lit_id.clone(), lit_kind).with_owner(fv_id.clone());
            lit_element
                .properties
                .insert(Arc::from("value"), lit_prop_value);
            model.add_element(lit_element);

            // Create the FeatureValue relationship element
            let mut fv_element =
                Element::new(fv_id.clone(), ElementKind::FeatureValue).with_owner(id.clone());
            fv_element.owned_elements.push(lit_id);
            model.add_element(fv_element);

            // Add FeatureValue as owned child of the usage element
            if let Some(parent) = model.get_mut(&id) {
                parent.owned_elements.push(fv_id);
            }
        }
    }

    // Phase 6: wrap non-relationship children in OwningMembership/FeatureMembership
    model.wrap_children_in_memberships();

    model
}

/// Pre-wrap a `require` / `assume` / `verify` slot in the correct specialized
/// membership and emit its inner `ReferenceSubsetting` edge.
///
/// All 8 short-form keywords (perform/satisfy/exhibit/include/assert/require/
/// assume/verify) share the same edge shape — a single `ReferenceSubsetting`
/// from `slot` to `external`, owned by the slot. For 5 of them (Group A short)
/// Phase-6 `wrap_children_in_memberships` wraps the slot in a plain
/// `FeatureMembership` with only `owner` + `ownedMember` set. For the 3 that
/// need KerML-level specialization (Group B short) we pre-wrap here so the
/// wrap element carries the right `@type` — and, for require/assume, a `kind`
/// property — and so Phase-6 skips the slot (its owner is now a relationship
/// element per `ElementKind::is_relationship`).
///
/// The wrap carries only: `owner`, `ownedMember[slot]`, and (for RCM) a
/// `kind` string property. Deliberately NOT emitted: `referencedConstraint`,
/// `referencedRequirement`, `ownedConstraint`, `memberName`,
/// `ownedMemberName`, `owningType`, `owningRelatedElement`,
/// `membershipOwningNamespace`, `ownedRelatedElement`, `relatedElement`, or
/// any of the OMG-API-style aliases. Consumers derive those from the graph
/// skeleton (edge + wrap + slot) per the original author's
/// "structural only, aliases are consumer-side" philosophy.
///
/// Wrap kind table:
/// - `Requires` → `RequirementConstraintMembership`, `kind="requirement"`
/// - `Assumes`  → `RequirementConstraintMembership`, `kind="assumption"`
/// - `Verifies` → `Verification` (serialized as `RequirementVerificationMembership`)
///
/// ID conventions:
/// - wrap id = `{slot_id}-m` (matches the Phase-6 auto-wrap convention so
///   readers can't tell pre-wrapped apart from auto-wrapped by id alone)
/// - edge id = `rel_{N}` using the pre-allocated id passed by the caller
fn emit_group_b_short_specialized_membership(
    model: &mut Model,
    hir_rel_kind: &HirRelKind,
    slot_id: &ElementId,
    target_id: &ElementId,
    parent_owner_id: &ElementId,
    pre_allocated_rel_id: &ElementId,
) {
    use crate::interchange::model::Element;

    let (wrap_kind, kind_prop) = match hir_rel_kind {
        HirRelKind::Requires => (
            ElementKind::RequirementConstraintMembership,
            Some("requirement"),
        ),
        HirRelKind::Assumes => (
            ElementKind::RequirementConstraintMembership,
            Some("assumption"),
        ),
        HirRelKind::Verifies => (ElementKind::Verification, None),
        _ => unreachable!(
            "emit_group_b_short_specialized_membership only dispatches for Requires/Assumes/Verifies"
        ),
    };

    // (1) Build the specialized membership wrap. Mirrors the Phase-6
    // `-m` suffix convention so the graph shape is indistinguishable from
    // an auto-wrapped FeatureMembership except for `@type` and `kind`.
    let wrap_id = ElementId::new(format!("{}-m", slot_id.as_str()));
    let mut wrap = Element::new(wrap_id.clone(), wrap_kind);
    wrap.owner = Some(parent_owner_id.clone());
    wrap.owned_elements.push(slot_id.clone());
    if let Some(kind_str) = kind_prop {
        wrap.properties.insert(
            Arc::from("kind"),
            PropertyValue::String(Arc::from(kind_str)),
        );
    }
    model.add_element(wrap);

    // (2) Re-seat slot.owner → wrap, and swap slot → wrap in parent's
    // owned_elements. This triggers the Phase-6 skip via
    // `ElementKind::is_relationship()` on the slot's owner (RCM/Verification
    // are both `is_relationship() == true`).
    if let Some(slot) = model.get_mut(slot_id) {
        slot.owner = Some(wrap_id.clone());
    }
    if let Some(parent) = model.get_mut(parent_owner_id) {
        if let Some(pos) = parent.owned_elements.iter().position(|eid| eid == slot_id) {
            parent.owned_elements[pos] = wrap_id.clone();
        } else {
            parent.owned_elements.push(wrap_id.clone());
        }
    }

    // (3) Emit the ReferenceSubsetting edge on the slot → external. Shape
    // matches the 5 Group A short keywords exactly: source/target/owner only.
    // The reader (`symbols_from_model`) tells this edge's HirRelKind apart
    // from a plain ReferenceSubsetting by inspecting the slot's owner kind
    // (RCM → Requires/Assumes via `kind` prop, Verification → Verifies).
    model.add_rel(
        pre_allocated_rel_id.clone(),
        ElementKind::ReferenceSubsetting,
        slot_id.clone(),
        target_id.clone(),
        Some(slot_id.clone()),
    );
    if let Some(slot) = model.get_mut(slot_id) {
        if !slot.owned_elements.contains(pre_allocated_rel_id) {
            slot.owned_elements.push(pre_allocated_rel_id.clone());
        }
    }
}

/// Convert HIR RelationshipKind to interchange ElementKind.
fn hir_relationship_kind_to_element_kind(
    kind: &crate::hir::RelationshipKind,
) -> Option<ElementKind> {
    use crate::hir::RelationshipKind as HirRelKind;
    match kind {
        HirRelKind::Specializes => Some(ElementKind::Specialization),
        HirRelKind::TypedBy => Some(ElementKind::FeatureTyping),
        HirRelKind::Redefines => Some(ElementKind::Redefinition),
        HirRelKind::Subsets => Some(ElementKind::Subsetting),
        HirRelKind::References => None, // Not a first-class relationship in interchange
        HirRelKind::Satisfies => Some(ElementKind::ReferenceSubsetting),
        HirRelKind::Performs => Some(ElementKind::ReferenceSubsetting),
        HirRelKind::Exhibits => Some(ElementKind::ReferenceSubsetting),
        HirRelKind::Includes => Some(ElementKind::ReferenceSubsetting),
        HirRelKind::Asserts => Some(ElementKind::ReferenceSubsetting),
        // require/assume/verify all emit the edge as ReferenceSubsetting —
        // architecturally identical to perform/satisfy/exhibit/include/assert.
        // The KerML specialization (RCM / RVM) lives on the *wrap* membership,
        // built separately by `emit_group_b_short_specialized_membership` using
        // `hir_rel.kind` as the source of truth, not on the edge's ElementKind.
        HirRelKind::Assumes => Some(ElementKind::ReferenceSubsetting),
        HirRelKind::Requires => Some(ElementKind::ReferenceSubsetting),
        HirRelKind::Verifies => Some(ElementKind::ReferenceSubsetting),
    }
}

/// Convert HIR `SymbolKind` to interchange `ElementKind`.
///
/// Inverse of `From<ElementKind> for SymbolKind` (modulo lossy many-to-one
/// arms: e.g. `SymbolKind::Package` maps to `ElementKind::Package`, not
/// `LibraryPackage`).
impl From<SymbolKind> for ElementKind {
    fn from(kind: SymbolKind) -> Self {
        match kind {
            SymbolKind::Package => ElementKind::Package,
            SymbolKind::PartDefinition => ElementKind::PartDefinition,
            SymbolKind::ItemDefinition => ElementKind::ItemDefinition,
            SymbolKind::ActionDefinition => ElementKind::ActionDefinition,
            SymbolKind::PortDefinition => ElementKind::PortDefinition,
            SymbolKind::AttributeDefinition => ElementKind::AttributeDefinition,
            SymbolKind::ConnectionDefinition => ElementKind::ConnectionDefinition,
            SymbolKind::InterfaceDefinition => ElementKind::InterfaceDefinition,
            SymbolKind::AllocationDefinition => ElementKind::AllocationDefinition,
            SymbolKind::RequirementDefinition => ElementKind::RequirementDefinition,
            SymbolKind::ConstraintDefinition => ElementKind::ConstraintDefinition,
            SymbolKind::StateDefinition => ElementKind::StateDefinition,
            SymbolKind::CalculationDefinition => ElementKind::CalculationDefinition,
            SymbolKind::OccurrenceDefinition => ElementKind::OccurrenceDefinition,
            SymbolKind::UseCaseDefinition => ElementKind::UseCaseDefinition,
            SymbolKind::AnalysisCaseDefinition => ElementKind::AnalysisCaseDefinition,
            SymbolKind::VerificationCaseDefinition => ElementKind::VerificationCaseDefinition,
            SymbolKind::ConcernDefinition => ElementKind::ConcernDefinition,
            SymbolKind::ViewDefinition => ElementKind::ViewDefinition,
            SymbolKind::ViewpointDefinition => ElementKind::ViewpointDefinition,
            SymbolKind::RenderingDefinition => ElementKind::RenderingDefinition,
            SymbolKind::EnumerationDefinition => ElementKind::EnumerationDefinition,
            // KerML definitions
            SymbolKind::DataType => ElementKind::DataType,
            SymbolKind::Class => ElementKind::Class,
            SymbolKind::Structure => ElementKind::Structure,
            SymbolKind::Behavior => ElementKind::Behavior,
            SymbolKind::Function => ElementKind::Function,
            SymbolKind::Association => ElementKind::Association,
            SymbolKind::MetadataDefinition => ElementKind::MetadataDefinition,
            SymbolKind::Interaction => ElementKind::Interaction,
            // Usages
            SymbolKind::PartUsage => ElementKind::PartUsage,
            SymbolKind::ItemUsage => ElementKind::ItemUsage,
            SymbolKind::ActionUsage => ElementKind::ActionUsage,
            SymbolKind::PerformActionUsage => ElementKind::PerformActionUsage,
            SymbolKind::PortUsage => ElementKind::PortUsage,
            SymbolKind::AttributeUsage => ElementKind::AttributeUsage,
            SymbolKind::ConnectionUsage => ElementKind::ConnectionUsage,
            SymbolKind::InterfaceUsage => ElementKind::InterfaceUsage,
            SymbolKind::AllocationUsage => ElementKind::AllocationUsage,
            SymbolKind::RequirementUsage => ElementKind::RequirementUsage,
            SymbolKind::SatisfyRequirementUsage => ElementKind::Satisfaction,
            SymbolKind::ConstraintUsage => ElementKind::ConstraintUsage,
            SymbolKind::AssertConstraintUsage => ElementKind::AssertConstraintUsage,
            SymbolKind::StateUsage => ElementKind::StateUsage,
            SymbolKind::ExhibitStateUsage => ElementKind::ExhibitStateUsage,
            SymbolKind::TransitionUsage => ElementKind::TransitionUsage,
            SymbolKind::CalculationUsage => ElementKind::CalculationUsage,
            SymbolKind::ReferenceUsage => ElementKind::ReferenceUsage,
            SymbolKind::OccurrenceUsage => ElementKind::OccurrenceUsage,
            SymbolKind::UseCaseUsage => ElementKind::UseCaseUsage,
            SymbolKind::IncludeUseCaseUsage => ElementKind::IncludeUseCaseUsage,
            SymbolKind::AnalysisCaseUsage => ElementKind::AnalysisCaseUsage,
            SymbolKind::VerificationCaseUsage => ElementKind::VerificationCaseUsage,
            SymbolKind::FlowConnectionUsage => ElementKind::FlowConnectionUsage,
            // Other
            SymbolKind::Import => ElementKind::Import,
            SymbolKind::Comment => ElementKind::Comment,
            SymbolKind::Alias => ElementKind::Alias,
            _ => ElementKind::Other,
        }
    }
}

/// Apply import metadata to symbols in an AnalysisHost.
///
/// This looks up each symbol's qualified name in the metadata and sets
/// the symbol's `element_id` to the original XMI element ID if found.
///
/// Call this after loading decompiled SysML files into the AnalysisHost.
///
/// ## Example
///
/// ```ignore
/// use syster::interchange::{decompile, apply_metadata_to_host};
/// use syster::ide::AnalysisHost;
///
/// // Decompile XMI to SysML + metadata
/// let result = decompile(&model);
///
/// // Parse the SysML text into host
/// let mut host = AnalysisHost::new();
/// host.set_file_content("model.sysml", &result.text);
///
/// // Apply metadata to restore element IDs
/// apply_metadata_to_host(&mut host, &result.metadata);
/// ```
pub fn apply_metadata_to_host(
    host: &mut crate::ide::AnalysisHost,
    metadata: &super::metadata::ImportMetadata,
) {
    use std::sync::Arc;

    // Rebuild index to ensure we have up-to-date symbols
    let _ = host.analysis();

    // Update each symbol's element_id based on metadata lookup
    host.update_symbols(|symbol| {
        if let Some(meta) = metadata.get_element(&symbol.qualified_name) {
            if let Some(id) = &meta.original_id {
                symbol.element_id = Arc::from(id.as_str());
            }
        }
    });
}

/// Result of applying ChangeTracker edits to an AnalysisHost.
#[derive(Debug)]
pub struct ApplyEditsResult {
    /// The new SysML text after edits (rendered via `render_dirty`).
    pub rendered_text: String,
    /// Parse errors from re-parsing the rendered text.
    pub parse_errors: Vec<crate::syntax::parser::ParseError>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::FileId;
    use crate::hir::{FileText, file_symbols_from_text};
    use crate::ide::AnalysisHost;

    #[test]
    fn test_model_from_database_empty() {
        // TDD Step 1: Write a failing test
        // Given an empty database with no files
        let db = RootDatabase::new();

        // When we convert to a model
        let model = model_from_database(&db);

        // Then the model should be empty
        assert!(
            model.elements.is_empty(),
            "Empty database should produce empty model"
        );
        assert!(
            model.roots.is_empty(),
            "Empty database should have no root elements"
        );
        assert_eq!(
            model.relationship_count(),
            0,
            "Empty database should have no relationships"
        );
    }

    #[test]
    fn test_model_from_database_single_package() {
        // Given a database with a single package
        let db = RootDatabase::new();
        let sysml = "package TestPackage;";
        let file_text = FileText::new(&db, FileId::new(0), sysml.to_string());

        // Extract symbols (this populates the database via Salsa queries)
        let symbols = file_symbols_from_text(&db, file_text);
        assert!(!symbols.is_empty(), "Should have parsed the package");

        // When we convert to a model
        let model = model_from_symbols(&symbols);

        // Then the model should have one package element
        assert_eq!(model.elements.len(), 1, "Should have one element");
        assert_eq!(model.roots.len(), 1, "Should have one root element");

        // The element should be a Package with the correct name
        let root_id = &model.roots[0];
        let element = model
            .elements
            .get(root_id)
            .expect("Root element should exist");
        assert_eq!(element.kind, super::super::model::ElementKind::Package);
        assert_eq!(element.name.as_deref(), Some("TestPackage"));
    }

    #[test]
    fn test_model_from_database_with_parts() {
        // Given a database with a package containing part definitions
        let db = RootDatabase::new();
        let sysml = r#"
            package Vehicle {
                part def Car;
                part def Engine;
            }
        "#;
        let file_text = FileText::new(&db, FileId::new(0), sysml.to_string());

        let symbols = file_symbols_from_text(&db, file_text);
        let model = model_from_symbols(&symbols);

        // Should have: Vehicle (package), Car (part def), Engine (part def)
        // Plus 2 membership wrappers (OwningMembership for Car and Engine)
        assert_eq!(model.roots.len(), 1, "Should have one root (Vehicle)");

        // Check content elements via owned_members (looks through memberships)
        let root_id = &model.roots[0];
        let members = model.owned_members(root_id);
        assert_eq!(members.len(), 2, "Vehicle should own 2 content members");

        // Check that Car exists and is a PartDefinition
        let car = model
            .elements
            .values()
            .find(|e| e.name.as_deref() == Some("Car"))
            .expect("Car should exist");
        assert_eq!(car.kind, super::super::model::ElementKind::PartDefinition);
        assert!(car.owner.is_some(), "Car should have an owner");

        // Car's direct owner should be a membership, whose owner is Vehicle
        let direct_owner = model
            .elements
            .get(car.owner.as_ref().unwrap())
            .expect("Direct owner should exist");
        assert!(
            direct_owner.kind.is_membership(),
            "Car's direct owner should be a membership"
        );
        let logical_owner = model
            .elements
            .get(direct_owner.owner.as_ref().unwrap())
            .expect("Logical owner should exist");
        assert_eq!(
            logical_owner.name.as_deref(),
            Some("Vehicle"),
            "Logical owner should be Vehicle"
        );
    }

    #[test]
    fn test_model_from_database_relationships() {
        // Given a database with specialization relationships
        let db = RootDatabase::new();
        let sysml = r#"
            package Types {
                part def Vehicle;
                part def Car :> Vehicle;
            }
        "#;
        let file_text = FileText::new(&db, FileId::new(0), sysml.to_string());

        let symbols = file_symbols_from_text(&db, file_text);
        let model = model_from_symbols(&symbols);

        // Should have relationships
        assert!(model.relationship_count() > 0, "Should have relationships");

        // Find the specialization from Car to Vehicle
        let specialization = model
            .iter_relationship_elements()
            .find(|e| e.kind == super::super::model::ElementKind::Specialization)
            .expect("Should have a specialization");

        // Source and target are in RelationshipData
        let rel_data = specialization.relationship.as_ref().unwrap();
        let source_elem = model
            .elements
            .get(&rel_data.source[0])
            .expect("Source element should exist");
        let target_elem = model
            .elements
            .get(&rel_data.target[0])
            .expect("Target element should exist");

        assert_eq!(
            source_elem.name.as_deref(),
            Some("Car"),
            "Source should be Car"
        );
        assert_eq!(
            target_elem.name.as_deref(),
            Some("Vehicle"),
            "Target should be Vehicle"
        );
    }

    #[test]
    fn test_model_from_symbols_perform_chain_usage_uses_target_name_and_reference_subsetting() {
        let sysml = r#"
            package sample {
                part car {
                    part engine {
                        perform starting.gen_start_cmd;
                    }
                }
                action starting {
                    action gen_start_cmd;
                }
            }
        "#;
        let mut host = AnalysisHost::new();
        let errors = host.set_file_content("test.sysml", sysml);
        assert!(errors.is_empty(), "source should parse cleanly: {errors:?}");
        let analysis = host.analysis();
        let symbols: Vec<_> = analysis.symbol_index().all_symbols().cloned().collect();
        let model = model_from_symbols(&symbols);

        // Slot qname under the anon-scope mangling contract
        // (`hir/symbols/context.rs::next_anon_scope`, authored 2026-02-17):
        // `<{rel_prefix}{target}#{counter}@L{line}>`. A prior codex attempt to
        // have short-form slots inherit their terminal target's short name
        // was reverted (commit d170dc0 "Revert shorthand naming ..."); this
        // test's assertions are realigned to the live mangled contract, in
        // line with sibling tests at lines ~2241/2246.
        let perform_usage = model
            .elements
            .values()
            .find(|e| {
                e.kind == super::super::model::ElementKind::PerformActionUsage
                    && e.qualified_name.as_deref().is_some_and(|qname| {
                        qname.starts_with("sample::car::engine::<perform:starting.gen_start_cmd")
                    })
            })
            .expect("perform usage should exist");

        let relationship = model
            .rel_elements_from(&perform_usage.id)
            .find(|rel| rel.kind == super::super::model::ElementKind::ReferenceSubsetting)
            .expect("perform target reference should be represented as ReferenceSubsetting");

        let target = model
            .rel_target(relationship)
            .expect("perform relationship should resolve to a target element");

        assert_eq!(
            target.qualified_name.as_deref(),
            Some("sample::starting::gen_start_cmd"),
            "perform chain reference subsetting should target the terminal action usage"
        );
    }

    #[test]
    fn test_model_from_symbols_include_chain_relationship_uses_terminal_target() {
        let sysml = r#"
            package sample {
                use case host {
                    include system.uc1;
                }

                part system {
                    use case uc1;
                }
            }
        "#;
        let mut host = AnalysisHost::new();
        let errors = host.set_file_content("test.sysml", sysml);
        assert!(errors.is_empty(), "source should parse cleanly: {errors:?}");
        let analysis = host.analysis();
        let symbols: Vec<_> = analysis.symbol_index().all_symbols().cloned().collect();
        let model = model_from_symbols(&symbols);

        // Mangled anon-scope contract — see perform-chain sibling test above.
        let include_usage = model
            .elements
            .values()
            .find(|e| {
                e.kind == super::super::model::ElementKind::IncludeUseCaseUsage
                    && e.qualified_name
                        .as_deref()
                        .is_some_and(|qname| qname.starts_with("sample::host::<include:system.uc1"))
            })
            .expect("include usage should exist");

        let relationship = model
            .rel_elements_from(&include_usage.id)
            .find(|rel| rel.kind == super::super::model::ElementKind::ReferenceSubsetting)
            .expect("include target reference should be represented as ReferenceSubsetting");

        let target = model
            .rel_target(relationship)
            .expect("include relationship should resolve to a target element");

        assert_eq!(
            target.qualified_name.as_deref(),
            Some("sample::system::uc1"),
            "include chain reference subsetting should target the terminal use case usage"
        );
    }

    #[test]
    fn test_model_from_symbols_exhibit_chain_relationship_uses_terminal_target() {
        let sysml = r#"
            package sample {
                part host {
                    exhibit system.ready;
                }

                part system {
                    state ready;
                }
            }
        "#;
        let mut host = AnalysisHost::new();
        let errors = host.set_file_content("test.sysml", sysml);
        assert!(errors.is_empty(), "source should parse cleanly: {errors:?}");
        let analysis = host.analysis();
        let symbols: Vec<_> = analysis.symbol_index().all_symbols().cloned().collect();
        let model = model_from_symbols(&symbols);

        let exhibit_usage = model
            .elements
            .values()
            .find(|e| {
                e.kind == super::super::model::ElementKind::ExhibitStateUsage
                    && e.qualified_name.as_deref().is_some_and(|qname| {
                        qname.starts_with("sample::host::<exhibit:system.ready")
                    })
            })
            .expect("exhibit usage should exist");

        assert_eq!(
            exhibit_usage.name.as_deref().map(|name| name.starts_with("<exhibit:system.ready#")),
            Some(true),
            "exhibit shorthand local usage should use anonymous helper naming"
        );

        let relationship = model
            .rel_elements_from(&exhibit_usage.id)
            .find(|rel| rel.kind == super::super::model::ElementKind::ReferenceSubsetting)
            .expect("exhibit target reference should be represented as ReferenceSubsetting");

        let target = model
            .rel_target(relationship)
            .expect("exhibit relationship should resolve to a target element");

        assert_eq!(
            target.qualified_name.as_deref(),
            Some("sample::system::ready"),
            "exhibit chain relationship should target the terminal state usage"
        );
    }

    #[test]
    fn test_model_from_symbols_assert_chain_relationship_uses_terminal_target() {
        let sysml = r#"
            package sample {
                part host {
                    assert checks.limit;
                }

                part checks {
                    constraint limit;
                }
            }
        "#;
        let mut host = AnalysisHost::new();
        let errors = host.set_file_content("test.sysml", sysml);
        assert!(errors.is_empty(), "source should parse cleanly: {errors:?}");
        let analysis = host.analysis();
        let symbols: Vec<_> = analysis.symbol_index().all_symbols().cloned().collect();
        let model = model_from_symbols(&symbols);

        let assert_usage = model
            .elements
            .values()
            .find(|e| {
                e.kind == super::super::model::ElementKind::AssertConstraintUsage
                    && e.qualified_name.as_deref().is_some_and(|qname| {
                        qname.starts_with("sample::host::<assert:checks.limit")
                    })
            })
            .expect("assert usage should exist");

        assert_eq!(
            assert_usage.name.as_deref().map(|name| name.starts_with("<assert:checks.limit#")),
            Some(true),
            "assert shorthand local usage should use anonymous helper naming"
        );

        let relationship = model
            .rel_elements_from(&assert_usage.id)
            .find(|rel| rel.kind == super::super::model::ElementKind::ReferenceSubsetting)
            .expect("assert target reference should be represented as ReferenceSubsetting");

        let target = model
            .rel_target(relationship)
            .expect("assert relationship should resolve to a target element");

        assert_eq!(
            target.qualified_name.as_deref(),
            Some("sample::checks::limit"),
            "assert chain relationship should target the terminal constraint usage"
        );
    }

    #[test]
    fn test_model_from_symbols_assume_and_require_chain_relationships_use_terminal_target() {
        // Minimal Group B · short shape (post cleanup per 2026-04-15 plan):
        //   - slot ConstraintUsage (anon-named via naming helper)
        //   - wrap: RequirementConstraintMembership with `kind=requirement|assumption`
        //   - edge: single ReferenceSubsetting from slot → terminal external
        // No `referencedConstraint` / `ownedConstraint` / `ownedMemberName` /
        // etc. aliases on the wrap — those were part of the OMG-API pilot
        // bloat that we reverted.
        let sysml = r#"
            package sample {
                part host {
                    assume checks.assumed;
                    require checks.required;
                }

                part checks {
                    constraint assumed;
                    constraint required;
                }
            }
        "#;
        let mut host = AnalysisHost::new();
        let errors = host.set_file_content("test.sysml", sysml);
        assert!(errors.is_empty(), "source should parse cleanly: {errors:?}");
        let analysis = host.analysis();
        let symbols: Vec<_> = analysis.symbol_index().all_symbols().cloned().collect();
        let model = model_from_symbols(&symbols);

        let assert_b_short_rcm = |slot_qname_prefix: &str,
                                  expected_kind: &str,
                                  expected_terminal: &str| {
            let slot = model
                .elements
                .values()
                .find(|e| {
                    e.qualified_name
                        .as_deref()
                        .is_some_and(|qname| qname.starts_with(slot_qname_prefix))
                })
                .unwrap_or_else(|| {
                    panic!("slot for {slot_qname_prefix} should exist")
                });

            // (A) slot.owner → RCM.
            let rcm_id = slot
                .owner
                .as_ref()
                .unwrap_or_else(|| panic!("slot {slot_qname_prefix} should have an owner"));
            let rcm = model
                .elements
                .get(rcm_id)
                .unwrap_or_else(|| panic!("slot owner {rcm_id:?} should exist"));
            assert_eq!(
                rcm.kind,
                super::super::model::ElementKind::RequirementConstraintMembership,
                "slot {slot_qname_prefix} should be owned by an RCM"
            );

            // (B) RCM carries `kind` property distinguishing requirement/assumption.
            match rcm.properties.get("kind") {
                Some(PropertyValue::String(k)) => assert_eq!(k.as_ref(), expected_kind),
                other => panic!(
                    "RCM for {slot_qname_prefix} should have kind={expected_kind}, got {other:?}"
                ),
            };

            // (C) Slot has a single ReferenceSubsetting edge → terminal external.
            let rs = model
                .rel_elements_from(&slot.id)
                .find(|rel| rel.kind == super::super::model::ElementKind::ReferenceSubsetting)
                .unwrap_or_else(|| {
                    panic!("slot {slot_qname_prefix} should have a ReferenceSubsetting")
                });
            let rs_target = model
                .rel_target(rs)
                .unwrap_or_else(|| panic!("ReferenceSubsetting should resolve a target"));
            assert_eq!(
                rs_target.qualified_name.as_deref(),
                Some(expected_terminal),
                "slot's ReferenceSubsetting should target the terminal constraint"
            );
        };

        assert_b_short_rcm(
            "sample::host::<assume:checks.assumed",
            "assumption",
            "sample::checks::assumed",
        );
        assert_b_short_rcm(
            "sample::host::<require:checks.required",
            "requirement",
            "sample::checks::required",
        );
    }

    #[test]
    fn test_model_from_symbols_satisfy_and_verify_chain_relationships_use_terminal_target() {
        let sysml = r#"
            package sample {
                part verifier;

                part host {
                    satisfy checks.required by verifier;
                    verify checks.verified;
                }

                part checks {
                    requirement required;
                    requirement verified;
                }
            }
        "#;
        let mut host = AnalysisHost::new();
        let errors = host.set_file_content("test.sysml", sysml);
        assert!(errors.is_empty(), "source should parse cleanly: {errors:?}");
        let analysis = host.analysis();
        let symbols: Vec<_> = analysis.symbol_index().all_symbols().cloned().collect();
        let model = model_from_symbols(&symbols);

        let satisfy_usage = model
            .elements
            .values()
            .find(|e| {
                e.kind == super::super::model::ElementKind::Satisfaction
                    && e.qualified_name.as_deref().is_some_and(|qname| {
                        qname.starts_with("sample::host::<satisfy:checks.required")
                    })
            })
            .expect("satisfy usage should exist");

        assert_eq!(
            satisfy_usage
                .name
                .as_deref()
                .map(|name| name.starts_with("<satisfy:checks.required#")),
            Some(true),
            "satisfy shorthand local usage should use anonymous helper naming"
        );

        let satisfy_relationship = model
            .rel_elements_from(&satisfy_usage.id)
            .find(|rel| rel.kind == super::super::model::ElementKind::ReferenceSubsetting)
            .expect("satisfy target reference should be represented as ReferenceSubsetting");

        let satisfy_target = model
            .rel_target(satisfy_relationship)
            .expect("satisfy relationship should resolve to a target element");

        assert_eq!(
            satisfy_target.qualified_name.as_deref(),
            Some("sample::checks::required"),
            "satisfy chain relationship should target the terminal requirement usage"
        );

        // `verify` is a Group B · short form. Minimal shape post-cleanup:
        //   - slot (RequirementUsage here since `verify` of a requirement)
        //   - wrap: Verification membership (serialized as
        //     RequirementVerificationMembership), owner=parent, no extra
        //     properties
        //   - edge: single ReferenceSubsetting from slot → terminal external
        let verify_usage = model
            .elements
            .values()
            .find(|e| {
                e.kind == super::super::model::ElementKind::RequirementUsage
                    && e.qualified_name.as_deref().is_some_and(|qname| {
                        qname.starts_with("sample::host::<verify:checks.verified")
                    })
            })
            .expect("verify usage should exist");

        let verify_wrapper_id = verify_usage
            .owner
            .as_ref()
            .expect("verify slot should have an owner");
        let verify_wrapper = model
            .elements
            .get(verify_wrapper_id)
            .expect("verify slot's owner should exist");
        assert_eq!(
            verify_wrapper.kind,
            super::super::model::ElementKind::Verification,
            "verify slot should be owned by a Verification (RequirementVerificationMembership) element"
        );

        let verify_rs = model
            .rel_elements_from(&verify_usage.id)
            .find(|rel| rel.kind == super::super::model::ElementKind::ReferenceSubsetting)
            .expect("verify slot should carry a ReferenceSubsetting");
        let verify_rs_target = model
            .rel_target(verify_rs)
            .expect("ReferenceSubsetting should resolve target");
        assert_eq!(
            verify_rs_target.qualified_name.as_deref(),
            Some("sample::checks::verified"),
            "verify slot's ReferenceSubsetting should target the terminal requirement"
        );
    }

    #[test]
    fn test_model_from_symbols_special_usage_relationships_use_official_metaclass_types() {
        let sysml = r#"
            package sample {
                part verifier;

                part host {
                    perform actions.starting;
                    exhibit system.ready;
                    assert checks.limit;
                    assume checks.assumed;
                    require checks.required;
                    satisfy reqs.satisfied by verifier;
                    verify reqs.verified;
                }

                use case scenario {
                    include system.uc1;
                }

                action actions {
                    action starting;
                }

                part system {
                    state ready;
                    use case uc1;
                }

                part checks {
                    constraint limit;
                    constraint assumed;
                    constraint required;
                }

                part reqs {
                    requirement satisfied;
                    requirement verified;
                }
            }
        "#;
        let mut host = AnalysisHost::new();
        let errors = host.set_file_content("test.sysml", sysml);
        assert!(errors.is_empty(), "source should parse cleanly: {errors:?}");
        let analysis = host.analysis();
        let symbols: Vec<_> = analysis.symbol_index().all_symbols().cloned().collect();
        let model = model_from_symbols(&symbols);

        let rel_for =
            |qualified_name_prefix: &str, kind: super::super::model::ElementKind| {
            let usage = model
                .elements
                .values()
                .find(|e| {
                    e.kind == kind
                        && e.qualified_name
                            .as_deref()
                            .is_some_and(|qname| qname.starts_with(qualified_name_prefix))
                })
                .unwrap_or_else(|| {
                    panic!("usage {qualified_name_prefix} ({kind:?}) should exist")
                });
            model.rel_elements_from(&usage.id)
                .next()
                .unwrap_or_else(|| {
                    panic!("relationship for {qualified_name_prefix} should exist")
                })
        };

        let perform_usage = model
            .elements
            .values()
            .find(|e| {
                e.qualified_name
                    .as_deref()
                    .is_some_and(|qname| qname.starts_with("sample::host::<perform:actions.starting"))
            })
            .expect("perform usage should exist");
        assert_eq!(
            perform_usage.kind.xsi_type(),
            "sysml:PerformActionUsage"
        );
        assert_eq!(
            model
                .rel_elements_from(&perform_usage.id)
                .next()
                .expect("perform relationship should exist")
                .kind
                .xsi_type(),
            "sysml:ReferenceSubsetting"
        );
        let include_usage = model
            .elements
            .values()
            .find(|e| {
                e.qualified_name
                    .as_deref()
                    .is_some_and(|qname| qname.starts_with("sample::scenario::<include:system.uc1"))
            })
            .expect("include usage should exist");
        assert_eq!(include_usage.kind.xsi_type(), "sysml:IncludeUseCaseUsage");
        assert_eq!(
            model
                .rel_elements_from(&include_usage.id)
                .next()
                .expect("include relationship should exist")
                .kind
                .xsi_type(),
            "sysml:ReferenceSubsetting"
        );
        assert_eq!(
            rel_for(
                "sample::host::<exhibit:system.ready",
                super::super::model::ElementKind::ExhibitStateUsage,
            )
            .kind
            .xsi_type(),
            "sysml:ReferenceSubsetting"
        );
        assert_eq!(
            rel_for(
                "sample::host::<assert:checks.limit",
                super::super::model::ElementKind::AssertConstraintUsage,
            )
            .kind
            .xsi_type(),
            "sysml:ReferenceSubsetting"
        );
        // Group B · short forms (`assume` / `require` / `verify`) emit a
        // specialized membership as the *outer* wrap of the slot (OMG pilot
        // shape); the slot's companion relationship is a ReferenceSubsetting.
        // The specialized membership therefore appears as the slot's owner,
        // not in `rel_elements_from(slot)`. See the helper
        // `emit_group_b_short_specialized_membership` and
        // `docs/conformance/2026-04-15-syster-group-b-short-fix-plan-v2.md`.
        let b_short_wrapper = |slot_qname_prefix: &str,
                               slot_kind: super::super::model::ElementKind|
         -> &Element {
            let slot = model
                .elements
                .values()
                .find(|e| {
                    e.kind == slot_kind
                        && e.qualified_name
                            .as_deref()
                            .is_some_and(|qname| qname.starts_with(slot_qname_prefix))
                })
                .unwrap_or_else(|| {
                    panic!("slot {slot_qname_prefix} ({slot_kind:?}) should exist")
                });
            let owner_id = slot
                .owner
                .as_ref()
                .unwrap_or_else(|| panic!("slot {slot_qname_prefix} should have an owner"));
            model
                .elements
                .get(owner_id)
                .unwrap_or_else(|| panic!("slot owner {owner_id:?} should exist"))
        };

        let assume_rel = b_short_wrapper(
            "sample::host::<assume:checks.assumed",
            super::super::model::ElementKind::ConstraintUsage,
        );
        assert_eq!(
            assume_rel.kind.xsi_type(),
            "sysml:RequirementConstraintMembership"
        );
        let require_rel = b_short_wrapper(
            "sample::host::<require:checks.required",
            super::super::model::ElementKind::ConstraintUsage,
        );
        assert_eq!(
            require_rel.kind.xsi_type(),
            "sysml:RequirementConstraintMembership"
        );
        assert_eq!(
            rel_for(
                "sample::host::<satisfy:reqs.satisfied",
                super::super::model::ElementKind::Satisfaction,
            )
            .kind
            .xsi_type(),
            "sysml:ReferenceSubsetting"
        );
        let verify_rel = b_short_wrapper(
            "sample::host::<verify:reqs.verified",
            super::super::model::ElementKind::RequirementUsage,
        );
        assert_eq!(
            verify_rel.kind.xsi_type(),
            "sysml:RequirementVerificationMembership"
        );

        assert_eq!(
            assume_rel.properties.get("kind"),
            Some(&PropertyValue::String(Arc::from("assumption"))),
            "assume relationship should export RequirementConstraintMembership.kind=assumption"
        );
        assert_eq!(
            require_rel.properties.get("kind"),
            Some(&PropertyValue::String(Arc::from("requirement"))),
            "require relationship should export RequirementConstraintMembership.kind=requirement"
        );
    }

    #[test]
    fn test_model_from_symbols_standard_relationships_preserve_standard_target_properties() {
        use super::super::{ModelFormat, Xmi};

        let sysml = r#"
            package sample {
                part def Vehicle;
                part myCar : Vehicle;
                part def FastCar :> Vehicle;
            }
        "#;
        let mut host = AnalysisHost::new();
        let errors = host.set_file_content("test.sysml", sysml);
        assert!(errors.is_empty(), "source should parse cleanly: {errors:?}");
        let analysis = host.analysis();
        let symbols: Vec<_> = analysis.symbol_index().all_symbols().cloned().collect();
        let model = model_from_symbols(&symbols);

        let vehicle = model
            .elements
            .values()
            .find(|e| e.qualified_name.as_deref() == Some("sample::Vehicle"))
            .expect("Vehicle should exist");

        let my_car = model
            .elements
            .values()
            .find(|e| e.qualified_name.as_deref() == Some("sample::myCar"))
            .expect("myCar should exist");
        let typed_by = model
            .rel_elements_from(&my_car.id)
            .find(|rel| rel.kind == ElementKind::FeatureTyping)
            .expect("myCar should have a FeatureTyping relationship");
        assert_eq!(
            typed_by.properties.get("type"),
            Some(&PropertyValue::Reference(vehicle.id.clone())),
            "FeatureTyping should preserve its standard target property"
        );

        let fast_car = model
            .elements
            .values()
            .find(|e| e.qualified_name.as_deref() == Some("sample::FastCar"))
            .expect("FastCar should exist");
        let specializes = model
            .rel_elements_from(&fast_car.id)
            .find(|rel| rel.kind == ElementKind::Specialization)
            .expect("FastCar should have a Specialization relationship");
        assert_eq!(
            specializes.properties.get("general"),
            Some(&PropertyValue::Reference(vehicle.id.clone())),
            "Specialization should preserve its standard target property"
        );

        let xmi_bytes = Xmi.write(&model).expect("XMI export should succeed");
        let xmi = String::from_utf8(xmi_bytes).expect("XMI should be UTF-8");
        assert!(
            xmi.contains(r#"xsi:type="kerml:FeatureTyping""#) && xmi.contains(r#"type=""#),
            "FeatureTyping XMI should still carry its standard type attribute"
        );
        assert!(
            xmi.contains(r#"xsi:type="kerml:Specialization""#) && xmi.contains(r#"general=""#),
            "Specialization XMI should still carry its standard general attribute"
        );
    }

    #[test]
    fn test_symbols_from_model_roundtrips_explicit_special_usage_relationship_kinds() {
        let sysml = r#"
            package sample {
                part verifier;

                part host {
                    perform actions.starting;
                    exhibit system.ready;
                    assert checks.limit;
                    assume checks.assumed;
                    require checks.required;
                    satisfy reqs.satisfied by verifier;
                    verify reqs.verified;
                }

                use case scenario {
                    include system.uc1;
                }

                action actions {
                    action starting;
                }

                part system {
                    state ready;
                    use case uc1;
                }

                part checks {
                    constraint limit;
                    constraint assumed;
                    constraint required;
                }

                part reqs {
                    requirement satisfied;
                    requirement verified;
                }
            }
        "#;
        let mut host = AnalysisHost::new();
        let errors = host.set_file_content("test.sysml", sysml);
        assert!(errors.is_empty(), "source should parse cleanly: {errors:?}");
        let analysis = host.analysis();
        let symbols: Vec<_> = analysis.symbol_index().all_symbols().cloned().collect();
        let model = model_from_symbols(&symbols);
        let roundtrip_symbols =
            symbols_from_model(&model).expect("round-trip import should succeed");

        let rel_kinds_for =
            |qualified_name_prefix: &str, kind: SymbolKind| -> Vec<HirRelKind> {
            roundtrip_symbols
                .iter()
                .find(|sym| {
                    sym.kind == kind
                        && sym
                            .qualified_name
                            .as_ref()
                            .starts_with(qualified_name_prefix)
                })
                .unwrap_or_else(|| {
                    panic!("round-trip symbol {qualified_name_prefix} ({kind:?}) should exist")
                })
                .relationships
                .iter()
                .map(|rel| rel.kind)
                .collect()
            };

        assert!(
            rel_kinds_for("sample::host::<perform:actions.starting", SymbolKind::PerformActionUsage)
                .contains(&HirRelKind::Performs),
            "perform relationship should survive round-trip"
        );
        assert!(
            rel_kinds_for("sample::scenario::<include:system.uc1", SymbolKind::IncludeUseCaseUsage)
                .contains(&HirRelKind::Includes),
            "include relationship should survive round-trip"
        );
        assert!(
            rel_kinds_for("sample::host::<exhibit:system.ready", SymbolKind::ExhibitStateUsage)
                .contains(&HirRelKind::Exhibits),
            "exhibit relationship should survive round-trip"
        );
        assert!(
            rel_kinds_for("sample::host::<assert:checks.limit", SymbolKind::AssertConstraintUsage)
                .contains(&HirRelKind::Asserts),
            "assert relationship should survive round-trip"
        );
        assert!(
            rel_kinds_for("sample::host::<assume:checks.assumed", SymbolKind::ConstraintUsage)
                .contains(&HirRelKind::Assumes),
            "assume relationship should survive round-trip"
        );
        assert!(
            rel_kinds_for("sample::host::<require:checks.required", SymbolKind::ConstraintUsage)
                .contains(&HirRelKind::Requires),
            "require relationship should survive round-trip"
        );
        assert!(
            rel_kinds_for(
                "sample::host::<satisfy:reqs.satisfied",
                SymbolKind::SatisfyRequirementUsage
            )
                .contains(&HirRelKind::Satisfies),
            "satisfy relationship should survive round-trip"
        );
        assert!(
            rel_kinds_for("sample::host::<verify:reqs.verified", SymbolKind::RequirementUsage)
                .contains(&HirRelKind::Verifies),
            "verify relationship should survive round-trip"
        );
    }

    #[test]
    fn test_model_from_symbols_special_usage_unresolved_chain_target_does_not_fallback_to_raw_name() {
        let db = RootDatabase::new();
        let sysml = r#"
            package sample {
                part host {
                    exhibit missing.ready;
                }
            }
        "#;
        let file_text = FileText::new(&db, FileId::new(0), sysml.to_string());

        let symbols = file_symbols_from_text(&db, file_text);
        let model = model_from_symbols(&symbols);

        let exhibit_usage = model
            .elements
            .values()
            .find(|e| {
                e.qualified_name
                    .as_deref()
                    .is_some_and(|qname| qname.starts_with("sample::host::<exhibit:missing.ready"))
            })
            .expect("exhibit usage should exist");

        assert!(
            model.rel_elements_from(&exhibit_usage.id).next().is_none(),
            "unresolved special-usage target should not fallback to a raw-name relationship"
        );
    }

    #[test]
    fn test_group_b_short_minimal_wrap_shape() {
        // Pins the minimal structural contract for all three Group B · short
        // keywords (`require`, `assume`, `verify`) post-cleanup.
        //
        // For each keyword the emitter must produce:
        //   (A) slot (usage) owned by a specialized membership wrap:
        //       - require/assume → RequirementConstraintMembership (RCM)
        //       - verify         → Verification (serialized as RVM)
        //   (B) wrap.owner = enclosing RequirementUsage/RequirementDefinition
        //   (C) wrap carries ONLY: `owner`, `ownedMember=[slot]`, and for RCM
        //       a `kind="requirement"|"assumption"` property. NO
        //       `referencedConstraint`, `referencedRequirement`,
        //       `ownedConstraint`, `ownedMemberName`, `memberName`,
        //       `membershipOwningNamespace`, `owningType`,
        //       `owningRelatedElement`, `ownedRelatedElement`,
        //       `relatedElement`.
        //   (D) slot carries a single ReferenceSubsetting edge to external:
        //       - source=[slot], target=[external], owner=slot
        //       - NO aliases (`subsettedFeature`, `referencedFeature`,
        //         `general`, `specific`, `subsettingFeature`,
        //         `referencingFeature`, `owningType`, `owningFeature`,
        //         `owningRelatedElement`, `relatedElement`).
        //   (E) parent.owned_elements contains the wrap (not the slot
        //       directly — Phase-6 skip condition).
        use super::super::model::{ElementKind, PropertyValue};
        let sysml = r#"
            package sample {
                part host {
                    assume checks.assumed;
                    require checks.required;
                    verify checks.verified;
                }

                part checks {
                    constraint assumed;
                    constraint required;
                    requirement verified;
                }
            }
        "#;
        let mut host = AnalysisHost::new();
        let errors = host.set_file_content("test.sysml", sysml);
        assert!(errors.is_empty(), "source should parse cleanly: {errors:?}");
        let analysis = host.analysis();
        let symbols: Vec<_> = analysis.symbol_index().all_symbols().cloned().collect();
        let model = model_from_symbols(&symbols);

        let parent = model
            .elements
            .values()
            .find(|e| e.qualified_name.as_deref() == Some("sample::host"))
            .expect("parent `sample::host` should exist");

        // (slot prefix, external qname, wrap kind, kind property or None for Verification)
        let cases: &[(&str, &str, ElementKind, Option<&str>)] = &[
            (
                "sample::host::<assume:checks.assumed",
                "sample::checks::assumed",
                ElementKind::RequirementConstraintMembership,
                Some("assumption"),
            ),
            (
                "sample::host::<require:checks.required",
                "sample::checks::required",
                ElementKind::RequirementConstraintMembership,
                Some("requirement"),
            ),
            (
                "sample::host::<verify:checks.verified",
                "sample::checks::verified",
                ElementKind::Verification,
                None,
            ),
        ];

        // Properties that must NOT appear anywhere on wrap or edge — these are
        // the 20+ OMG-API-pilot aliases we reverted.
        const FORBIDDEN_WRAP_PROPS: &[&str] = &[
            "referencedConstraint",
            "referencedRequirement",
            "ownedConstraint",
            "ownedMemberName",
            "memberName",
            "membershipOwningNamespace",
            "owningType",
            "owningRelatedElement",
            "ownedRelatedElement",
            "relatedElement",
        ];
        const FORBIDDEN_EDGE_PROPS: &[&str] = &[
            "subsettedFeature",
            "referencedFeature",
            "general",
            "specific",
            "subsettingFeature",
            "referencingFeature",
            "owningType",
            "owningFeature",
            "owningRelatedElement",
            "relatedElement",
        ];

        for (slot_qname_prefix, external_qname, carrier_kind, kind_prop) in cases.iter() {
            let slot = model
                .elements
                .values()
                .find(|e| {
                    e.qualified_name
                        .as_deref()
                        .is_some_and(|qname| qname.starts_with(slot_qname_prefix))
                })
                .unwrap_or_else(|| panic!("slot for {slot_qname_prefix} should exist"));
            let external = model
                .elements
                .values()
                .find(|e| e.qualified_name.as_deref() == Some(external_qname))
                .unwrap_or_else(|| panic!("external {external_qname} should exist"));

            // (A) slot.owner → wrap of expected kind.
            let wrap_id = slot
                .owner
                .as_ref()
                .unwrap_or_else(|| panic!("slot {slot_qname_prefix} must have an owner"));
            let wrap = model
                .elements
                .get(wrap_id)
                .unwrap_or_else(|| panic!("wrap {wrap_id:?} must exist"));
            assert_eq!(
                wrap.kind, *carrier_kind,
                "slot {slot_qname_prefix} must be owned by {carrier_kind:?}"
            );

            // (B) wrap.owner = parent.
            assert_eq!(
                wrap.owner.as_ref(),
                Some(&parent.id),
                "wrap for {slot_qname_prefix} must be owned by the enclosing parent"
            );

            // (C) wrap carries `kind` iff RCM; no other property aliases.
            match (kind_prop, wrap.properties.get("kind")) {
                (Some(expected), Some(PropertyValue::String(k))) => {
                    assert_eq!(
                        k.as_ref(),
                        *expected,
                        "RCM.kind for {slot_qname_prefix} must equal {expected:?}"
                    );
                }
                (Some(expected), other) => panic!(
                    "RCM for {slot_qname_prefix} must carry kind={expected:?}, got {other:?}"
                ),
                (None, None) => {}
                (None, Some(other)) => panic!(
                    "Verification for {slot_qname_prefix} must NOT carry kind property, got {other:?}"
                ),
            }
            for prop in FORBIDDEN_WRAP_PROPS {
                assert!(
                    wrap.properties.get(*prop).is_none(),
                    "wrap for {slot_qname_prefix} must NOT carry {prop} (OMG-API alias)"
                );
            }

            // wrap.ownedMember must be exactly [slot] — the wrap IS the
            // membership, its sole member is the slot.
            assert_eq!(
                wrap.owned_elements.as_slice(),
                &[slot.id.clone()],
                "wrap.owned_elements for {slot_qname_prefix} must be exactly [slot]"
            );

            // (D) slot has exactly one ReferenceSubsetting edge → external,
            // with no aliases.
            let edges: Vec<_> = model
                .rel_elements_from(&slot.id)
                .filter(|rel| rel.kind == ElementKind::ReferenceSubsetting)
                .collect();
            assert_eq!(
                edges.len(),
                1,
                "slot {slot_qname_prefix} must have exactly one ReferenceSubsetting edge"
            );
            let rs = edges[0];
            assert_eq!(
                rs.source(),
                Some(&slot.id),
                "RS for {slot_qname_prefix} must have source=[slot]"
            );
            assert_eq!(
                rs.target(),
                Some(&external.id),
                "RS for {slot_qname_prefix} must have target=[external]"
            );
            assert_eq!(
                rs.owner.as_ref(),
                Some(&slot.id),
                "RS for {slot_qname_prefix} must be owned by the slot"
            );
            for prop in FORBIDDEN_EDGE_PROPS {
                assert!(
                    rs.properties.get(*prop).is_none(),
                    "RS for {slot_qname_prefix} must NOT carry {prop} (OMG-API alias)"
                );
            }

            // (E) parent.owned_elements contains the wrap, not the slot.
            assert!(
                parent.owned_elements.contains(wrap_id),
                "parent.owned_elements must list the wrap for {slot_qname_prefix}"
            );
            assert!(
                !parent.owned_elements.contains(&slot.id),
                "parent.owned_elements must NOT list the slot directly for {slot_qname_prefix}"
            );
        }
    }

    #[test]
    fn test_jsonld_perform_uses_specialized_local_usage_and_reference_subsetting() {
        use crate::interchange::{JsonLd, ModelFormat};
        use serde_json::Value;

        let sysml = r#"
            package sample {
                part host {
                    perform actions.starting;
                }

                action actions {
                    action starting;
                }
            }
        "#;
        let mut host = AnalysisHost::new();
        let errors = host.set_file_content("test.sysml", sysml);
        assert!(errors.is_empty(), "source should parse cleanly: {errors:?}");
        let analysis = host.analysis();
        let symbols: Vec<_> = analysis.symbol_index().all_symbols().cloned().collect();
        let model = model_from_symbols(&symbols);
        let json_bytes = JsonLd.write(&model).expect("JSON-LD export should succeed");
        let json: Value = serde_json::from_slice(&json_bytes).expect("JSON-LD should be valid");
        let items = json
            .as_array()
            .expect("perform export should serialize as an array");

        let perform_usage = items
            .iter()
            .find(|item| {
                item.get("@type") == Some(&Value::String("PerformActionUsage".into()))
                    && item
                        .get("qualifiedName")
                        .and_then(Value::as_str)
                        .is_some_and(|qname| {
                            qname.starts_with("sample::host::<perform:actions.starting")
                        })
            })
            .expect("perform local usage should be exported as PerformActionUsage");

        assert_eq!(
            perform_usage
                .get("name")
                .and_then(Value::as_str)
                .map(|name| name.starts_with("<perform:actions.starting#")),
            Some(true),
            "perform shorthand local usage should keep anonymous helper naming"
        );

        let perform_usage_id = perform_usage
            .get("@id")
            .and_then(Value::as_str)
            .expect("perform local usage should have an id");

        let reference_subsetting = items
            .iter()
            .find(|item| {
                item.get("@type") == Some(&Value::String("ReferenceSubsetting".into()))
                    && item.get("source")
                        == Some(&serde_json::json!({"@id": perform_usage_id}))
            })
            .expect("perform target reference should be exported as ReferenceSubsetting");

        assert!(
            reference_subsetting.get("target").is_some_and(Value::is_object),
            "perform ReferenceSubsetting should continue using generic target references"
        );
    }

    #[test]
    fn test_symbols_from_model_reports_error_on_requirement_constraint_membership_without_kind() {
        let mut model = Model::new();

        model.add_element(Element::new("src", ElementKind::ConstraintUsage).with_name("source"));
        model.add_element(Element::new("tgt", ElementKind::ConstraintUsage).with_name("target"));

        model.add_rel(
            "rel1",
            ElementKind::RequirementConstraintMembership,
            "src",
            "tgt",
            Some(ElementId::new("src")),
        );

        let err = symbols_from_model(&model)
            .expect_err("RequirementConstraintMembership without kind should fail clearly");
        assert!(
            err.to_string()
                .contains("RequirementConstraintMembership missing valid kind"),
            "error should explain the missing RequirementConstraintMembership.kind: {err}"
        );
    }

    #[test]
    fn test_roundtrip_through_xmi() {
        use super::super::{ModelFormat, Xmi};

        // Given a database with a simple model (just the root package)
        let db = RootDatabase::new();
        let sysml = "package Vehicles;";
        let file_text = FileText::new(&db, FileId::new(0), sysml.to_string());

        let symbols = file_symbols_from_text(&db, file_text);
        let model = model_from_symbols(&symbols);

        // Verify our model has what we expect
        assert_eq!(model.elements.len(), 1, "Should have one package");

        // Capture the original element ID
        let original_id = model.elements.keys().next().unwrap().clone();

        // When we write to XMI and read back
        let xmi_bytes = Xmi.write(&model).expect("Should write XMI");
        let roundtrip_model = Xmi.read(&xmi_bytes).expect("Should read XMI");

        // Then we should find our original element
        assert!(
            !roundtrip_model.elements.is_empty(),
            "Should have at least one element after roundtrip"
        );
        assert!(
            roundtrip_model.elements.contains_key(&original_id),
            "Should find the original element by ID after roundtrip"
        );
    }

    // ========== symbols_from_model() tests ==========

    #[test]
    fn test_symbols_from_empty_model() {
        // Given an empty model
        let model = Model::new();

        // When we convert to symbols
        let symbols = symbols_from_model(&model).expect("empty model import should succeed");

        // Then we should get no symbols
        assert!(symbols.is_empty(), "Empty model should produce no symbols");
    }

    #[test]
    fn test_symbols_from_model_single_package() {
        // Given a model with a single package
        let mut model = Model::new();
        let pkg = Element::new(ElementId::new("TestPackage"), ElementKind::Package)
            .with_name("TestPackage");
        model.add_element(pkg);

        // When we convert to symbols
        let symbols = symbols_from_model(&model).expect("package import should succeed");

        // Then we should get one symbol
        assert_eq!(symbols.len(), 1, "Should have one symbol");
        assert_eq!(symbols[0].name.as_ref(), "TestPackage");
        assert_eq!(symbols[0].kind, SymbolKind::Package);
        assert_eq!(symbols[0].qualified_name.as_ref(), "TestPackage");
    }

    #[test]
    fn test_symbols_from_model_with_part_definitions() {
        // Given a model with part definitions
        let mut model = Model::new();

        let pkg =
            Element::new(ElementId::new("Vehicle"), ElementKind::Package).with_name("Vehicle");
        model.add_element(pkg);

        let car = Element::new(ElementId::new("Vehicle::Car"), ElementKind::PartDefinition)
            .with_name("Car")
            .with_owner(ElementId::new("Vehicle"));
        model.add_element(car);

        let engine = Element::new(
            ElementId::new("Vehicle::Engine"),
            ElementKind::PartDefinition,
        )
        .with_name("Engine")
        .with_owner(ElementId::new("Vehicle"));
        model.add_element(engine);

        // When we convert to symbols
        let symbols = symbols_from_model(&model).expect("definition import should succeed");

        // Then we should get 3 symbols with correct kinds
        assert_eq!(symbols.len(), 3, "Should have 3 symbols");

        let car_sym = symbols
            .iter()
            .find(|s| s.name.as_ref() == "Car")
            .expect("Should have Car");
        assert_eq!(car_sym.kind, SymbolKind::PartDefinition);
        // Note: Without qualified_name set in the Element, this falls back to just the name
        assert_eq!(car_sym.qualified_name.as_ref(), "Car");

        let engine_sym = symbols
            .iter()
            .find(|s| s.name.as_ref() == "Engine")
            .expect("Should have Engine");
        assert_eq!(engine_sym.kind, SymbolKind::PartDefinition);
    }

    #[test]
    fn test_symbols_from_model_with_relationships() {
        // Given a model with specialization: Car :> Vehicle
        let mut model = Model::new();

        let vehicle = Element::new(ElementId::new("Vehicle"), ElementKind::PartDefinition)
            .with_name("Vehicle");
        model.add_element(vehicle);

        let car = Element::new(ElementId::new("Car"), ElementKind::PartDefinition).with_name("Car");
        model.add_element(car);

        // Add specialization relationship
        model.add_rel(
            ElementId::new("rel_1"),
            ElementKind::Specialization,
            ElementId::new("Car"),
            ElementId::new("Vehicle"),
            None,
        );

        // When we convert to symbols
        let symbols = symbols_from_model(&model).expect("relationship import should succeed");

        // Then Car should have a specialization relationship
        let car_sym = symbols
            .iter()
            .find(|s| s.name.as_ref() == "Car")
            .expect("Should have Car");
        assert!(
            !car_sym.relationships.is_empty(),
            "Car should have relationships"
        );

        let spec_rel = car_sym
            .relationships
            .iter()
            .find(|r| r.kind == HirRelKind::Specializes)
            .expect("Should have specialization");
        assert_eq!(spec_rel.target.as_ref(), "Vehicle");

        // Should also be in supertypes
        assert!(car_sym.supertypes.iter().any(|s| s.as_ref() == "Vehicle"));
    }

    #[test]
    fn test_symbols_from_model_with_documentation() {
        // Given a model with documented element
        let mut model = Model::new();

        let mut pkg =
            Element::new(ElementId::new("MyPackage"), ElementKind::Package).with_name("MyPackage");
        pkg.documentation = Some("This is a documented package".into());
        model.add_element(pkg);

        // When we convert to symbols
        let symbols = symbols_from_model(&model).expect("documentation import should succeed");

        // Then the symbol should have documentation
        assert_eq!(symbols.len(), 1);
        assert_eq!(
            symbols[0].doc.as_deref(),
            Some("This is a documented package")
        );
    }

    #[test]
    fn test_symbols_from_model_roundtrip() {
        // Given: Parse SysML → Model → Symbols → Model → Symbols
        // The symbol counts should match
        let db = RootDatabase::new();
        let sysml = r#"
            package Types {
                part def Vehicle;
                part def Car :> Vehicle;
            }
        "#;
        let file_text = FileText::new(&db, FileId::new(0), sysml.to_string());

        // SysML → HirSymbols
        let original_symbols = file_symbols_from_text(&db, file_text);

        // HirSymbols → Model
        let model = model_from_symbols(&original_symbols);

        // Model → HirSymbols (the new function)
        let roundtrip_symbols = symbols_from_model(&model).expect("round-trip import should succeed");

        // Should have same number of non-relationship symbols
        let original_count = original_symbols.len();
        let roundtrip_count = roundtrip_symbols.len();

        assert_eq!(
            roundtrip_count, original_count,
            "Roundtrip should preserve symbol count: {} → {}",
            original_count, roundtrip_count
        );

        // Names should match
        for orig in &original_symbols {
            let found = roundtrip_symbols
                .iter()
                .find(|s| s.qualified_name == orig.qualified_name);
            assert!(
                found.is_some(),
                "Symbol {} should exist after roundtrip",
                orig.qualified_name
            );
        }
    }

    #[test]
    fn test_model_roundtrip_preserves_usage_flags_and_is_composite_output() {
        use super::super::{JsonLd, ModelFormat, Xmi};

        let db = RootDatabase::new();
        let sysml = r#"
            package Modifiers {
                composite part assembly;
                part def Vehicle {
                    composite part wheel;
                    attribute values[*] nonunique;
                }
                portion part slice;
                action def Control {
                    in port inputPort;
                }
            }
        "#;
        let file_text = FileText::new(&db, FileId::new(0), sysml.to_string());

        let original_symbols = file_symbols_from_text(&db, file_text);
        let model = model_from_symbols(&original_symbols);

        let wheel = model
            .elements
            .values()
            .find(|e| e.name.as_deref() == Some("wheel"))
            .expect("wheel element should exist");
        assert_eq!(
            wheel.properties.get("isComposite"),
            Some(&PropertyValue::Boolean(true))
        );

        let assembly = model
            .elements
            .values()
            .find(|e| e.name.as_deref() == Some("assembly"))
            .expect("assembly element should exist");
        assert_eq!(
            assembly.properties.get("isComposite"),
            Some(&PropertyValue::Boolean(false))
        );

        let xmi_bytes = Xmi.write(&model).expect("Should write XMI");
        let xmi = String::from_utf8(xmi_bytes).expect("XMI should be UTF-8");
        assert!(xmi.contains("isComposite=\"true\""));
        assert!(xmi.contains("isComposite=\"false\""));

        let jsonld_bytes = JsonLd.write(&model).expect("Should write JSON-LD");
        let jsonld = String::from_utf8(jsonld_bytes).expect("JSON-LD should be UTF-8");
        assert!(jsonld.contains("\"isComposite\": true"));
        assert!(jsonld.contains("\"isComposite\": false"));

        let roundtrip_symbols =
            symbols_from_model(&model).expect("round-trip import should succeed");

        let wheel_symbol = roundtrip_symbols
            .iter()
            .find(|s| s.qualified_name.as_ref() == "Modifiers::Vehicle::wheel")
            .expect("wheel symbol should exist after roundtrip");
        assert_eq!(wheel_symbol.is_composite, Some(true));
        let slice_symbol = roundtrip_symbols
            .iter()
            .find(|s| s.qualified_name.as_ref() == "Modifiers::slice")
            .expect("slice symbol should exist after roundtrip");
        assert!(slice_symbol.is_portion);
        assert_eq!(slice_symbol.is_composite, Some(false));

        let values_symbol = roundtrip_symbols
            .iter()
            .find(|s| s.qualified_name.as_ref() == "Modifiers::Vehicle::values")
            .expect("values symbol should exist after roundtrip");
        assert!(values_symbol.is_nonunique);

        let input_port_symbol = roundtrip_symbols
            .iter()
            .find(|s| s.qualified_name.as_ref() == "Modifiers::Control::inputPort")
            .expect("inputPort symbol should exist after roundtrip");
        assert_eq!(input_port_symbol.direction, Some(Direction::In));
    }

    #[test]
    fn test_model_from_symbols_only_writes_is_composite_for_feature_kinds() {
        let db = RootDatabase::new();
        let sysml = r#"
            package sample {
                part def A {
                    part def B;
                    part p;
                }
            }
        "#;
        let file_text = FileText::new(&db, FileId::new(0), sysml.to_string());

        let original_symbols = file_symbols_from_text(&db, file_text);
        let model = model_from_symbols(&original_symbols);

        let definition = model
            .elements
            .values()
            .find(|e| e.qualified_name.as_deref() == Some("sample::A::B"))
            .expect("nested definition should exist");
        assert!(!definition.kind.is_feature_kind());
        assert!(definition.properties.get("isComposite").is_none());

        let usage = model
            .elements
            .values()
            .find(|e| e.qualified_name.as_deref() == Some("sample::A::p"))
            .expect("part usage should exist");
        assert!(usage.kind.is_feature_kind());
        assert_eq!(
            usage.properties.get("isComposite"),
            Some(&PropertyValue::Boolean(true))
        );
    }

    #[test]
    fn test_apply_metadata_to_host() {
        use crate::ide::AnalysisHost;
        use crate::interchange::integrate::apply_metadata_to_host;
        use crate::interchange::metadata::{ElementMeta, ImportMetadata};

        // Create a host with a simple SysML file
        let mut host = AnalysisHost::new();
        let sysml = r#"
package TestPkg {
    part def Car;
}
"#;
        host.set_file_content("/test.sysml", sysml);

        // Create metadata with element IDs
        let mut metadata = ImportMetadata::new();
        metadata.add_element("TestPkg", ElementMeta::with_id("uuid-pkg-1"));
        metadata.add_element("TestPkg::Car", ElementMeta::with_id("uuid-car-1"));

        // Apply metadata to host
        apply_metadata_to_host(&mut host, &metadata);

        // Verify element IDs were applied
        let analysis = host.analysis();

        let pkg = analysis
            .symbol_index()
            .lookup_qualified("TestPkg")
            .expect("Should find TestPkg");
        assert_eq!(
            pkg.element_id.as_ref(),
            "uuid-pkg-1",
            "Package should have metadata element_id"
        );

        let car = analysis
            .symbol_index()
            .lookup_qualified("TestPkg::Car")
            .expect("Should find TestPkg::Car");
        assert_eq!(
            car.element_id.as_ref(),
            "uuid-car-1",
            "Car should have metadata element_id"
        );
    }
}
