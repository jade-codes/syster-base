use super::*;

// =============================================================================
// SysML Package/Import/Alias (Per sysml.pest)
// =============================================================================

/// Parse a package
/// Per pest: package = { prefix_metadata? ~ package_declaration ~ package_body }
/// Per pest: package_declaration = { package_token ~ identification? }
/// Per pest: package_body = { semi_colon | ( "{" ~ package_body_items ~ "}" ) }
pub fn parse_package<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::PACKAGE);

    p.expect(SyntaxKind::PACKAGE_KW);
    p.skip_trivia();

    // Optional identification
    if p.at_name_token() || p.at(SyntaxKind::LT) {
        p.parse_identification();
        p.skip_trivia();
    }

    // Package body: ; or { ... }
    parse_namespace_body(p);

    p.finish_node();
}

/// Parse a library package
/// Per pest: library_package = { standard_token? ~ library_token ~ prefix_metadata? ~ package_declaration ~ package_body }
pub fn parse_library_package<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::LIBRARY_PACKAGE);

    // Optional 'standard'
    if p.at(SyntaxKind::STANDARD_KW) {
        p.bump();
        p.skip_trivia();
    }

    p.expect(SyntaxKind::LIBRARY_KW);
    p.skip_trivia();

    p.expect(SyntaxKind::PACKAGE_KW);
    p.skip_trivia();

    // Optional identification
    if p.at_name_token() || p.at(SyntaxKind::LT) {
        p.parse_identification();
        p.skip_trivia();
    }

    // Package body
    parse_namespace_body(p);

    p.finish_node();
}

/// Parse an import
/// Per pest: import = { (namespace_import | membership_import) ~ filter_package? ~ relationship_body }
/// Per pest: import_prefix = { visibility? ~ import_token ~ all_token? }
/// Per pest: imported_membership = { qualified_name }
/// Per pest: imported_namespace = { qualified_name ~ "::" ~ "*" ~ ("::*"*")? }
pub fn parse_import<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::IMPORT);

    p.expect(SyntaxKind::IMPORT_KW);
    p.skip_trivia();

    // Optional 'all'
    if p.at(SyntaxKind::ALL_KW) {
        p.bump();
        p.skip_trivia();
    }

    // Qualified name
    p.parse_qualified_name();
    p.skip_trivia();

    // Optional wildcards: ::* or ::**, or ::**::*
    while p.at(SyntaxKind::COLON_COLON) {
        p.bump();
        p.skip_trivia();
        if p.at(SyntaxKind::STAR_STAR) {
            // Recursive wildcard: **
            p.bump();
            p.skip_trivia();
        } else if p.at(SyntaxKind::STAR) {
            // Simple wildcard: *
            p.bump();
            p.skip_trivia();
        } else {
            break;
        }
    }

    // Optional filter package: [@filter]
    if p.at(SyntaxKind::L_BRACKET) {
        parse_filter_package(p);
        p.skip_trivia();
    }

    // Relationship body: ; or { ... }
    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
    } else if p.at(SyntaxKind::L_BRACE) {
        p.parse_body();
    } else {
        error_missing_body_terminator(p, "import statement");
    }

    p.finish_node();
}

/// Parse an alias
/// Per pest: alias_member_element = { visibility? ~ alias_token ~ identification? ~ for_token ~ element_reference ~ relationship_body }
pub fn parse_alias<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::ALIAS_MEMBER);

    p.expect(SyntaxKind::ALIAS_KW);
    p.skip_trivia();

    // Optional identification (alias name)
    if p.at_name_token() || p.at(SyntaxKind::LT) {
        p.parse_identification();
        p.skip_trivia();
    }

    // 'for' keyword
    p.expect(SyntaxKind::FOR_KW);
    p.skip_trivia();

    // Element reference (qualified name)
    p.parse_qualified_name();
    p.skip_trivia();

    // Relationship body: ; or { ... }
    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
    } else if p.at(SyntaxKind::L_BRACE) {
        p.parse_body();
    } else {
        error_missing_body_terminator(p, "alias declaration");
    }

    p.finish_node();
}

/// Parse a namespace body: ; or { members* }
fn parse_namespace_body<P: SysMLParser>(p: &mut P) {
    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
    } else if p.at(SyntaxKind::L_BRACE) {
        p.parse_body();
    } else {
        error_missing_body_terminator(p, "declaration");
    }
}

/// Parse filter package: [@expression]
fn parse_filter_package<P: SysMLParser>(p: &mut P) {
    if !p.at(SyntaxKind::L_BRACKET) {
        return;
    }

    p.start_node(SyntaxKind::FILTER_PACKAGE);

    while p.at(SyntaxKind::L_BRACKET) {
        p.bump(); // [
        p.skip_trivia();

        // Optional @ prefix
        if p.at(SyntaxKind::AT) {
            p.bump();
            p.skip_trivia();
        }

        // Filter expression (qualified name or expression)
        if p.at_name_token() {
            p.parse_qualified_name();
        }
        p.skip_trivia();

        p.expect(SyntaxKind::R_BRACKET);
        p.skip_trivia();
    }

    p.finish_node(); // FILTER_PACKAGE
}
