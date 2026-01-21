use crate::semantic::types::SemanticError;
use crate::syntax::sysml::ast::{Element, SysMLFile};
use crate::syntax::sysml::visitor::AstVisitor;

use crate::semantic::adapters::SysmlAdapter;

/// Work item for iterative AST traversal
enum WorkItem<'a> {
    /// Visit an element
    Visit(&'a Element),
    /// Exit a namespace (deferred action after processing package children)
    ExitNamespace,
}

impl<'a> SysmlAdapter<'a> {
    /// Populates the symbol table by visiting all elements in the SysML file.
    ///
    /// # Errors
    ///
    /// Returns a vector of `SemanticError` if any semantic errors are encountered
    /// during population, such as duplicate symbol definitions.
    pub fn populate(&mut self, file: &SysMLFile) -> Result<(), Vec<SemanticError>> {
        // If there's a file-level namespace, enter it first
        let namespace_name = if let Some(ref ns) = file.namespace {
            self.visit_namespace(ns);
            Some(ns.name.clone())
        } else {
            None
        };

        // Collect initial work items
        let mut initial_elements: Vec<&Element> = Vec::new();

        for element in file.elements.iter() {
            // Skip Package element if it's the same as the file-level namespace
            // (we've already processed it via visit_namespace above)
            if let Element::Package(p) = element
                && let Some(ref ns_name) = namespace_name
                && p.name.as_ref() == Some(ns_name)
            {
                // This is the file-level package - skip it, we've already entered its namespace
                // But still process its children
                initial_elements.extend(p.elements.iter());
                continue;
            }
            initial_elements.push(element);
        }

        // Process elements iteratively
        self.visit_elements_iterative(&initial_elements);

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(std::mem::take(&mut self.errors))
        }
    }

    /// Iteratively visit a slice of elements using an explicit work stack.
    /// This avoids deep recursion that can cause stack overflow on deeply nested ASTs.
    pub(super) fn visit_elements_iterative(&mut self, elements: &[&Element]) {
        // Work stack - we process items from the end (LIFO)
        let mut work: Vec<WorkItem> = elements.iter().rev().map(|e| WorkItem::Visit(e)).collect();

        while let Some(item) = work.pop() {
            match item {
                WorkItem::Visit(element) => {
                    match element {
                        Element::Package(p) => {
                            self.visit_package(p);
                            // Schedule exit_namespace BEFORE children so it runs AFTER them (LIFO)
                            if p.name.is_some() {
                                work.push(WorkItem::ExitNamespace);
                            }
                            // Add children in reverse order so they're processed in order
                            for child in p.elements.iter().rev() {
                                work.push(WorkItem::Visit(child));
                            }
                        }
                        Element::Definition(d) => self.visit_definition(d),
                        Element::Usage(u) => self.visit_usage(u),
                        Element::Comment(c) => self.visit_comment(c),
                        Element::Import(i) => self.visit_import(i),
                        Element::Alias(a) => self.visit_alias(a),
                        Element::Dependency(dep) => self.visit_dependency(dep),
                        Element::Filter(f) => self.visit_filter(f),
                    }
                }
                WorkItem::ExitNamespace => {
                    self.exit_namespace();
                }
            }
        }
    }
}
