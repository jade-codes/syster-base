use super::*;

impl NormalizedPackage {
    pub(super) fn from_rowan(pkg: &RowanPackage) -> Self {
        Self {
            name: pkg.name().and_then(|n| n.text()),
            short_name: pkg
                .name()
                .and_then(|n| n.short_name())
                .and_then(|sn| sn.text()),
            range: Some(pkg.syntax().text_range()),
            name_range: pkg.name().map(|n| n.syntax().text_range()),
            doc: parser::extract_doc_comment(pkg.syntax()),
            children: pkg
                .body()
                .map(|b| {
                    b.members()
                        .map(|m| NormalizedElement::from_rowan(&m))
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}

impl NormalizedDefinition {
    pub(super) fn from_rowan(def: &RowanDefinition) -> Self {
        let kind = match def.definition_kind() {
            Some(RowanDefinitionKind::Part) => NormalizedDefKind::Part,
            Some(RowanDefinitionKind::Item) => NormalizedDefKind::Item,
            Some(RowanDefinitionKind::Action) => NormalizedDefKind::Action,
            Some(RowanDefinitionKind::Port) => NormalizedDefKind::Port,
            Some(RowanDefinitionKind::Attribute) => NormalizedDefKind::Attribute,
            Some(RowanDefinitionKind::Connection) => NormalizedDefKind::Connection,
            Some(RowanDefinitionKind::Interface) => NormalizedDefKind::Interface,
            Some(RowanDefinitionKind::Allocation) => NormalizedDefKind::Allocation,
            Some(RowanDefinitionKind::Requirement) => NormalizedDefKind::Requirement,
            Some(RowanDefinitionKind::Constraint) => NormalizedDefKind::Constraint,
            Some(RowanDefinitionKind::State) => NormalizedDefKind::State,
            Some(RowanDefinitionKind::Calc) => NormalizedDefKind::Calculation,
            Some(RowanDefinitionKind::Case) | Some(RowanDefinitionKind::UseCase) => {
                NormalizedDefKind::UseCase
            }
            Some(RowanDefinitionKind::Analysis) | Some(RowanDefinitionKind::Verification) => {
                NormalizedDefKind::AnalysisCase
            }
            Some(RowanDefinitionKind::Concern) => NormalizedDefKind::Concern,
            Some(RowanDefinitionKind::View) => NormalizedDefKind::View,
            Some(RowanDefinitionKind::Viewpoint) => NormalizedDefKind::Viewpoint,
            Some(RowanDefinitionKind::Rendering) => NormalizedDefKind::Rendering,
            Some(RowanDefinitionKind::Enum) => NormalizedDefKind::Enumeration,
            Some(RowanDefinitionKind::Flow) => NormalizedDefKind::Other, // Map flow def to Other
            Some(RowanDefinitionKind::Metadata) => NormalizedDefKind::Other,
            Some(RowanDefinitionKind::Occurrence) => NormalizedDefKind::Other,
            // KerML mappings to SysML equivalents
            Some(RowanDefinitionKind::Class) => NormalizedDefKind::Part, // class -> part def
            Some(RowanDefinitionKind::Struct) => NormalizedDefKind::Part, // struct -> part def
            Some(RowanDefinitionKind::Datatype) => NormalizedDefKind::Attribute, // datatype -> attribute def
            Some(RowanDefinitionKind::Assoc) => NormalizedDefKind::Connection, // assoc -> connection def
            Some(RowanDefinitionKind::Behavior) => NormalizedDefKind::Action, // behavior -> action def
            Some(RowanDefinitionKind::Function) => NormalizedDefKind::Calculation, // function -> calc def
            Some(RowanDefinitionKind::Predicate) => NormalizedDefKind::Constraint, // predicate -> constraint def
            Some(RowanDefinitionKind::Interaction) => NormalizedDefKind::Action, // interaction -> action def
            Some(RowanDefinitionKind::Classifier) => NormalizedDefKind::Part, // classifier -> part def
            Some(RowanDefinitionKind::Type) => NormalizedDefKind::Other,      // type -> other
            Some(RowanDefinitionKind::Metaclass) => NormalizedDefKind::Metaclass, // metaclass -> metaclass
            None => NormalizedDefKind::Other,
        };

        // Extract relationships from specializations
        let mut relationships: Vec<NormalizedRelationship> = def
            .specializations()
            .filter_map(|spec| {
                // If kind is None but target exists, it's a comma-separated continuation
                // Default to Specializes since `:> A, B, C` means A, B, C all specialize
                let rel_kind = match spec.kind() {
                    Some(SpecializationKind::Specializes) => NormalizedRelKind::Specializes,
                    Some(SpecializationKind::Subsets) => NormalizedRelKind::Subsets,
                    Some(SpecializationKind::Redefines) => NormalizedRelKind::Redefines,
                    Some(SpecializationKind::References) => NormalizedRelKind::References,
                    Some(SpecializationKind::Conjugates) => NormalizedRelKind::Specializes,
                    Some(SpecializationKind::FeatureChain) => NormalizedRelKind::Specializes,
                    None => NormalizedRelKind::Specializes, // Comma-continuation inherits Specializes
                };
                let target_node = spec.target()?;
                let target = target_node.to_string();
                Some(NormalizedRelationship {
                    kind: rel_kind,
                    target: RelTarget::Simple(target),
                    range: Some(target_node.syntax().text_range()),
                })
            })
            .collect();

        // Extract expression references from ALL expressions in this definition
        // (e.g., constraint def bodies)
        // IMPORTANT: Only extract expressions that are NOT inside nested scopes
        // to avoid duplicate extraction - children will extract their own expressions
        for expr in def.descendants::<Expression>() {
            // Skip expressions that are inside a nested scope
            let mut is_in_nested_scope = false;
            let mut ancestor = expr.syntax().parent();
            let def_syntax = def.syntax();
            while let Some(ref node) = ancestor {
                // Stop when we reach our own def node
                if node.text_range().start() == def_syntax.text_range().start() {
                    break;
                }
                // If we hit any USAGE/DEFINITION before reaching our own node,
                // this expression belongs to a nested scope
                let is_boundary = matches!(
                    node.kind(),
                    crate::parser::SyntaxKind::NAMESPACE_BODY
                        | crate::parser::SyntaxKind::USAGE
                        | crate::parser::SyntaxKind::DEFINITION
                );
                if is_boundary {
                    is_in_nested_scope = true;
                    break;
                }
                ancestor = node.parent();
            }
            if is_in_nested_scope {
                continue;
            }

            extract_expression_chains(&expr, &mut relationships);
        }

        // Extract prefix metadata (#name) as Meta relationships
        // PREFIX_METADATA nodes are preceding siblings, not children of DEFINITION
        for prefix_meta in def.prefix_metadata() {
            if let (Some(name), Some(range)) = (prefix_meta.name(), prefix_meta.name_range()) {
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::Meta,
                    target: RelTarget::Simple(name),
                    range: Some(range),
                });
            }
        }

        // Extract children from body
        // Try NAMESPACE_BODY first, then CONSTRAINT_BODY (for constraint/calc defs)
        let children: Vec<NormalizedElement> = def
            .body()
            .map(|b| {
                b.members()
                    .map(|m| NormalizedElement::from_rowan(&m))
                    .collect()
            })
            .or_else(|| {
                def.constraint_body().map(|cb| {
                    cb.members()
                        .map(|m| NormalizedElement::from_rowan(&m))
                        .collect()
                })
            })
            .unwrap_or_default();

        Self {
            name: def.name().and_then(|n| n.text()),
            short_name: def
                .name()
                .and_then(|n| n.short_name())
                .and_then(|sn| sn.text()),
            kind,
            range: Some(def.syntax().text_range()),
            name_range: def.name().map(|n| n.syntax().text_range()),
            short_name_range: def
                .name()
                .and_then(|n| n.short_name())
                .map(|sn| sn.syntax().text_range()),
            doc: parser::extract_doc_comment(def.syntax()),
            relationships,
            children,
            is_abstract: def.is_abstract(),
            is_variation: def.is_variation(),
            is_individual: def.is_individual(),
        }
    }
}

