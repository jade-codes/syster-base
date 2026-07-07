use super::*;

// =============================================================================
// Annotation Parsing
// =============================================================================

// tag::parse_metadata_annotation[]
/// Parse metadata annotation (@ or @@ or metadata keyword)
/// Grammar: see docs/grammar-mapping.adoc#parse_metadata_annotation
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
// end::parse_metadata_annotation[]

// tag::parse_locale_annotation[]
/// Parse locale annotation: locale "string" /* comment */
/// Grammar: see docs/grammar-mapping.adoc#parse_locale_annotation
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
// end::parse_locale_annotation[]

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

// tag::parse_comment_doc_annotation[]
/// Parse the shared body of a `comment` or `doc` annotation (keyword already consumed)
/// Grammar: see docs/grammar-mapping.adoc#parse_comment_doc_annotation
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
// end::parse_comment_doc_annotation[]

// tag::parse_annotation[]
/// Parse annotation (comment, doc, locale)
/// Grammar: see docs/grammar-mapping.adoc#parse_annotation
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
// end::parse_annotation[]

// tag::parse_annotation_body[]
/// Parse annotation body
/// Grammar: see docs/grammar-mapping.adoc#parse_annotation_body
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
// end::parse_annotation_body[]
