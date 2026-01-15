use crate::semantic::types::SemanticError;
use crate::syntax::kerml::KerMLFile;
use crate::syntax::kerml::ast::Element;

use crate::semantic::adapters::KermlAdapter;

impl<'a> KermlAdapter<'a> {
    pub fn populate(&mut self, file: &KerMLFile) -> Result<(), Vec<SemanticError>> {
        // If there's a file-level namespace, enter it first
        let namespace_name = if let Some(ref ns) = file.namespace {
            self.visit_namespace(ns);
            Some(ns.name.clone())
        } else {
            None
        };

        // Process all elements in the file
        for element in file.elements.iter() {
            // Skip Package element if it's the same as the file-level namespace
            // (we've already processed it via visit_namespace above)
            if let Element::Package(p) = element
                && let Some(ref ns_name) = namespace_name
                && p.name.as_ref() == Some(ns_name)
            {
                // This is the file-level package - skip it, we've already entered its namespace
                // But still process its children
                for child in &p.elements {
                    self.visit_element(child);
                }
                continue;
            }

            self.visit_element(element);
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(std::mem::take(&mut self.errors))
        }
    }
}
