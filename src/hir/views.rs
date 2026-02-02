//! SysML v2 View definitions and related structures.
//!
//! This module contains HIR representations for SysML v2 Views (Section 7.26),
//! including ViewDefinition, ViewUsage, filter conditions, expose relationships,
//! and rendering specifications.

use crate::syntax::Span;
use std::sync::Arc;

/// Qualified name type (alias for Arc<str>).
pub type QualifiedName = Arc<str>;

/// View-specific data attached to HirSymbol.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ViewData {
    /// A ViewDefinition (defines what elements to show).
    ViewDefinition(ViewDefinition),
    /// A ViewUsage (instance of a view).
    ViewUsage(ViewUsage),
    /// A ViewpointDefinition (defines stakeholder concerns).
    ViewpointDefinition(ViewpointDefinition),
    /// A ViewpointUsage (instance of a viewpoint).
    ViewpointUsage(ViewpointUsage),
    /// A RenderingDefinition (defines how to render).
    RenderingDefinition(RenderingDefinition),
    /// A RenderingUsage (instance of a rendering).
    RenderingUsage(RenderingUsage),
    /// An expose relationship (makes elements visible in a view).
    ExposeRelationship(ExposeRelationship),
}

/// A ViewDefinition defines what elements should be visible in a diagram.
///
/// Example:
/// ```sysml
/// view def VehicleStructureView {
///     expose Model::Vehicle::**;
///     filter @SysML::PartUsage;
///     render Views::asTreeDiagram;
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ViewDefinition {
    /// Elements exposed (made visible) by this view.
    pub expose: Vec<ExposeRelationship>,
    /// Filter conditions to apply to exposed elements.
    pub filter: Vec<FilterCondition>,
    /// Rendering specification (how to visualize).
    pub rendering: Option<RenderingSpec>,
    /// Span in source code.
    pub span: Option<Span>,
}

/// A ViewUsage is an instance of a ViewDefinition.
///
/// Example:
/// ```sysml
/// view vehicleView : VehicleStructureView {
///     expose Model::Vehicle::engine;
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ViewUsage {
    /// The ViewDefinition this view is typed by.
    pub view_def: Option<QualifiedName>,
    /// Additional elements exposed by this usage.
    pub expose: Vec<ExposeRelationship>,
    /// Additional filter conditions.
    pub filter: Vec<FilterCondition>,
    /// Rendering specification override.
    pub rendering: Option<RenderingSpec>,
    /// Span in source code.
    pub span: Option<Span>,
}

/// A ViewpointDefinition defines stakeholder concerns.
///
/// Example:
/// ```sysml
/// viewpoint def SafetyViewpoint {
///     require stakeholder : SafetyEngineer;
///     require concern : "Safety analysis";
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ViewpointDefinition {
    /// Required stakeholders for this viewpoint.
    pub stakeholders: Vec<QualifiedName>,
    /// Concerns addressed by this viewpoint.
    pub concerns: Vec<QualifiedName>,
    /// Span in source code.
    pub span: Option<Span>,
}

/// A ViewpointUsage is an instance of a ViewpointDefinition.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ViewpointUsage {
    /// The ViewpointDefinition this is typed by.
    pub viewpoint_def: Option<QualifiedName>,
    /// Span in source code.
    pub span: Option<Span>,
}

/// A RenderingDefinition defines how to visualize a view.
///
/// Example:
/// ```sysml
/// rendering def TreeDiagram {
///     // Layout algorithm, styling, etc.
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenderingDefinition {
    /// Layout algorithm (e.g., "layered", "tree", "force-directed").
    pub layout: Option<String>,
    /// Span in source code.
    pub span: Option<Span>,
}

/// A RenderingUsage is an instance of a RenderingDefinition.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenderingUsage {
    /// The RenderingDefinition this is typed by.
    pub rendering_def: Option<QualifiedName>,
    /// Span in source code.
    pub span: Option<Span>,
}

/// An expose relationship makes elements visible in a view.
///
/// Examples:
/// - `expose Model::Vehicle;` (member expose)
/// - `expose Model::Vehicle::*;` (namespace expose - direct children)
/// - `expose Model::Vehicle::**;` (recursive expose - all descendants)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExposeRelationship {
    /// The import path being exposed.
    pub import_path: ImportPath,
    /// Whether this is a namespace expose (`*`) or recursive expose (`**`).
    pub is_recursive: bool,
    /// Span in source code.
    pub span: Option<Span>,
}

/// An import path in an expose relationship.
///
/// Examples:
/// - `Model::Vehicle` (specific element)
/// - `Model::Vehicle::*` (all direct children)
/// - `Model::Vehicle::**` (all descendants recursively)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImportPath {
    /// The qualified name of the target element.
    pub target: QualifiedName,
    /// Whether this is a wildcard import (`*` = direct, `**` = recursive).
    pub wildcard: WildcardKind,
}

/// Wildcard kinds for import paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WildcardKind {
    /// No wildcard - specific element.
    None,
    /// Single wildcard (`*`) - direct children only.
    Direct,
    /// Double wildcard (`**`) - all descendants recursively.
    Recursive,
}

/// A filter condition determines which elements to include in a view.
///
/// Examples:
/// - `filter @SysML::PartUsage;` (metadata check)
/// - `filter element.type == "PartDef";` (property check - DEFERRED in Phase 4)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FilterCondition {
    /// Filter by metadata annotation (e.g., `@SysML::PartUsage`).
    Metadata(MetadataFilter),
    /// Custom boolean expression (DEFERRED to Phase 4).
    Expression(String),
}

/// A metadata filter checks for specific annotations.
///
/// Example: `@SysML::PartUsage` checks if element has that metadata.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MetadataFilter {
    /// The qualified name of the metadata annotation.
    pub annotation: QualifiedName,
    /// Span in source code.
    pub span: Option<Span>,
}

/// A rendering specification (how to visualize a view).
///
/// Example: `render Views::asTreeDiagram;`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenderingSpec {
    /// The qualified name of the rendering definition.
    pub rendering: QualifiedName,
    /// Span in source code.
    pub span: Option<Span>,
}

impl ViewDefinition {
    /// Create a new empty ViewDefinition.
    pub fn new() -> Self {
        Self {
            expose: Vec::new(),
            filter: Vec::new(),
            rendering: None,
            span: None,
        }
    }

    /// Add an expose relationship.
    pub fn add_expose(&mut self, expose: ExposeRelationship) {
        self.expose.push(expose);
    }

    /// Add a filter condition.
    pub fn add_filter(&mut self, filter: FilterCondition) {
        self.filter.push(filter);
    }

    /// Set the rendering specification.
    pub fn set_rendering(&mut self, rendering: RenderingSpec) {
        self.rendering = Some(rendering);
    }

    /// Check if an element passes all filter conditions in this view.
    ///
    /// Returns `true` if the element passes all filters (OR if no filters are defined).
    /// An element must match ALL filter conditions to pass (AND logic).
    ///
    /// # Arguments
    /// * `element_metadata` - List of metadata annotations on the element
    ///
    /// # Example
    /// ```rust
    /// use syster::hir::{ViewDefinition, FilterCondition};
    /// use std::sync::Arc;
    ///
    /// let mut view = ViewDefinition::new();
    /// view.add_filter(FilterCondition::metadata(Arc::from("SysML::PartUsage")));
    ///
    /// let metadata: Vec<Arc<str>> = vec![Arc::from("SysML::PartUsage")];
    /// assert!(view.passes_filters(&metadata));
    ///
    /// let wrong_metadata: Vec<Arc<str>> = vec![Arc::from("SysML::ActionUsage")];
    /// assert!(!view.passes_filters(&wrong_metadata));
    /// ```
    pub fn passes_filters(&self, element_metadata: &[QualifiedName]) -> bool {
        // If no filters, everything passes
        if self.filter.is_empty() {
            return true;
        }

        // All filters must match (AND logic)
        self.filter
            .iter()
            .all(|filter| filter.matches(element_metadata))
    }

    /// Apply this view to a set of symbols, returning the final visible elements.
    ///
    /// This is the main entry point for view application. It:
    /// 1. Resolves all expose relationships to get candidate symbols
    /// 2. Applies filter conditions to keep only matching elements
    /// 3. Returns the final list of qualified names that should be visible
    ///
    /// # Arguments
    /// * `symbols` - Iterator of tuples (qualified_name, metadata_annotations)
    ///
    /// # Returns
    /// Vector of qualified names that should be visible in this view
    ///
    /// # Example
    /// ```rust
    /// use syster::hir::{ViewDefinition, ExposeRelationship, FilterCondition, WildcardKind};
    /// use std::sync::Arc;
    ///
    /// let mut view = ViewDefinition::new();
    /// view.add_expose(ExposeRelationship::new(
    ///     Arc::from("Model::Vehicle"),
    ///     WildcardKind::Direct
    /// ));
    /// view.add_filter(FilterCondition::metadata(Arc::from("SysML::PartUsage")));
    ///
    /// let symbols: Vec<(&str, Vec<Arc<str>>)> = vec![
    ///     ("Model::Vehicle::engine", vec![Arc::from("SysML::PartUsage")]),
    ///     ("Model::Vehicle::wheels", vec![Arc::from("SysML::PartUsage")]),
    ///     ("Model::Vehicle::name", vec![Arc::from("SysML::AttributeUsage")]),
    /// ];
    ///
    /// let result = view.apply(symbols.iter().map(|(qn, meta)| (*qn, meta.as_slice())));
    /// assert_eq!(result.len(), 2); // Only engine and wheels (PartUsage), not name
    /// ```
    pub fn apply<'a, I>(&self, symbols: I) -> Vec<QualifiedName>
    where
        I: Iterator<Item = (&'a str, &'a [QualifiedName])> + Clone,
    {
        // If no expose relationships, nothing is visible
        if self.expose.is_empty() {
            return Vec::new();
        }

        // Step 1: Resolve all expose relationships to get candidate qualified names
        let mut candidates = std::collections::HashSet::new();
        let symbol_names: Vec<&str> = symbols.clone().map(|(qn, _)| qn).collect();

        for expose_rel in &self.expose {
            let resolved = expose_rel.resolve(symbol_names.iter().copied());
            candidates.extend(resolved);
        }

        // Step 2: Apply filters to candidates
        if self.filter.is_empty() {
            // No filters, return all candidates
            candidates.into_iter().collect()
        } else {
            // Filter candidates by metadata
            let symbol_map: std::collections::HashMap<&str, &[QualifiedName]> = symbols.collect();

            candidates
                .into_iter()
                .filter(|qname| {
                    // Get metadata for this symbol
                    if let Some(&metadata) = symbol_map.get(qname.as_ref()) {
                        self.passes_filters(metadata)
                    } else {
                        // Symbol has no metadata entry, apply filters to empty metadata
                        self.passes_filters(&[])
                    }
                })
                .collect()
        }
    }
}

impl Default for ViewDefinition {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewUsage {
    /// Create a new ViewUsage.
    pub fn new(view_def: Option<QualifiedName>) -> Self {
        Self {
            view_def,
            expose: Vec::new(),
            filter: Vec::new(),
            rendering: None,
            span: None,
        }
    }

    /// Add an expose relationship.
    pub fn add_expose(&mut self, expose: ExposeRelationship) {
        self.expose.push(expose);
    }

    /// Add a filter condition.
    pub fn add_filter(&mut self, filter: FilterCondition) {
        self.filter.push(filter);
    }

    /// Check if an element passes all filter conditions in this view usage.
    ///
    /// Returns `true` if the element passes all filters (OR if no filters are defined).
    /// An element must match ALL filter conditions to pass (AND logic).
    ///
    /// # Arguments
    /// * `element_metadata` - List of metadata annotations on the element
    pub fn passes_filters(&self, element_metadata: &[QualifiedName]) -> bool {
        // If no filters, everything passes
        if self.filter.is_empty() {
            return true;
        }

        // All filters must match (AND logic)
        self.filter
            .iter()
            .all(|filter| filter.matches(element_metadata))
    }

    /// Apply this view usage to a set of symbols, returning the final visible elements.
    ///
    /// Similar to ViewDefinition::apply(), but for view instances.
    ///
    /// # Arguments
    /// * `symbols` - Iterator of tuples (qualified_name, metadata_annotations)
    ///
    /// # Returns
    /// Vector of qualified names that should be visible in this view usage
    pub fn apply<'a, I>(&self, symbols: I) -> Vec<QualifiedName>
    where
        I: Iterator<Item = (&'a str, &'a [QualifiedName])> + Clone,
    {
        // If no expose relationships, nothing is visible
        if self.expose.is_empty() {
            return Vec::new();
        }

        // Step 1: Resolve all expose relationships to get candidate qualified names
        let mut candidates = std::collections::HashSet::new();
        let symbol_names: Vec<&str> = symbols.clone().map(|(qn, _)| qn).collect();

        for expose_rel in &self.expose {
            let resolved = expose_rel.resolve(symbol_names.iter().copied());
            candidates.extend(resolved);
        }

        // Step 2: Apply filters to candidates
        if self.filter.is_empty() {
            // No filters, return all candidates
            candidates.into_iter().collect()
        } else {
            // Filter candidates by metadata
            let symbol_map: std::collections::HashMap<&str, &[QualifiedName]> = symbols.collect();

            candidates
                .into_iter()
                .filter(|qname| {
                    // Get metadata for this symbol
                    if let Some(&metadata) = symbol_map.get(qname.as_ref()) {
                        self.passes_filters(metadata)
                    } else {
                        // Symbol has no metadata entry, apply filters to empty metadata
                        self.passes_filters(&[])
                    }
                })
                .collect()
        }
    }
}

impl ExposeRelationship {
    /// Create a new expose relationship.
    pub fn new(target: QualifiedName, wildcard: WildcardKind) -> Self {
        Self {
            import_path: ImportPath { target, wildcard },
            is_recursive: wildcard == WildcardKind::Recursive,
            span: None,
        }
    }

    /// Create from an import path.
    pub fn from_path(import_path: ImportPath) -> Self {
        Self {
            is_recursive: import_path.wildcard == WildcardKind::Recursive,
            import_path,
            span: None,
        }
    }

    /// Get the target qualified name.
    pub fn target(&self) -> &QualifiedName {
        &self.import_path.target
    }

    /// Check if this is a recursive expose.
    pub fn is_recursive(&self) -> bool {
        self.is_recursive
    }

    /// Check if this is a namespace expose (direct children).
    pub fn is_namespace(&self) -> bool {
        self.import_path.wildcard == WildcardKind::Direct
    }

    /// Check if this is a member expose (specific element).
    pub fn is_member(&self) -> bool {
        self.import_path.wildcard == WildcardKind::None
    }

    /// Resolve this expose relationship to a list of qualified names.
    ///
    /// Returns all symbols that should be exposed according to this relationship.
    ///
    /// # Arguments
    /// * `all_symbols` - Iterator over all available symbols with their qualified names
    ///
    /// # Returns
    /// Vector of qualified names that match this expose relationship
    ///
    /// # Examples
    ///
    /// ```rust
    /// use syster::hir::{ExposeRelationship, WildcardKind};
    /// use std::sync::Arc;
    ///
    /// // Member expose: expose Model::Vehicle
    /// let expose = ExposeRelationship::new(Arc::from("Model::Vehicle"), WildcardKind::None);
    /// let symbols: Vec<Arc<str>> = vec![
    ///     Arc::from("Model::Vehicle"),
    ///     Arc::from("Model::Vehicle::engine"),
    /// ];
    /// let result = expose.resolve(symbols.iter().map(|s| s.as_ref()));
    /// assert_eq!(result, vec![Arc::from("Model::Vehicle")]);
    ///
    /// // Namespace expose: expose Model::Vehicle::*
    /// let expose = ExposeRelationship::new(Arc::from("Model::Vehicle"), WildcardKind::Direct);
    /// let symbols: Vec<Arc<str>> = vec![
    ///     Arc::from("Model::Vehicle"),
    ///     Arc::from("Model::Vehicle::engine"),
    ///     Arc::from("Model::Vehicle::wheels"),
    ///     Arc::from("Model::Vehicle::wheels::tire"),
    /// ];
    /// let result = expose.resolve(symbols.iter().map(|s| s.as_ref()));
    /// assert_eq!(result.len(), 2); // engine and wheels, not tire (not recursive)
    ///
    /// // Recursive expose: expose Model::Vehicle::**
    /// let expose = ExposeRelationship::new(Arc::from("Model::Vehicle"), WildcardKind::Recursive);
    /// let result = expose.resolve(symbols.iter().map(|s| s.as_ref()));
    /// assert_eq!(result.len(), 3); // engine, wheels, and tire (recursive)
    /// ```
    pub fn resolve<'a, I>(&self, all_symbols: I) -> Vec<QualifiedName>
    where
        I: Iterator<Item = &'a str>,
    {
        let target_str = self.import_path.target.as_ref();

        match self.import_path.wildcard {
            WildcardKind::None => {
                // Member expose: just return the target if it exists
                all_symbols
                    .filter(|qname| *qname == target_str)
                    .map(Arc::from)
                    .collect()
            }
            WildcardKind::Direct => {
                // Namespace expose: return direct children only
                // A direct child has exactly one more :: segment after the target
                let prefix = format!("{}::", target_str);
                all_symbols
                    .filter(|qname| {
                        if let Some(rest) = qname.strip_prefix(&prefix) {
                            // Direct child: no more :: separators in the rest
                            !rest.contains("::")
                        } else {
                            false
                        }
                    })
                    .map(Arc::from)
                    .collect()
            }
            WildcardKind::Recursive => {
                // Recursive expose: return all descendants
                let prefix = format!("{}::", target_str);
                all_symbols
                    .filter(|qname| qname.starts_with(&prefix))
                    .map(Arc::from)
                    .collect()
            }
        }
    }
}

impl FilterCondition {
    /// Create a metadata filter.
    pub fn metadata(annotation: QualifiedName) -> Self {
        Self::Metadata(MetadataFilter {
            annotation,
            span: None,
        })
    }

    /// Create an expression filter (DEFERRED to Phase 4).
    pub fn expression(expr: String) -> Self {
        Self::Expression(expr)
    }

    /// Evaluate whether an element matches this filter condition.
    ///
    /// Returns `true` if the element passes the filter, `false` otherwise.
    /// For metadata filters, checks if the element has the specified annotation.
    ///
    /// # Arguments
    /// * `element_metadata` - List of metadata annotations on the element
    ///
    /// # Example
    /// ```rust
    /// use syster::hir::FilterCondition;
    /// use std::sync::Arc;
    ///
    /// let filter = FilterCondition::metadata(Arc::from("SysML::PartUsage"));
    /// let metadata: Vec<Arc<str>> = vec![Arc::from("SysML::PartUsage"), Arc::from("Doc::note")];
    /// assert!(filter.matches(&metadata));
    /// ```
    pub fn matches(&self, element_metadata: &[QualifiedName]) -> bool {
        match self {
            FilterCondition::Metadata(meta_filter) => {
                // Check if the element has the required metadata annotation
                element_metadata.iter().any(|annotation| {
                    // Exact match or suffix match (e.g., "PartUsage" matches "SysML::PartUsage")
                    **annotation == *meta_filter.annotation
                        || annotation.ends_with(&format!("::{}", meta_filter.annotation))
                })
            }
            FilterCondition::Expression(_) => {
                // Expression filters are not yet implemented (Phase 4)
                // For now, return true to not filter anything
                true
            }
        }
    }
}
