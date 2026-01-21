use crate::semantic::symbol_table::Symbol;
use crate::semantic::types::TokenType;
use crate::syntax::sysml::ast::enums::{DefinitionMember, UsageKind};
use crate::syntax::sysml::ast::types::{Dependency, PerformRel, RedefinitionRel, SubsettingRel};
use crate::syntax::sysml::ast::{
    Alias, Comment, Definition, Import, NamespaceDeclaration, Package, Usage,
};
use crate::syntax::sysml::visitor::AstVisitor;
use tracing::trace;

use crate::semantic::adapters::SysmlAdapter;

/// Strip single quotes from a string if present.
/// E.g., "'Robotic Vacuum Cleaner'" -> "Robotic Vacuum Cleaner"
fn strip_quotes(s: &str) -> String {
    if s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2 {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

/// Strip quotes from each part of a qualified name.
/// E.g., "'Robotic Vacuum Cleaner'::*" -> "Robotic Vacuum Cleaner::*"
fn strip_qualified_name_quotes(s: &str) -> String {
    s.split("::")
        .map(|part| strip_quotes(part.trim()))
        .collect::<Vec<_>>()
        .join("::")
}

/// Get the implicit supertype for a definition kind.
/// In SysML v2, definitions without explicit specialization implicitly specialize
/// a base type from the standard library (e.g., `part def X` implicitly specializes `Parts::Part`).
fn implicit_supertype_for_kind(kind: &str) -> Option<&'static str> {
    match kind {
        "Part" => Some("Parts::Part"),
        "Item" => Some("Items::Item"),
        "Action" => Some("Actions::Action"),
        "Attribute" => Some("Attributes::Attribute"),
        "Connection" => Some("Connections::Connection"),
        "Interface" => Some("Interfaces::Interface"),
        "Port" => Some("Ports::Port"),
        "Allocation" => Some("Allocations::Allocation"),
        "Requirement" => Some("Requirements::Requirement"),
        "Constraint" => Some("Constraints::Constraint"),
        "State" => Some("States::StateAction"),
        "Calculation" => Some("Calculations::Calculation"),
        "Case" | "UseCase" => Some("Cases::Case"),
        "AnalysisCase" => Some("AnalysisCases::AnalysisCase"),
        "Flow" => Some("Flows::FlowConnection"),
        "Occurrence" => Some("Occurrences::Occurrence"),
        _ => None,
    }
}

/// Extract relationship targets from a list of relationships.
/// For feature chains (where chain_context is set), reconstructs the full chain.
/// E.g., [("localClock", Some((["localClock", "currentTime"], 0))), ("currentTime", Some((["localClock", "currentTime"], 1)))]
/// becomes ["localClock.currentTime"]
fn extract_relationship_targets<T: HasTargetAndChain>(rels: &[T]) -> Vec<String> {
    let mut result = Vec::new();
    let mut processed_chains = std::collections::HashSet::new();
    
    for rel in rels {
        if let Some((chain_parts, _)) = rel.chain_context() {
            // This is part of a feature chain - reconstruct and dedupe
            let full_chain = chain_parts.join(".");
            if processed_chains.insert(full_chain.clone()) {
                result.push(full_chain);
            }
        } else {
            // Regular single target
            result.push(rel.target().to_string());
        }
    }
    result
}

/// Trait for relationship types that have a target and optional chain context
trait HasTargetAndChain {
    fn target(&self) -> &str;
    fn chain_context(&self) -> Option<&(Vec<String>, usize)>;
}

impl HasTargetAndChain for SubsettingRel {
    fn target(&self) -> &str { &self.target }
    fn chain_context(&self) -> Option<&(Vec<String>, usize)> { self.chain_context.as_ref() }
}

impl HasTargetAndChain for RedefinitionRel {
    fn target(&self) -> &str { &self.target }
    fn chain_context(&self) -> Option<&(Vec<String>, usize)> { self.chain_context.as_ref() }
}

impl HasTargetAndChain for PerformRel {
    fn target(&self) -> &str { &self.target }
    fn chain_context(&self) -> Option<&(Vec<String>, usize)> { self.chain_context.as_ref() }
}

/// Extract documentation from a definition body.
/// Returns the content of the first `doc /* ... */` comment found.
fn extract_doc_from_definition(body: &[DefinitionMember]) -> Option<String> {
    for member in body {
        if let DefinitionMember::Comment(comment) = member {
            let content = comment.content.trim();
            // Check if it's a doc comment (starts with "doc")
            if content.starts_with("doc") {
                // Extract the comment text from "doc /* text */" or "doc text;"
                return Some(extract_doc_text(content));
            }
        }
    }
    None
}

/// Extract documentation from a usage body.
fn extract_doc_from_usage(
    body: &[crate::syntax::sysml::ast::enums::UsageMember],
) -> Option<String> {
    for member in body {
        if let crate::syntax::sysml::ast::enums::UsageMember::Comment(comment) = member {
            let content = comment.content.trim();
            if content.starts_with("doc") {
                return Some(extract_doc_text(content));
            }
        }
    }
    None
}

/// Extract the text content from a doc comment.
/// Handles: "doc /* text */", "doc name /* text */", "doc;"
fn extract_doc_text(doc: &str) -> String {
    // Remove "doc" prefix
    let rest = doc.strip_prefix("doc").unwrap_or(doc).trim();

    // Try to extract content from /* ... */
    if let Some(start) = rest.find("/*") {
        if let Some(end) = rest.rfind("*/") {
            let inner = &rest[start + 2..end];
            // Clean up the text: trim whitespace and asterisks from each line
            return inner
                .lines()
                .map(|line| line.trim().trim_start_matches('*').trim())
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
        }
    }

    // No block comment, return empty or the identifier
    String::new()
}

impl<'a> AstVisitor for SysmlAdapter<'a> {
    fn visit_namespace(&mut self, namespace: &NamespaceDeclaration) {
        let qualified_name = self.qualified_name(&namespace.name);
        let scope_id = self.symbol_table.current_scope_id();
        let current_file = self.symbol_table.current_file().map(String::from);
        let symbol = Symbol::Package {
            name: namespace.name.clone(),
            qualified_name,
            scope_id,
            source_file: current_file,
            span: namespace.span,
            documentation: None, // Namespace declarations don't have doc bodies
        };
        self.insert_symbol(namespace.name.clone(), symbol);
        self.enter_namespace(namespace.name.clone());
    }

    fn visit_package(&mut self, package: &Package) {
        if let Some(name) = &package.name {
            let qualified_name = self.qualified_name(name);
            let scope_id = self.symbol_table.current_scope_id();
            let source_file = self.symbol_table.current_file().map(String::from);
            let symbol = Symbol::Package {
                name: name.clone(),
                qualified_name,
                scope_id,
                source_file,
                span: package.span,
                documentation: None, // TODO: extract from package elements if needed
            };
            self.insert_symbol(name.clone(), symbol);
            self.enter_namespace(name.clone());
        }
    }

    fn visit_definition(&mut self, definition: &Definition) {
        if let Some(name) = &definition.name {
            let qualified_name = self.qualified_name(name);
            let kind = Self::map_definition_kind(&definition.kind);
            let semantic_role = Self::definition_kind_to_semantic_role(&definition.kind);
            let scope_id = self.symbol_table.current_scope_id();
            let documentation = extract_doc_from_definition(&definition.body);

            // Extract explicit specializes relationship targets
            let mut specializes: Vec<String> = definition
                .relationships
                .specializes
                .iter()
                .map(|s| s.target.clone())
                .collect();

            // If no explicit specialization, add implicit supertype based on definition kind
            if specializes.is_empty() {
                if let Some(implicit) = implicit_supertype_for_kind(&kind) {
                    specializes.push(implicit.to_string());
                }
            }

            let symbol = Symbol::Definition {
                name: name.clone(),
                qualified_name: qualified_name.clone(),
                kind,
                semantic_role: Some(semantic_role),
                scope_id,
                source_file: self.symbol_table.current_file().map(String::from),
                span: definition.span,
                documentation,
                specializes,
            };
            self.insert_symbol(name.clone(), symbol);

            // If there's a short name, create an alias for it
            if let Some(ref short_name) = definition.short_name {
                let short_qualified_name = self.qualified_name(short_name);
                let alias_symbol = Symbol::Alias {
                    name: short_name.clone(),
                    qualified_name: short_qualified_name,
                    target: qualified_name.clone(),
                    target_span: definition.span,
                    scope_id,
                    source_file: self.symbol_table.current_file().map(String::from),
                    span: definition.short_name_span,
                };
                self.insert_symbol(short_name.clone(), alias_symbol);
            }

            // Index all relationship references for reverse lookups
            for spec in &definition.relationships.specializes {
                self.index_reference_with_chain_context(&qualified_name, &spec.target, spec.span, None, spec.chain_context.clone());
            }
            for redef in &definition.relationships.redefines {
                self.index_reference_with_chain_context(&qualified_name, &redef.target, redef.span, None, redef.chain_context.clone());
            }
            for include in &definition.relationships.includes {
                self.index_reference_with_chain_context(&qualified_name, &include.target, include.span, None, include.chain_context.clone());
            }
            // Index meta type references (e.g., filter @SysML::PartUsage inside view def)
            for meta in &definition.relationships.meta {
                self.index_reference_with_chain_context(&qualified_name, &meta.target, meta.span, None, meta.chain_context.clone());
            }

            // Index domain relationships from nested usages
            for member in &definition.body {
                if let crate::syntax::sysml::ast::enums::DefinitionMember::Usage(usage) = member {
                    if let Some((target, span)) = usage.domain_target() {
                        self.index_reference(&qualified_name, target, span);
                    }
                    // Also index explicit relationship targets
                    for satisfy in &usage.relationships.satisfies {
                        self.index_reference_with_chain_context(&qualified_name, &satisfy.target, satisfy.span, None, satisfy.chain_context.clone());
                    }
                    for perform in &usage.relationships.performs {
                        self.index_reference_with_chain_context(&qualified_name, &perform.target, perform.span, None, perform.chain_context.clone());
                    }
                    for exhibit in &usage.relationships.exhibits {
                        self.index_reference_with_chain_context(&qualified_name, &exhibit.target, exhibit.span, None, exhibit.chain_context.clone());
                    }
                    for include in &usage.relationships.includes {
                        self.index_reference_with_chain_context(&qualified_name, &include.target, include.span, None, include.chain_context.clone());
                    }
                }
            }

            // Visit nested members in the body
            self.enter_namespace(name.clone());
            for member in &definition.body {
                match member {
                    crate::syntax::sysml::ast::enums::DefinitionMember::Usage(usage) => {
                        self.visit_usage(usage);
                    }
                    crate::syntax::sysml::ast::enums::DefinitionMember::Import(import) => {
                        self.visit_import(import);
                    }
                    crate::syntax::sysml::ast::enums::DefinitionMember::Comment(comment) => {
                        self.visit_comment(comment);
                    }
                }
            }
            self.exit_namespace();
        }
    }

    fn visit_usage(&mut self, usage: &Usage) {
        trace!("[VISITOR] visit_usage: name={:?} kind={:?} typed_by={:?} redefines={:?} subsets={:?} expr_refs={:?}",
            usage.name, usage.kind, usage.relationships.typed_by, 
            usage.relationships.redefines, usage.relationships.subsets,
            usage.expression_refs);
        
        // Determine the name and span: prefer explicit name, fall back to first redefinition or subsetting target
        let (name, name_span, is_anonymous) = if let Some(name) = &usage.name {
            (name.clone(), usage.span, false)
        } else if let Some(first_redef) = usage.relationships.redefines.first() {
            // Use the redefinition target as the name, with its span (for :>>)
            (first_redef.target.clone(), first_redef.span, true)
        } else if let Some(first_subset) = usage.relationships.subsets.first() {
            // For perform/exhibit/satisfy/include usages with subsets (e.g., `perform startVehicle.trigger1`),
            // these are references to external elements, NOT declarations of new children.
            // Don't create child symbols - instead, add perform targets to the parent symbol.
            let is_reference_only = matches!(
                usage.kind,
                UsageKind::PerformAction
                    | UsageKind::ExhibitState
                    | UsageKind::SatisfyRequirement
                    | UsageKind::IncludeUseCase
            );
            
            if is_reference_only {
                let parent_qname = self.current_namespace.join("::");
                // Index all subset references for hover/go-to-definition
                for subset in &usage.relationships.subsets {
                    self.index_reference_with_chain_context(
                        &parent_qname,
                        &subset.target,
                        subset.span,
                        Some(TokenType::Property),
                        subset.chain_context.clone(),
                    );
                }
                
                // Add perform targets to the parent symbol for resolution
                // e.g., `perform startVehicle.sendStatus` adds "startVehicle.sendStatus" to parent's performs
                if usage.kind == UsageKind::PerformAction {
                    let perform_targets = extract_relationship_targets(&usage.relationships.subsets);
                    if let Some(parent) = self.symbol_table.find_by_qualified_name_mut(&parent_qname) {
                        if let Symbol::Usage { performs, .. } = parent {
                            for pt in perform_targets {
                                if !performs.contains(&pt) {
                                    performs.push(pt);
                                }
                            }
                        }
                    }
                }
                return;
            }
            
            // For other anonymous usages with subsets, use the target as the name
            (first_subset.target.clone(), first_subset.span, true)
        } else if let Some(first_perform) = usage.relationships.performs.first() {
            // Use the perform target's base name (e.g., "startVehicle" from "startVehicle.trigger1")
            // as the name, with its span (fallback in case performs are not in subsets)
            let base_name = first_perform.target.split('.').next().unwrap_or(&first_perform.target).to_string();
            (base_name, first_perform.span, true)
        } else {
            // Anonymous usage with no name - still need to index references for hover/semantic tokens
            // This is common for flow usages like `flow of Exposure from focus.xrsl to shoot.xsf;`
            // and connection end usages like `end #original ::> vehicleSpecification.vehicleMassRequirement;`
            let parent_qname = self.current_namespace.join("::");
            
            // Index the typed_by reference (e.g., Exposure in `flow of Exposure`)
            if let Some(ref target) = usage.relationships.typed_by {
                self.index_reference_with_type(
                    &parent_qname,
                    target,
                    usage.relationships.typed_by_span,
                    Some(TokenType::Type),
                );
            }
            
            // Index expression references (e.g., focus.xrsl and shoot.xsf in flow)
            for expr_ref in &usage.expression_refs {
                self.index_reference_with_chain_context(
                    &parent_qname,
                    &expr_ref.name,
                    expr_ref.span,
                    Some(TokenType::Property),
                    expr_ref.chain_context.clone(),
                );
            }
            
            // Index references (::>) - e.g., `end #original ::> vehicleSpecification.vehicleMassRequirement;`
            for reference in &usage.relationships.references {
                self.index_reference_with_chain_context(
                    &parent_qname,
                    &reference.target,
                    reference.span,
                    Some(TokenType::Property),
                    reference.chain_context.clone(),
                );
            }
            
            // IMPORTANT: Visit nested members even for anonymous usages!
            // This is needed for connection usages like `#derivation connection { end #original ::> ... }`
            // where the end usages are nested inside the anonymous connection usage.
            for member in &usage.body {
                match member {
                    crate::syntax::sysml::ast::enums::UsageMember::Usage(nested_usage) => {
                        self.visit_usage(nested_usage);
                    }
                    crate::syntax::sysml::ast::enums::UsageMember::Comment(comment) => {
                        self.visit_comment(comment);
                    }
                }
            }
            
            return;
        };

        let qualified_name = self.qualified_name(&name);

        // Always index relationship references, even for duplicate anonymous usages.
        // This ensures semantic tokens are generated for type references like
        // `ref :> annotatedElement : SysML::ConnectionDefinition;`
        //
        // Token types are determined by what the reference points to:
        // - redefines/subsets → Property (they reference usages/features)
        // - typed_by → Type (they reference definitions/classifiers)
        for rel in &usage.relationships.redefines {
            self.index_reference_with_chain_context(
                &qualified_name,
                &rel.target,
                rel.span,
                Some(TokenType::Property),
                rel.chain_context.clone(),
            );
        }
        for subset in &usage.relationships.subsets {
            self.index_reference_with_chain_context(
                &qualified_name,
                &subset.target,
                subset.span,
                Some(TokenType::Property),
                subset.chain_context.clone(),
            );
        }
        if let Some(ref target) = usage.relationships.typed_by {
            self.index_reference_with_type(
                &qualified_name,
                target,
                usage.relationships.typed_by_span,
                Some(TokenType::Type),
            );
        }
        // references (::>) target usages, so Property token
        for reference in &usage.relationships.references {
            self.index_reference_with_chain_context(
                &qualified_name,
                &reference.target,
                reference.span,
                Some(TokenType::Property),
                reference.chain_context.clone(),
            );
        }
        // crosses (=>) target usages, so Property token
        for cross in &usage.relationships.crosses {
            self.index_reference_with_chain_context(
                &qualified_name,
                &cross.target,
                cross.span,
                Some(TokenType::Property),
                cross.chain_context.clone(),
            );
        }
        for meta in &usage.relationships.meta {
            self.index_reference(&qualified_name, &meta.target, meta.span);
        }
        // NOTE: expression_refs are indexed later, inside the body namespace (if any)
        // so they can resolve to payload parameters defined in accept statements.

        // For perform/exhibit/satisfy/include usages with the "action X" / "state X" / etc. form,
        // the name is also a reference to the element being performed/exhibited/satisfied/included.
        // e.g., "perform action providePower;" - providePower is both the name AND a reference to ActionTree::providePower
        // This only applies when there are no explicit relationship targets (performs/exhibits/satisfies/includes).
        if !is_anonymous {
            match usage.kind {
                UsageKind::PerformAction if usage.relationships.performs.is_empty() => {
                    trace!("[VISITOR] perform action usage '{}' has name but no explicit performs - indexing name as reference", name);
                    self.index_reference_with_type(
                        &qualified_name,
                        &name,
                        name_span,
                        Some(TokenType::Property), // Actions are features/properties
                    );
                }
                UsageKind::ExhibitState if usage.relationships.exhibits.is_empty() => {
                    trace!("[VISITOR] exhibit state usage '{}' has name but no explicit exhibits - indexing name as reference", name);
                    self.index_reference_with_type(
                        &qualified_name,
                        &name,
                        name_span,
                        Some(TokenType::Property),
                    );
                }
                UsageKind::SatisfyRequirement if usage.relationships.satisfies.is_empty() => {
                    trace!("[VISITOR] satisfy requirement usage '{}' has name but no explicit satisfies - indexing name as reference", name);
                    self.index_reference_with_type(
                        &qualified_name,
                        &name,
                        name_span,
                        Some(TokenType::Type),
                    );
                }
                UsageKind::IncludeUseCase if usage.relationships.includes.is_empty() => {
                    trace!("[VISITOR] include use case usage '{}' has name but no explicit includes - indexing name as reference", name);
                    self.index_reference_with_type(
                        &qualified_name,
                        &name,
                        name_span,
                        Some(TokenType::Property),
                    );
                }
                _ => {}
            }
        }

        // Skip duplicate anonymous usages (don't add to symbol table twice)
        if is_anonymous
            && self
                .symbol_table
                .find_by_qualified_name(&qualified_name)
                .is_some()
        {
            return;
        }

        let kind = Self::map_usage_kind(&usage.kind);
        let semantic_role = Self::usage_kind_to_semantic_role(&usage.kind);
        let scope_id = self.symbol_table.current_scope_id();
        let documentation = extract_doc_from_usage(&usage.body);

        // Extract subsets and redefines relationship targets
        // For feature chains, reconstruct the full chain as the target
        // e.g., `:>> localClock.currentTime` should have redefines = ["localClock.currentTime"]
        let mut subsets: Vec<String> = extract_relationship_targets(&usage.relationships.subsets);
        let redefines: Vec<String> = extract_relationship_targets(&usage.relationships.redefines);
        
        // For PerformAction usages, performs are also subsets
        if usage.kind == UsageKind::PerformAction && !usage.relationships.performs.is_empty() {
            let perform_targets = extract_relationship_targets(&usage.relationships.performs);
            for pt in perform_targets {
                if !subsets.contains(&pt) {
                    subsets.push(pt);
                }
            }
        }

        // Build references list from `::>` featured_by relationships
        let references: Vec<String> = usage.relationships.references.iter()
            .map(|r| r.target.clone())
            .collect();

        let symbol = Symbol::Usage {
            name: name.clone(),
            qualified_name: qualified_name.clone(),
            kind,
            semantic_role: Some(semantic_role),
            usage_type: usage.relationships.typed_by.clone(),
            scope_id,
            source_file: self.symbol_table.current_file().map(String::from),
            span: name_span,
            documentation,
            subsets,
            redefines,
            performs: Vec::new(), // Populated by nested perform usages when visited
            references,
        };
        self.insert_symbol(name.clone(), symbol);

        // If there's a short name, create an alias
        if let Some(ref short_name) = usage.short_name {
            let short_qualified_name = self.qualified_name(short_name);
            let alias_symbol = Symbol::Alias {
                name: short_name.clone(),
                qualified_name: short_qualified_name,
                target: qualified_name.clone(),
                target_span: usage.span,
                scope_id,
                source_file: self.symbol_table.current_file().map(String::from),
                span: usage.short_name_span,
            };
            self.insert_symbol(short_name.clone(), alias_symbol);
        }

        // Visit nested members
        if !usage.body.is_empty() {
            self.enter_namespace(name.clone());
            
            // Index expression_refs AFTER entering namespace, so they can resolve
            // to payload parameters defined in accept statements within the body.
            // e.g., "if ignitionCmd.ignitionOnOff==..." where ignitionCmd comes from
            // "accept ignitionCmd:IgnitionCmd via ignitionCmdPort"
            trace!("[VISITOR] indexing expression_refs for '{}': {:?}", qualified_name, usage.expression_refs);
            for expr_ref in &usage.expression_refs {
                trace!("[VISITOR]   expr_ref: name='{}' span={:?} chain={:?}", expr_ref.name, expr_ref.span, expr_ref.chain_context);
                self.index_reference_with_chain_context(
                    &qualified_name,
                    &expr_ref.name,
                    expr_ref.span,
                    Some(TokenType::Property),
                    expr_ref.chain_context.clone(),
                );
            }
            
            for member in &usage.body {
                match member {
                    crate::syntax::sysml::ast::enums::UsageMember::Usage(nested_usage) => {
                        self.visit_usage(nested_usage);
                    }
                    crate::syntax::sysml::ast::enums::UsageMember::Comment(comment) => {
                        self.visit_comment(comment);
                    }
                }
            }
            self.exit_namespace();
        } else {
            // No body - index expression_refs in current scope
            trace!("[VISITOR] indexing expression_refs for '{}': {:?}", qualified_name, usage.expression_refs);
            for expr_ref in &usage.expression_refs {
                trace!("[VISITOR]   expr_ref: name='{}' span={:?} chain={:?}", expr_ref.name, expr_ref.span, expr_ref.chain_context);
                self.index_reference_with_chain_context(
                    &qualified_name,
                    &expr_ref.name,
                    expr_ref.span,
                    Some(TokenType::Property),
                    expr_ref.chain_context.clone(),
                );
            }
        }
    }

    fn visit_import(&mut self, import: &Import) {
        let current_file = self.symbol_table.current_file().map(String::from);
        self.symbol_table.add_import(
            import.path.clone(),
            import.is_recursive,
            import.is_public,
            import.span,
            current_file.clone(),
        );

        let scope_id = self.symbol_table.current_scope_id();
        let qualified_name = format!("import::{}::{}", scope_id, import.path);
        let key = format!("import::{}", import.path);

        let symbol = Symbol::Import {
            path: import.path.clone(),
            path_span: import.path_span,
            qualified_name: qualified_name.clone(),
            is_recursive: import.is_recursive,
            scope_id,
            source_file: current_file,
            span: import.span,
        };
        self.insert_symbol(key, symbol);

        // Index the import target for hover support
        // Strip wildcard markers to get the base target:
        //   "PictureTaking::*" -> "PictureTaking"
        //   "PictureTaking::**" -> "PictureTaking"
        //   "PictureTaking::*::**" -> "PictureTaking"
        //   "PictureTaking::takePicture" -> "PictureTaking::takePicture"
        // Also strip quotes from each part of the qualified name:
        //   "'Robotic Vacuum Cleaner'::*" -> "Robotic Vacuum Cleaner"
        let target = strip_qualified_name_quotes(
            import
                .path
                .trim_end_matches("::**")
                .trim_end_matches("::*")
                .trim_end_matches("::**"),
        );
        if !target.is_empty() {
            self.index_reference(&qualified_name, &target, import.path_span);
        }
    }

    fn visit_comment(&mut self, comment: &Comment) {
        // If the comment has a name, register it as a symbol
        if let Some(name) = &comment.name {
            let qualified_name = self.qualified_name(name);
            let scope_id = self.symbol_table.current_scope_id();
            let source_file = self.symbol_table.current_file().map(String::from);
            
            // Extract the actual comment text (from block comment)
            let doc_text = extract_doc_text(&comment.content);
            
            let symbol = Symbol::Comment {
                name: name.clone(),
                qualified_name: qualified_name.clone(),
                scope_id,
                source_file,
                span: comment.name_span,
                documentation: if doc_text.is_empty() { None } else { Some(doc_text) },
            };
            self.insert_symbol(name.clone(), symbol);
            
            // Index the `about` references
            for about_ref in &comment.about {
                self.index_reference(&qualified_name, &about_ref.name, about_ref.span);
            }
        } else if !comment.about.is_empty() {
            // Anonymous comment with `about` references - still index them
            // Use the current namespace for context
            let context_name = self.current_namespace.join("::");
            let qualified_name = if context_name.is_empty() {
                "<anonymous_comment>".to_string()
            } else {
                format!("{}::<anonymous_comment>", context_name)
            };
            
            for about_ref in &comment.about {
                self.index_reference(&qualified_name, &about_ref.name, about_ref.span);
            }
        }
    }

    fn visit_alias(&mut self, alias: &Alias) {
        if let Some(name) = &alias.name {
            let qualified_name = self.qualified_name(name);
            let scope_id = self.symbol_table.current_scope_id();
            
            // Qualify the target - it may be a simple name like "duration" that needs
            // to be resolved to "ISQSpaceTime::duration" for the alias to work properly
            let qualified_target = if alias.target.contains("::") {
                // Already qualified
                alias.target.clone()
            } else {
                // Try to qualify using current namespace
                let candidate = self.qualified_name(&alias.target);
                // Check if the candidate exists in the symbol table
                if self.symbol_table.find_by_qualified_name(&candidate).is_some() {
                    candidate
                } else {
                    // Fallback to raw target (may be from import)
                    alias.target.clone()
                }
            };
            trace!("[VISITOR] visit_alias: name='{}' raw_target='{}' qualified_target='{}'", 
                name, alias.target, qualified_target);
            
            let symbol = Symbol::Alias {
                name: name.clone(),
                qualified_name: qualified_name.clone(),
                target: qualified_target.clone(),
                target_span: alias.target_span,
                scope_id,
                source_file: self.symbol_table.current_file().map(String::from),
                span: alias.span,
            };
            self.insert_symbol(name.clone(), symbol);

            // Index the alias target reference for hover/go-to-definition
            // e.g., in `alias Torque for ISQ::TorqueValue;`, index ISQ::TorqueValue
            self.index_reference(&qualified_name, &alias.target, alias.target_span);
        }
    }
}

impl<'a> SysmlAdapter<'a> {
    pub fn visit_dependency(&mut self, dependency: &Dependency) {
        // Index source references (before "to")
        let source_qname = self.current_namespace.join("::");
        for source in &dependency.sources {
            self.index_reference(&source_qname, &source.path, source.span);
        }

        // Index target references (after "to")
        for target in &dependency.targets {
            self.index_reference(&source_qname, &target.path, target.span);
        }
    }
}
impl<'a> SysmlAdapter<'a> {
    /// Visit an element filter member (e.g., `filter @Safety;`)
    /// Filter members don't create symbols, but their expression refs need to be indexed.
    pub fn visit_filter(&mut self, filter: &crate::syntax::sysml::ast::types::Filter) {
        // Use the enclosing namespace as the source for references
        let source_qname = if self.current_namespace.is_empty() {
            "<root>".to_string()
        } else {
            self.current_namespace.join("::")
        };

        // Index metadata references (e.g., @Safety, @SysML::PartUsage)
        for meta_ref in &filter.meta_refs {
            self.index_reference(&source_qname, &meta_ref.target, meta_ref.span);
        }

        // Index feature references (e.g., Safety::isMandatory)
        for expr_ref in &filter.expression_refs {
            self.index_reference(&source_qname, &expr_ref.name, expr_ref.span);
        }
    }
}