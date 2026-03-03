use super::*;

// =============================================================================
// SysML-specific Specialization, Annotation, and Relationship Parsing
// These are SysML-native implementations that don't depend on kerml.rs
// =============================================================================

/// Parse feature specializations (SysML-specific)
/// Per SysML Pest grammar:
/// feature_specialization_part = feature_specialization+ ~ multiplicity_part ~ feature_specialization*
///                              | feature_specialization+
///                              | multiplicity_part ~ feature_specialization*
///                              | multiplicity_part
/// feature_specialization = typings | subsettings | references | crosses | redefinitions
/// Per pest: feature_specialization = { typing | subsetting | redefinition | reference_subsetting | featuring | conjugation | ... }\n/// Per pest: typing = { \":\" ~ qualified_name ~ (\",\" ~ qualified_name)* | \"typed\" ~ \"by\" ~ qualified_name }\n/// Pattern: Handles all specialization operators: :, :>, :>>, ::>, typed, subsets, redefines, etc.
pub fn parse_specializations<P: SysMLParser>(p: &mut P) {
    while p.at_any(&[
        SyntaxKind::COLON,
        SyntaxKind::TYPED_KW,
        // Note: OF_KW removed - it's handled separately in message/flow parsing
        SyntaxKind::COLON_GT,
        SyntaxKind::COLON_GT_GT,
        SyntaxKind::COLON_COLON_GT,
        SyntaxKind::SPECIALIZES_KW,
        SyntaxKind::SUBSETS_KW,
        SyntaxKind::REDEFINES_KW,
        SyntaxKind::REFERENCES_KW,
        SyntaxKind::CONJUGATES_KW,
        SyntaxKind::TILDE,
        SyntaxKind::DISJOINT_KW,
        SyntaxKind::INTERSECTS_KW,
        SyntaxKind::DIFFERENCES_KW,
        SyntaxKind::UNIONS_KW,
        SyntaxKind::CHAINS_KW,
        SyntaxKind::INVERSE_KW,
        SyntaxKind::FEATURING_KW,
        SyntaxKind::CROSSES_KW,
        SyntaxKind::FAT_ARROW,
    ]) {
        // Handle typing specially as it has different structure
        if p.at(SyntaxKind::COLON) || p.at(SyntaxKind::TYPED_KW) {
            p.parse_typing();
            p.skip_trivia();
            continue;
        }

        p.start_node(SyntaxKind::SPECIALIZATION);

        let keyword = p.current_kind();
        p.bump();
        p.skip_trivia();

        if (keyword == SyntaxKind::DISJOINT_KW && p.at(SyntaxKind::FROM_KW))
            || (keyword == SyntaxKind::INVERSE_KW && p.at(SyntaxKind::OF_KW))
        {
            p.bump();
            p.skip_trivia();
        }

        p.parse_qualified_name();
        p.finish_node();
        p.skip_trivia();

        while p.at(SyntaxKind::COMMA) {
            p.bump();
            p.skip_trivia();
            p.start_node(SyntaxKind::SPECIALIZATION);
            p.parse_qualified_name();
            p.finish_node();
            p.skip_trivia();
        }
    }
}

/// Parse annotation (comment, doc, locale) - SysML-specific
/// Per SysML Pest grammar:
/// - locale_annotation = { locale_token ~ string_value ~ block_comment? }
/// - comment_annotation = { comment_token ~ identifier? ~ (locale_token ~ quoted_name)? ~ (about_token ~ element_reference)* ~ (block_comment | semi_colon)? }
/// - documentation = { doc_token ~ identifier? ~ (locale_token ~ quoted_name)? ~ (block_comment | semi_colon)? }
pub fn parse_annotation<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::COMMENT_ELEMENT);

    // Metadata feature: @ or @@ or metadata keyword followed by reference
    if p.at(SyntaxKind::AT) || p.at(SyntaxKind::AT_AT) || p.at(SyntaxKind::METADATA_KW) {
        // All annotation markers get bumped the same way
        p.bump(); // metadata, @, or @@
        p.skip_trivia();

        // Optional identification with typing
        if p.at_name_token()
            && p.peek_kind(1) != SyntaxKind::SEMICOLON
            && p.peek_kind(1) != SyntaxKind::L_BRACE
        {
            // Could be identification if followed by : or typed
            let next = p.peek_kind(1);
            if next == SyntaxKind::COLON || next == SyntaxKind::TYPED_KW {
                p.parse_identification();
                p.skip_trivia();
                if p.at(SyntaxKind::COLON) || p.at(SyntaxKind::TYPED_KW) {
                    p.bump();
                    p.skip_trivia();
                    if p.at(SyntaxKind::BY_KW) {
                        p.bump();
                        p.skip_trivia();
                    }
                }
            }
        }

        // Qualified reference
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        }

        // Optional 'about' clause
        if p.at(SyntaxKind::ABOUT_KW) {
            p.bump();
            p.skip_trivia();
            p.parse_qualified_name_list();
            p.skip_trivia();
        }

        // Body or semicolon
        if p.at(SyntaxKind::L_BRACE) {
            parse_annotation_body(p);
        } else if p.at(SyntaxKind::SEMICOLON) {
            p.bump();
        }

        p.finish_node();
        return;
    }

    // Locale annotation: locale "en_US" /* text */
    if p.at(SyntaxKind::LOCALE_KW) {
        p.bump();
        p.skip_trivia_except_block_comments();

        // String value after locale
        if p.at(SyntaxKind::STRING) {
            p.bump();
            p.skip_trivia_except_block_comments();
        }

        // Optional block comment content
        if p.at(SyntaxKind::BLOCK_COMMENT) {
            p.bump();
        }

        p.finish_node();
        return;
    }

    // comment or doc keyword
    if p.at(SyntaxKind::COMMENT_KW) || p.at(SyntaxKind::DOC_KW) {
        p.bump();
    }

    p.skip_trivia_except_block_comments();

    // Check for block comment content first (doc /* text */)
    if p.at(SyntaxKind::BLOCK_COMMENT) {
        p.bump();
        p.finish_node();
        return;
    }

    // Optional identification (can be identifier or short name with <)
    if p.at_name_token() || p.at(SyntaxKind::LT) {
        // Check this isn't 'about' or 'locale'
        if !p.at(SyntaxKind::ABOUT_KW) && !p.at(SyntaxKind::LOCALE_KW) {
            p.parse_identification();
            p.skip_trivia_except_block_comments();
        }
    }

    // Check for block comment after identification
    if p.at(SyntaxKind::BLOCK_COMMENT) {
        p.bump();
        p.finish_node();
        return;
    }

    // Optional locale (can appear after identification)
    if p.at(SyntaxKind::LOCALE_KW) {
        p.bump();
        p.skip_trivia_except_block_comments();
        if p.at(SyntaxKind::STRING) {
            p.bump();
            p.skip_trivia_except_block_comments();
        }
    }

    // Check for block comment after locale
    if p.at(SyntaxKind::BLOCK_COMMENT) {
        p.bump();
        p.finish_node();
        return;
    }

    // Optional 'about' targets
    if p.at(SyntaxKind::ABOUT_KW) {
        p.bump();
        p.skip_trivia();
        p.parse_qualified_name_list();
        p.skip_trivia_except_block_comments();

        // locale can also appear after 'about'
        if p.at(SyntaxKind::LOCALE_KW) {
            p.bump();
            p.skip_trivia_except_block_comments();
            if p.at(SyntaxKind::STRING) {
                p.bump();
                p.skip_trivia_except_block_comments();
            }
        }
    }

    // Check for block comment after 'about'
    if p.at(SyntaxKind::BLOCK_COMMENT) {
        p.bump();
        p.finish_node();
        return;
    }

    // Body or semicolon
    if p.at(SyntaxKind::L_BRACE) {
        parse_annotation_body(p);
    } else if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
    }

    p.finish_node();
}

/// Parse annotation body
fn parse_annotation_body<P: SysMLParser>(p: &mut P) {
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

/// Parse standalone relationship declarations (SysML-specific)
/// E.g., `specialization Super subclassifier A specializes B;`
/// E.g., `subclassifier C specializes A;`
/// E.g., `redefinition MyRedef redefines x :>> y;`
/// Per SysML Pest grammar: specialization_prefix ~ relationship_keyword ~ from ~ operator ~ to ~ relationship_body
pub fn parse_standalone_relationship<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::RELATIONSHIP);

    // Handle optional 'specialization' prefix with optional identification
    let _has_specialization = if p.at(SyntaxKind::SPECIALIZATION_KW) {
        p.bump(); // specialization
        p.skip_trivia();

        // Check for optional identification after 'specialization'
        // The next token should be one of the relationship keywords if no identification
        if p.at_name_token()
            && !p.at_any(&[
                SyntaxKind::SUBCLASSIFIER_KW,
                SyntaxKind::SUBTYPE_KW,
                SyntaxKind::SUBSET_KW,
                SyntaxKind::REDEFINITION_KW,
                SyntaxKind::TYPING_KW,
            ])
        {
            p.parse_identification();
            p.skip_trivia();
        }
        true
    } else {
        false
    };

    // Handle special featuring syntax: featuring [id? of]? feature by type
    if p.at(SyntaxKind::FEATURING_KW) {
        p.bump(); // featuring
        p.skip_trivia();

        // Check for optional identification + 'of'
        // We can tell by looking ahead for 'of' after a name
        if p.at_name_token() {
            // Parse first name (could be id or feature)
            p.parse_identification();
            p.skip_trivia();

            // If 'of' follows, parse the actual feature reference
            if p.at(SyntaxKind::OF_KW) {
                p.bump(); // of
                p.skip_trivia();
                if p.at_name_token() {
                    p.parse_qualified_name();
                    p.skip_trivia();
                }
            }
            // Otherwise the identification was the feature reference itself
        }

        // Parse 'by' clause
        if p.at(SyntaxKind::BY_KW) {
            p.bump(); // by
            p.skip_trivia();
            if p.at_name_token() {
                p.parse_qualified_name();
                p.skip_trivia();
            }
        }

        p.parse_body();
        p.finish_node();
        return;
    }

    // Handle special typing syntax: [specialization id?]? typing feature (':' | 'typed by') type
    if p.at(SyntaxKind::TYPING_KW) {
        p.bump(); // typing
        p.skip_trivia();

        // Parse the feature being typed
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        }

        // Parse typing operator (: or 'typed by')
        if p.at(SyntaxKind::COLON) || p.at(SyntaxKind::TYPED_KW) {
            p.parse_typing();
        }

        p.parse_body();
        p.finish_node();
        return;
    }

    // Handle special conjugation syntax: [conjugation id?]? conjugate type1 ('~' | 'conjugates') type2
    if p.at(SyntaxKind::CONJUGATION_KW) {
        p.bump(); // conjugation
        p.skip_trivia();

        // Optional identification
        if p.at_name_token() && !p.at(SyntaxKind::CONJUGATE_KW) {
            p.parse_identification();
            p.skip_trivia();
        }

        // Expect 'conjugate' keyword
        if p.at(SyntaxKind::CONJUGATE_KW) {
            p.bump(); // conjugate
            p.skip_trivia();
        }

        // Parse first type (the conjugate type)
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        }

        // Parse 'conjugates' or '~' operator
        if p.at(SyntaxKind::CONJUGATES_KW) || p.at(SyntaxKind::TILDE) {
            p.bump();
            p.skip_trivia();
        }

        // Parse second type (the original type)
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        }

        p.parse_body();
        p.finish_node();
        return;
    }

    // Handle the relationship keyword (subclassifier, subtype, subset, redefinition, etc.)
    if p.at_any(STANDALONE_RELATIONSHIP_KEYWORDS) {
        p.bump(); // relationship keyword
        p.skip_trivia();
    }

    // Parse the source element (before the operator)
    if p.at_name_token() {
        p.parse_qualified_name();
        p.skip_trivia();
    }

    // Parse the operator (specializes/:>, subsets/:>, redefines/:>>, etc.)
    if p.at_any(RELATIONSHIP_OPERATORS) {
        p.bump(); // operator
        p.skip_trivia();
    }

    // Parse the target element (after the operator)
    if p.at_name_token() {
        p.parse_qualified_name();
        p.skip_trivia();
    }

    // Parse body or semicolon
    p.parse_body();

    p.finish_node();
}

/// Parse SysML parameter (return, in, out, inout)\n/// This extends KerML parameters with SysML-specific prefixes like REF_KW\n/// Per pest: feature_member = { direction? ~ (usage_prefix* ~ usage_element | owned_feature_declaration) }\n/// Per pest: direction = { \"in\" | \"out\" | \"inout\" }\n/// Per pest: usage_prefix = { ref_prefix | abstract_prefix | readonly_prefix | derived_prefix | end_prefix | ... }\n/// Pattern: in|out|inout|return [ref|readonly|...] [usage_keyword] [<name>|:>> <ref>] [mult] [typing] [specializations] [default] semicolon
pub fn parse_sysml_parameter<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    // Parameter direction keyword
    if p.at_any(&[
        SyntaxKind::IN_KW,
        SyntaxKind::OUT_KW,
        SyntaxKind::INOUT_KW,
        SyntaxKind::END_KW,
        SyntaxKind::RETURN_KW,
    ]) {
        p.bump();
    }
    p.skip_trivia();

    // SysML-specific prefixes (ref, readonly, etc.)
    while p.at_any(&[
        SyntaxKind::REF_KW,
        SyntaxKind::READONLY_KW,
        SyntaxKind::DERIVED_KW,
        SyntaxKind::VAR_KW,
        SyntaxKind::COMPOSITE_KW,
        SyntaxKind::PORTION_KW,
        SyntaxKind::MEMBER_KW,
        SyntaxKind::ABSTRACT_KW,
    ]) {
        p.bump();
        p.skip_trivia();
    }

    // Optional usage keyword (attribute, part, etc.)
    if p.at_any(&[
        SyntaxKind::ATTRIBUTE_KW,
        SyntaxKind::PART_KW,
        SyntaxKind::ITEM_KW,
        SyntaxKind::CALC_KW,
        SyntaxKind::ACTION_KW,
        SyntaxKind::STATE_KW,
        SyntaxKind::PORT_KW,
    ]) {
        p.bump();
        p.skip_trivia();
    }

    // Redefines/subsets or identification
    if p.at(SyntaxKind::REDEFINES_KW)
        || p.at(SyntaxKind::COLON_GT_GT)
        || p.at(SyntaxKind::SUBSETS_KW)
        || p.at(SyntaxKind::COLON_GT)
    {
        p.bump();
        p.skip_trivia();
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        }
    } else if p.at_name_token() || p.at(SyntaxKind::LT) {
        p.parse_identification();
    }

    p.skip_trivia();

    // Multiplicity before typing
    if p.at(SyntaxKind::L_BRACKET) {
        p.parse_multiplicity();
        p.skip_trivia();
    }

    // Typing
    if p.at(SyntaxKind::COLON) || p.at(SyntaxKind::TYPED_KW) {
        p.parse_typing();
    }

    p.skip_trivia();

    // Multiplicity after typing
    if p.at(SyntaxKind::L_BRACKET) {
        p.parse_multiplicity();
        p.skip_trivia();
    }

    // Specializations
    parse_specializations(p);
    p.skip_trivia();

    // Default value
    if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) {
        p.bump();
        p.skip_trivia();
        parse_expression(p);
    }

    p.skip_trivia();
    p.parse_body();

    p.finish_node();
}

/// Parse a return expression statement: return <expression>;
/// This is different from return parameter declaration (return x : Type;)
/// Pattern: return <expression> ;
pub(super) fn parse_return_expression<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    p.expect(SyntaxKind::RETURN_KW);
    p.skip_trivia();

    // Parse the expression
    parse_expression(p);
    p.skip_trivia();

    // Expect semicolon
    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
    }

    p.finish_node();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sysml_definition_keywords() {
        assert!(is_sysml_definition_keyword(SyntaxKind::PART_KW));
        assert!(is_sysml_definition_keyword(SyntaxKind::ACTION_KW));
        assert!(is_sysml_definition_keyword(SyntaxKind::REQUIREMENT_KW));
        assert!(!is_sysml_definition_keyword(SyntaxKind::CLASS_KW)); // KerML
    }

    #[test]
    fn test_sysml_usage_keywords() {
        assert!(is_sysml_usage_keyword(SyntaxKind::PART_KW));
        assert!(is_sysml_usage_keyword(SyntaxKind::SEND_KW));
        assert!(is_sysml_usage_keyword(SyntaxKind::PERFORM_KW));
    }
}
