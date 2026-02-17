use super::*;

// =============================================================================
// Annotation Parsing
// =============================================================================

/// Parse metadata annotation (@ or @@ or metadata keyword)
/// Per pest: metadata_feature = { prefix_metadata? ~ (at_symbol | metadata_token) ~ (identification ~ (":" | typed_token ~ by_token))? ~ qualified_reference_chain ~ (about_token ~ annotation ~ ("," ~ annotation)*)? ~ metadata_body }
fn parse_metadata_annotation<P: KerMLParser>(p: &mut P) {
    bump_and_skip(p); // METADATA_KW, @, or @@

    // Optional identification with typing (name : Type)
    if p.at_name_token()
        && p.peek_kind(1) != SyntaxKind::SEMICOLON
        && p.peek_kind(1) != SyntaxKind::L_BRACE
        && matches!(p.peek_kind(1), SyntaxKind::COLON | SyntaxKind::TYPED_KW)
    {
        parse_identification_and_skip(p);
        bump_and_skip(p); // COLON or TYPED_KW
        consume_if(p, SyntaxKind::BY_KW);
    }

    parse_optional_qualified_name(p);

    if p.at(SyntaxKind::ABOUT_KW) {
        bump_and_skip(p);
        p.parse_qualified_name_list();
        p.skip_trivia();
    }

    if p.at(SyntaxKind::L_BRACE) {
        parse_annotation_body(p);
    } else if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
    }
}

/// Parse locale annotation: locale "string" /* comment */
/// Per pest: Note - locale is typically part of comment_annotation and documentation
fn parse_locale_annotation<P: KerMLParser>(p: &mut P) {
    p.bump(); // locale
    p.skip_trivia_except_block_comments();

    if p.at(SyntaxKind::STRING) {
        p.bump();
        p.skip_trivia_except_block_comments();
    }

    if p.at(SyntaxKind::BLOCK_COMMENT) {
        p.bump();
    }
}

/// Parse comment/doc annotation with optional identification, locale, about
/// Check for block comment and return true if found
fn check_block_comment<P: KerMLParser>(p: &mut P) -> bool {
    if p.at(SyntaxKind::BLOCK_COMMENT) {
        p.bump();
        true
    } else {
        false
    }
}

/// Parse optional locale clause
fn parse_locale_clause<P: KerMLParser>(p: &mut P) {
    if p.at(SyntaxKind::LOCALE_KW) {
        p.bump();
        p.skip_trivia_except_block_comments();
        if p.at(SyntaxKind::STRING) {
            p.bump();
            p.skip_trivia_except_block_comments();
        }
    }
}

/// Parse 'about' clause with optional locale
fn parse_about_clause<P: KerMLParser>(p: &mut P) {
    if p.at(SyntaxKind::ABOUT_KW) {
        p.bump();
        p.skip_trivia();
        p.parse_qualified_name_list();
        p.skip_trivia_except_block_comments();
        parse_locale_clause(p);
    }
}

/// Per pest: comment_annotation = { (comment_token ~ identification? ~ (about_token ~ element_reference ~ ("," ~ element_reference)*)?)? ~ (locale_token ~ string_value)? ~ block_comment }
/// Per pest: documentation = { doc_token ~ identification? ~ (locale_token ~ string_value)? ~ (block_comment | ";")? }
fn parse_comment_doc_annotation<P: KerMLParser>(p: &mut P) {
    // comment or doc keyword already consumed
    p.skip_trivia_except_block_comments();

    if check_block_comment(p) {
        return;
    }

    // Optional identification
    if (p.at_name_token() || p.at(SyntaxKind::LT))
        && !p.at(SyntaxKind::ABOUT_KW)
        && !p.at(SyntaxKind::LOCALE_KW)
    {
        p.parse_identification();
        p.skip_trivia_except_block_comments();

        if check_block_comment(p) {
            return;
        }
    }

    parse_locale_clause(p);
    if check_block_comment(p) {
        return;
    }

    parse_about_clause(p);
    if check_block_comment(p) {
        return;
    }

    // Body or semicolon
    if p.at(SyntaxKind::L_BRACE) {
        parse_annotation_body(p);
    } else if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
    }
}

/// Parse annotation (comment, doc, locale)
/// Per Pest grammar:
/// - locale_annotation = { locale_token ~ string_value ~ block_comment? }
/// - comment_annotation = { comment_token ~ identifier? ~ (locale_token ~ quoted_name)? ~ (about_token ~ element_reference)* ~ (block_comment | semi_colon)? }
/// - documentation = { doc_token ~ identifier? ~ (locale_token ~ quoted_name)? ~ (block_comment | semi_colon)? }
pub fn parse_annotation<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::COMMENT_ELEMENT);

    if p.at(SyntaxKind::AT) || p.at(SyntaxKind::AT_AT) || p.at(SyntaxKind::METADATA_KW) {
        parse_metadata_annotation(p);
    } else if p.at(SyntaxKind::LOCALE_KW) {
        parse_locale_annotation(p);
    } else {
        // comment or doc keyword
        if p.at(SyntaxKind::COMMENT_KW) || p.at(SyntaxKind::DOC_KW) {
            p.bump();
        }
        parse_comment_doc_annotation(p);
    }

    p.finish_node();
}

/// Parse annotation body
/// Per pest: metadata_body = { ";" | "{" ~ metadata_body_element* ~ "}" }
/// Per pest: metadata_body_element = { non_feature_member | metadata_body_feature_member | alias_member | import }
fn parse_annotation_body<P: KerMLParser>(p: &mut P) {
    p.expect(SyntaxKind::L_BRACE);
    p.skip_trivia();

    // Content inside braces - for now just skip to closing brace
    let mut depth = 1;
    while depth > 0 {
        if p.at(SyntaxKind::L_BRACE) {
            depth += 1;
            p.bump();
        } else if p.at(SyntaxKind::R_BRACE) {
            depth -= 1;
            if depth > 0 {
                p.bump();
            }
        } else if p.current_kind() == SyntaxKind::ERROR {
            break; // EOF
        } else {
            p.bump();
        }
    }

    p.expect(SyntaxKind::R_BRACE);
}
