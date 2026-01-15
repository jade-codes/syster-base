use crate::semantic::symbol_table::Symbol;
use crate::semantic::types::SemanticError;

use crate::semantic::adapters::KermlAdapter;

impl<'a> KermlAdapter<'a> {
    pub(super) fn qualified_name(&self, name: &str) -> String {
        if self.current_namespace.is_empty() {
            name.to_string()
        } else {
            format!("{}::{}", self.current_namespace.join("::"), name)
        }
    }

    pub(super) fn enter_namespace(&mut self, name: String) {
        self.current_namespace.push(name);
        self.symbol_table.enter_scope();
    }

    pub(super) fn exit_namespace(&mut self) {
        self.current_namespace.pop();
        self.symbol_table.exit_scope();
    }

    pub(super) fn insert_symbol(&mut self, name: String, symbol: Symbol) {
        if let Err(e) = self.symbol_table.insert(name.clone(), symbol) {
            self.errors
                .push(SemanticError::duplicate_definition(name, None));
            if let Some(last_error) = self.errors.last_mut() {
                last_error.message = e;
            }
        }
    }
}
