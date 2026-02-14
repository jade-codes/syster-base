use super::*;

pub fn parse_case_body<P: SysMLParser>(p: &mut P) {
    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
        return;
    }

    if !p.at(SyntaxKind::L_BRACE) {
        error_missing_body_terminator(p, "case definition");
        return;
    }

    p.start_node(SyntaxKind::NAMESPACE_BODY);
    p.bump(); // {
    p.skip_trivia();

    // Parse case body items: objective, subject, actor, case_calculation_body_item
    while !p.at(SyntaxKind::R_BRACE) && !p.at(SyntaxKind::ERROR) {
        if p.at(SyntaxKind::OBJECTIVE_KW) {
            parse_objective_usage(p);
        } else if p.at(SyntaxKind::SUBJECT_KW) {
            parse_subject_usage(p);
        } else if p.at(SyntaxKind::ACTOR_KW) {
            parse_actor_usage(p);
        } else if p.at(SyntaxKind::STAKEHOLDER_KW) {
            parse_stakeholder_usage(p);
        } else if p.at(SyntaxKind::RETURN_KW) {
            parse_sysml_parameter(p);
        } else if p.at(SyntaxKind::IDENT) {
            // Check if this looks like an expression (followed by operator) or a feature member
            let lookahead = skip_trivia_lookahead(p, 1);
            let next = p.peek_kind(lookahead);

            // If followed by expression operators, parse as expression
            if matches!(
                next,
                SyntaxKind::DOT
                    | SyntaxKind::COLON_COLON
                    | SyntaxKind::L_BRACKET
                    | SyntaxKind::L_PAREN
                    | SyntaxKind::PLUS
                    | SyntaxKind::MINUS
                    | SyntaxKind::STAR
                    | SyntaxKind::SLASH
                    | SyntaxKind::PERCENT
                    | SyntaxKind::GT
                    | SyntaxKind::LT
                    | SyntaxKind::EQ_EQ
                    | SyntaxKind::BANG_EQ
            ) {
                // Parse as expression (shared expression grammar from kerml_expressions.pest)
                parse_expression(p);
            } else {
                // Parse as package body element (feature member)
                parse_package_body_element(p);
            }
        } else {
            // Other case body items (calculation body, annotations)
            parse_package_body_element(p);
        }
        p.skip_trivia();
    }

    p.expect(SyntaxKind::R_BRACE);
    p.finish_node();
}

// Parse metadata body (for metadata definitions)
pub fn parse_metadata_body<P: SysMLParser>(p: &mut P) {
    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
        return;
    }

    if !p.at(SyntaxKind::L_BRACE) {
        error_missing_body_terminator(p, "metadata definition");
        return;
    }

    p.start_node(SyntaxKind::NAMESPACE_BODY);
    p.bump(); // {
    p.skip_trivia();

    // Metadata body can contain metadata_body_usage items
    while !p.at(SyntaxKind::R_BRACE) && !p.at(SyntaxKind::ERROR) {
        // Metadata body usage pattern: [ref] [:>>] identifier
        // Need to distinguish from other body elements
        let is_metadata_usage = if p.at(SyntaxKind::REF_KW) {
            // Check if ref is followed by :>> or identifier (not a usage keyword)
            let lookahead = skip_trivia_lookahead(p, 1);
            let next = p.peek_kind(lookahead);
            matches!(
                next,
                SyntaxKind::COLON_GT_GT
                    | SyntaxKind::COLON_GT
                    | SyntaxKind::REDEFINES_KW
                    | SyntaxKind::IDENT
            )
        } else if p.at(SyntaxKind::COLON_GT_GT) || p.at(SyntaxKind::REDEFINES_KW) {
            // Starts with redefines operator
            true
        } else if p.at(SyntaxKind::IDENT) {
            // Just an identifier - could be metadata usage or other element
            // In metadata body, bare identifiers are metadata body usages
            true
        } else {
            false
        };

        if is_metadata_usage {
            parse_metadata_body_usage(p);
        } else {
            // Other body elements (imports, relationships, annotations)
            parse_package_body_element(p);
        }
        p.skip_trivia();
    }

    p.expect(SyntaxKind::R_BRACE);
    p.finish_node();
}

// Parse metadata body usage: ref? :>>? identifier typing? specializations? default? meta? value? body
// Handles patterns like:
//   - `ref :>> annotatedElement : SysML::Usage;`
//   - `:>> baseType default global_sd meta SysML::PortUsage;`
fn parse_metadata_body_usage<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    // Optional 'ref'
    if p.at(SyntaxKind::REF_KW) {
        p.bump();
        p.skip_trivia();
    }

    // Optional redefines operator - wrap in SPECIALIZATION node for AST extraction
    if p.at(SyntaxKind::COLON_GT_GT) || p.at(SyntaxKind::COLON_GT) || p.at(SyntaxKind::REDEFINES_KW)
    {
        p.start_node(SyntaxKind::SPECIALIZATION);
        p.bump(); // :>> or :> or redefines
        p.skip_trivia();

        // Required identifier (as qualified name)
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        } else {
            p.error("expected identifier in metadata body usage");
        }
        p.finish_node(); // SPECIALIZATION
    } else {
        // No redefines - parse name directly as NAME node for hoverable symbol
        if p.at(SyntaxKind::IDENT) {
            p.start_node(SyntaxKind::NAME);
            p.bump();
            p.finish_node();
            p.skip_trivia();
        } else {
            p.error("expected identifier in metadata body usage");
        }
    }

    // Optional typing
    parse_optional_typing(p);

    // Optional specializations
    parse_specializations_with_skip(p);

    // Optional 'default' clause with expression
    // Pattern: `default <expression>`
    if p.at(SyntaxKind::DEFAULT_KW) {
        p.bump(); // default
        p.skip_trivia();
        // The default value is an expression (usually an identifier reference)
        if p.can_start_expression() {
            parse_expression(p);
            p.skip_trivia();
        }
    }

    // Optional 'meta' clause with type reference
    // Pattern: `meta <qualified_name>`
    // We use TYPING node to wrap this since it functions similarly
    if p.at(SyntaxKind::META_KW) {
        p.start_node(SyntaxKind::TYPING);
        p.bump(); // meta
        p.skip_trivia();
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        }
        p.finish_node();
    }

    // Optional value (= expression or 'as' cast)
    if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) {
        p.bump();
        p.skip_trivia();
        parse_expression(p);
        p.skip_trivia();
    }

    // Body (semicolon or nested metadata body)
    if p.at(SyntaxKind::L_BRACE) {
        parse_metadata_body(p);
    } else {
        p.expect(SyntaxKind::SEMICOLON);
    }

    p.finish_node();
}

/// Parse calc body for SysML (extends KerML calc body with behavior usages)
/// Per pest: calculation_body_item includes behavior_usage_member (perform, send, etc.)
/// and result_expression_member (final expression without semicolon)
pub fn parse_sysml_calc_body<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::NAMESPACE_BODY);

    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
    } else if p.at(SyntaxKind::L_BRACE) {
        p.bump();
        p.skip_trivia();

        while !p.at(SyntaxKind::R_BRACE) && !p.at(SyntaxKind::ERROR) {
            let start_pos = p.get_pos();

            // Parameters (in, out) - treat as SysML usages to handle ref prefix
            if p.at_any(&[SyntaxKind::IN_KW, SyntaxKind::OUT_KW, SyntaxKind::INOUT_KW]) {
                parse_usage(p);
            }
            // RETURN_KW can be either a return parameter or return expression
            else if p.at(SyntaxKind::RETURN_KW) {
                // Look ahead to distinguish: return <name>? : ... vs return <expr>
                let lookahead = skip_trivia_lookahead(p, 1);
                let after_return = p.peek_kind(lookahead);

                // If return is followed directly by colon, it's a parameter: return : Type
                if after_return == SyntaxKind::COLON || after_return == SyntaxKind::TYPED_KW {
                    parse_usage(p);
                } else if after_return == SyntaxKind::IDENT {
                    let after_that = p.peek_kind(skip_trivia_lookahead(p, lookahead + 1));
                    // If followed by name + colon/typing/default, it's a parameter declaration
                    // EQ handles: return p = expr; (named result with default value)
                    if after_that == SyntaxKind::COLON
                        || after_that == SyntaxKind::TYPED_KW
                        || after_that == SyntaxKind::L_BRACKET
                        || after_that == SyntaxKind::COLON_GT
                        || after_that == SyntaxKind::COLON_GT_GT
                        || after_that == SyntaxKind::SEMICOLON
                        || after_that == SyntaxKind::EQ
                    {
                        parse_usage(p);
                    } else {
                        // return expression statement
                        parse_return_expression(p);
                    }
                } else if is_usage_keyword(after_return) {
                    // return part x; or return attribute y;
                    parse_usage(p);
                } else {
                    // return expression statement
                    parse_return_expression(p);
                }
            }
            // Behavior usages (perform, send, accept, etc.)
            else if p.at(SyntaxKind::PERFORM_KW) {
                parse_perform_action(p);
            } else if p.at(SyntaxKind::SEND_KW) {
                parse_send_action(p);
            } else if p.at(SyntaxKind::ACCEPT_KW) {
                parse_accept_action(p);
            }
            // General namespace elements (definitions, usages, etc.)
            else if p.at_any(&[
                SyntaxKind::ATTRIBUTE_KW,
                SyntaxKind::PART_KW,
                SyntaxKind::ITEM_KW,
                SyntaxKind::CALC_KW,
                SyntaxKind::CONSTRAINT_KW,
                SyntaxKind::ACTION_KW,
                SyntaxKind::DOC_KW,
                SyntaxKind::COMMENT_KW,
                SyntaxKind::PRIVATE_KW,
                SyntaxKind::PUBLIC_KW,
                SyntaxKind::PROTECTED_KW,
            ]) {
                parse_package_body_element(p);
            }
            // Result expression (identifier, new, literal, or any expression start)
            // Per sysml.pest: calculation_body_item includes result_expression_member
            else if p.can_start_expression() {
                parse_expression(p);
                p.skip_trivia();
                // Optional semicolon for expression statements
                if p.at(SyntaxKind::SEMICOLON) {
                    p.bump();
                }
            } else {
                parse_package_body_element(p);
            }

            p.skip_trivia();

            if p.get_pos() == start_pos && !p.at(SyntaxKind::R_BRACE) {
                let got = if let Some(text) = p.current_token_text() {
                    format!("'{}'", text)
                } else {
                    p.current_kind().display_name().to_string()
                };
                p.error(format!("unexpected {} in calc body", got));
                p.bump();
            }
        }

        p.expect(SyntaxKind::R_BRACE);
    } else {
        error_missing_body_terminator(p, "calc definition");
    }

    p.finish_node();
}
