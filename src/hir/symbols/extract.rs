//! Unified extraction entry points and AST member dispatch.

use crate::base::FileId;
use crate::parser::NamespaceMember;

use super::context::ExtractionContext;
use super::extract_definition::extract_definition_from_ast;
use super::extract_leaf::{
    extract_alias_from_ast, extract_comment_from_ast, extract_dependency_from_ast,
    extract_import_from_ast,
};
use super::extract_package::{
    extract_filter_from_ast, extract_library_package_from_ast, extract_package_from_ast,
};
use super::extract_special::{
    extract_accept_action_from_ast, extract_bare_transition_from_ast, extract_bind_from_ast,
    extract_connect_usage_from_ast, extract_connector_from_ast, extract_control_node_from_ast,
    extract_for_loop_from_ast, extract_if_action_from_ast, extract_send_action_from_ast,
    extract_state_subaction_from_ast, extract_succession_from_ast, extract_while_loop_from_ast,
};
use super::extract_usage::{extract_metadata_member_from_ast, extract_usage_from_ast};
use super::types::{ExtractionResult, HirSymbol};

/// Extract all symbols from a parsed syntax file.
///
/// Handles both SysML and KerML through a unified code path.
pub fn extract_symbols_unified(file: FileId, syntax: &crate::syntax::SyntaxFile) -> Vec<HirSymbol> {
    extract_with_filters(file, syntax).symbols
}

/// Extract symbols and filters from any syntax file.
///
/// Returns both symbols and scope filter information for import filtering.
pub fn extract_with_filters(file: FileId, syntax: &crate::syntax::SyntaxFile) -> ExtractionResult {
    let mut result = ExtractionResult::default();
    let line_index = syntax.line_index();
    let mut context = ExtractionContext {
        file,
        prefix: String::new(),
        anon_counter: 0,
        scope_stack: Vec::new(),
        line_index,
    };

    // Get the rowan SourceFile and iterate over its members
    if let Some(source_file) = syntax.source_file() {
        for member in source_file.members() {
            extract_from_ast_member(&mut result, &mut context, &member);
        }
    }

    result
}

/// Dispatch extraction for a single AST NamespaceMember.
///
/// Each variant is handled by a dedicated AST-direct extraction function.
pub(super) fn extract_from_ast_member(
    result: &mut ExtractionResult,
    ctx: &mut ExtractionContext,
    member: &NamespaceMember,
) {
    match member {
        // Phase 1: leaf extractors — consume AST directly
        NamespaceMember::Comment(comment) => {
            extract_comment_from_ast(&mut result.symbols, ctx, comment)
        }
        NamespaceMember::Alias(alias) => {
            extract_alias_from_ast(&mut result.symbols, ctx, alias)
        }
        NamespaceMember::Import(import) => {
            extract_import_from_ast(result, ctx, import)
        }
        NamespaceMember::Dependency(dep) => {
            extract_dependency_from_ast(&mut result.symbols, ctx, dep)
        }
        // Phase 3: package extractors
        NamespaceMember::Package(pkg) => {
            extract_package_from_ast(result, ctx, pkg)
        }
        NamespaceMember::LibraryPackage(pkg) => {
            extract_library_package_from_ast(result, ctx, pkg)
        }
        // Phase 6: filter/expose — consume AST directly
        NamespaceMember::Filter(filter) => {
            extract_filter_from_ast(result, ctx, filter)
        }
        // Phase 4: definition extraction
        NamespaceMember::Definition(def) => {
            extract_definition_from_ast(&mut result.symbols, ctx, def)
        }
        // Phase 5: usage extraction (main Usage variant)
        NamespaceMember::Usage(usage) => {
            extract_usage_from_ast(&mut result.symbols, ctx, usage)
        }
        // Phase 5: special NamespaceMember variants → extract as usage
        NamespaceMember::Metadata(meta) => {
            extract_metadata_member_from_ast(&mut result.symbols, ctx, meta)
        }
        NamespaceMember::Bind(bind) => {
            extract_bind_from_ast(&mut result.symbols, ctx, bind);
        }
        NamespaceMember::Succession(succ) => {
            extract_succession_from_ast(&mut result.symbols, ctx, succ);
        }
        NamespaceMember::Transition(trans) => {
            extract_bare_transition_from_ast(&mut result.symbols, ctx, trans);
        }
        NamespaceMember::Connector(conn) => {
            extract_connector_from_ast(&mut result.symbols, ctx, conn);
        }
        NamespaceMember::ConnectUsage(conn) => {
            extract_connect_usage_from_ast(&mut result.symbols, ctx, conn);
        }
        NamespaceMember::SendAction(send) => {
            extract_send_action_from_ast(&mut result.symbols, ctx, send);
        }
        NamespaceMember::AcceptAction(accept) => {
            extract_accept_action_from_ast(&mut result.symbols, ctx, accept);
        }
        NamespaceMember::StateSubaction(sub) => {
            extract_state_subaction_from_ast(&mut result.symbols, ctx, sub);
        }
        NamespaceMember::ControlNode(node) => {
            extract_control_node_from_ast(&mut result.symbols, ctx, node);
        }
        NamespaceMember::ForLoop(for_loop) => {
            extract_for_loop_from_ast(&mut result.symbols, ctx, for_loop);
        }
        NamespaceMember::IfAction(if_action) => {
            extract_if_action_from_ast(&mut result.symbols, ctx, if_action);
        }
        NamespaceMember::WhileLoop(while_loop) => {
            extract_while_loop_from_ast(&mut result.symbols, ctx, while_loop);
        }
    }
}

/// Extract from an AST NamespaceMember into a symbol list (no filter support).
/// Used for nested extraction within definitions/usages.
pub(super) fn extract_from_ast_member_into_symbols(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    member: &NamespaceMember,
) {
    match member {
        NamespaceMember::Comment(comment) => {
            extract_comment_from_ast(symbols, ctx, comment)
        }
        NamespaceMember::Alias(alias) => {
            extract_alias_from_ast(symbols, ctx, alias)
        }
        NamespaceMember::Import(import) => {
            // For nested imports (inside definitions), ignore filters
            let mut result = ExtractionResult::default();
            extract_import_from_ast(&mut result, ctx, import);
            symbols.extend(result.symbols);
        }
        NamespaceMember::Dependency(dep) => {
            extract_dependency_from_ast(symbols, ctx, dep)
        }
        NamespaceMember::Package(pkg) => {
            let mut result = ExtractionResult::default();
            extract_package_from_ast(&mut result, ctx, pkg);
            symbols.extend(result.symbols);
        }
        NamespaceMember::LibraryPackage(pkg) => {
            let mut result = ExtractionResult::default();
            extract_library_package_from_ast(&mut result, ctx, pkg);
            symbols.extend(result.symbols);
        }
        NamespaceMember::Filter(_) => {
            // Filters inside definitions don't produce symbols
        }
        NamespaceMember::Definition(def) => {
            extract_definition_from_ast(symbols, ctx, def)
        }
        NamespaceMember::Usage(usage) => {
            extract_usage_from_ast(symbols, ctx, usage)
        }
        NamespaceMember::Metadata(meta) => {
            extract_metadata_member_from_ast(symbols, ctx, meta)
        }
        NamespaceMember::Bind(bind) => {
            extract_bind_from_ast(symbols, ctx, bind)
        }
        NamespaceMember::Succession(succ) => {
            extract_succession_from_ast(symbols, ctx, succ)
        }
        NamespaceMember::Transition(trans) => {
            extract_bare_transition_from_ast(symbols, ctx, trans)
        }
        NamespaceMember::Connector(conn) => {
            extract_connector_from_ast(symbols, ctx, conn)
        }
        NamespaceMember::ConnectUsage(conn) => {
            extract_connect_usage_from_ast(symbols, ctx, conn)
        }
        NamespaceMember::SendAction(send) => {
            extract_send_action_from_ast(symbols, ctx, send)
        }
        NamespaceMember::AcceptAction(accept) => {
            extract_accept_action_from_ast(symbols, ctx, accept)
        }
        NamespaceMember::StateSubaction(sub) => {
            extract_state_subaction_from_ast(symbols, ctx, sub)
        }
        NamespaceMember::ControlNode(node) => {
            extract_control_node_from_ast(symbols, ctx, node)
        }
        NamespaceMember::ForLoop(for_loop) => {
            extract_for_loop_from_ast(symbols, ctx, for_loop)
        }
        NamespaceMember::IfAction(if_action) => {
            extract_if_action_from_ast(symbols, ctx, if_action)
        }
        NamespaceMember::WhileLoop(while_loop) => {
            extract_while_loop_from_ast(symbols, ctx, while_loop)
        }
    }
}
