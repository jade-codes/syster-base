use crate::semantic::resolver::{Resolver, build_export_maps, resolve_imports};
use crate::semantic::workspace::{Workspace, populator::WorkspacePopulator};
use crate::syntax::SyntaxFile;
use std::path::PathBuf;

impl Workspace<SyntaxFile> {
    /// Populates the symbol table and reference index for all files
    pub fn populate_all(&mut self) -> Result<(), String> {
        let mut populator = WorkspacePopulator::new(
            &self.files,
            &mut self.symbol_table,
            &mut self.reference_index,
        );
        let populated_paths = populator.populate_all()?;

        for path in populated_paths {
            self.mark_file_populated(&path);
        }

        // Phase 2: Resolve import paths to fully qualified paths
        resolve_imports(&mut self.symbol_table);

        // Phase 3: Build export maps with fixpoint iteration
        build_export_maps(&mut self.symbol_table);

        // Re-resolve reference targets after population
        // This handles cross-file references that used simple names during population
        self.resolve_reference_targets();

        Ok(())
    }

    /// Populates only unpopulated files (for incremental updates)
    pub fn populate_affected(&mut self) -> Result<usize, String> {
        let mut populator = WorkspacePopulator::new(
            &self.files,
            &mut self.symbol_table,
            &mut self.reference_index,
        );
        let populated_paths = populator.populate_affected()?;
        let count = populated_paths.len();

        for path in populated_paths {
            self.mark_file_populated(&path);
        }

        // Phase 2: Resolve import paths to fully qualified paths
        resolve_imports(&mut self.symbol_table);

        // Phase 3: Build export maps with fixpoint iteration
        build_export_maps(&mut self.symbol_table);

        // Re-resolve reference targets after population
        self.resolve_reference_targets();

        Ok(count)
    }

    /// Populates a specific file
    pub fn populate_file(&mut self, path: &PathBuf) -> Result<(), String> {
        let mut populator = WorkspacePopulator::new(
            &self.files,
            &mut self.symbol_table,
            &mut self.reference_index,
        );
        populator.populate_file(path)?;
        self.mark_file_populated(path);

        // Phase 2: Resolve import paths to fully qualified paths
        resolve_imports(&mut self.symbol_table);

        // Phase 3: Build export maps with fixpoint iteration
        build_export_maps(&mut self.symbol_table);

        // Re-resolve reference targets for the updated file
        self.resolve_reference_targets();

        Ok(())
    }

    /// Re-resolve reference targets from simple names to qualified names.
    ///
    /// After population, some cross-file references may have been stored with simple names
    /// because import resolution hadn't run yet. This method uses the Resolver to look up
    /// the correct qualified names based on each file's scope.
    fn resolve_reference_targets(&mut self) {
        // Build a map of file path -> scope ID for resolution
        let file_scopes: std::collections::HashMap<PathBuf, usize> = self
            .symbol_table
            .iter_symbols()
            .filter_map(|sym| {
                sym.source_file()
                    .and_then(|f| self.symbol_table.get_scope_for_file(f))
                    .map(|scope| (PathBuf::from(sym.source_file().unwrap()), scope))
            })
            .collect();

        // Resolve simple (non-chain) references
        self.reference_index
            .resolve_targets(|simple_name, file, scope_id| {
                // Use scope_id from reference if available, otherwise fall back to file scope
                let scope = scope_id.or_else(|| file_scopes.get(file).copied())?;

                // Use the Resolver to look up the symbol in scope
                let resolver = Resolver::new(&self.symbol_table);
                let symbol = resolver.resolve_in_scope(simple_name, scope)?;

                Some(symbol.qualified_name().to_string())
            });

        // Resolve feature chain references (e.g., takePicture.focus)
        self.reference_index
            .resolve_chain_targets(|chain_parts, chain_index, scope_id| {
                let resolver = Resolver::new(&self.symbol_table);
                let parts: Vec<&str> = chain_parts.iter().map(|s| s.as_str()).collect();
                let symbol = resolver.resolve_feature_chain(&parts, chain_index, scope_id)?;
                Some(symbol.qualified_name().to_string())
            });
    }
}
