use super::*;

impl NormalizedImport {
    pub(super) fn from_rowan(import: &RowanImport) -> Self {
        let target = import.target();
        let path_range = target.as_ref().map(|t| t.syntax().text_range());
        let path = target
            .map(|t| {
                let mut path = t.to_string();
                if import.is_wildcard() {
                    path.push_str("::*");
                }
                if import.is_recursive() {
                    // Change ::* to ::** if recursive
                    if path.ends_with("::*") {
                        path.push('*');
                    } else {
                        path.push_str("::**");
                    }
                }
                path
            })
            .unwrap_or_default();

        // Extract filter metadata from bracket syntax [@Filter]
        // Multiple filters like [@A][@B] are all inside one FILTER_PACKAGE
        let filters = import
            .filter()
            .map(|fp| fp.targets().into_iter().map(|qn| qn.to_string()).collect())
            .unwrap_or_default();

        Self {
            path,
            path_range,
            range: Some(import.syntax().text_range()),
            is_public: import.is_public(),
            filters,
        }
    }
}

impl NormalizedAlias {
    pub(super) fn from_rowan(alias: &parser::Alias) -> Self {
        Self {
            name: alias.name().and_then(|n| n.text()),
            short_name: alias
                .name()
                .and_then(|n| n.short_name())
                .and_then(|sn| sn.text()),
            target: alias.target().map(|t| t.to_string()).unwrap_or_default(),
            target_range: alias.target().map(|t| t.syntax().text_range()),
            name_range: alias.name().map(|n| n.syntax().text_range()),
            range: Some(alias.syntax().text_range()),
        }
    }
}

impl NormalizedDependency {
    pub(super) fn from_rowan(dep: &parser::Dependency) -> Self {
        let mut sources = Vec::new();
        let mut targets = Vec::new();
        let mut relationships = Vec::new();

        // Extract source references (before "to")
        for source in dep.sources() {
            let target_str = source.to_string();
            let rel_target = make_chain_or_simple(&target_str, &source);
            sources.push(NormalizedRelationship {
                kind: NormalizedRelKind::DependencySource,
                target: rel_target,
                range: Some(source.syntax().text_range()),
            });
        }

        // Extract target reference (after "to")
        if let Some(target) = dep.target() {
            let target_str = target.to_string();
            let rel_target = make_chain_or_simple(&target_str, &target);
            targets.push(NormalizedRelationship {
                kind: NormalizedRelKind::DependencyTarget,
                target: rel_target,
                range: Some(target.syntax().text_range()),
            });
        }

        // Extract prefix metadata (#name) as Meta relationships
        for prefix_meta in dep.prefix_metadata() {
            if let (Some(name), Some(range)) = (prefix_meta.name(), prefix_meta.name_range()) {
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::Meta,
                    target: RelTarget::Simple(name),
                    range: Some(range),
                });
            }
        }

        Self {
            name: None, // Dependencies typically don't have names
            short_name: None,
            sources,
            targets,
            relationships,
            range: Some(dep.syntax().text_range()),
        }
    }
}

// ============================================================================
