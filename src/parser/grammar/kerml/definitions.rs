use super::*;

// =============================================================================
// Definitions — types, classes, packages, imports, aliases
// =============================================================================

/// Parse typing clause (:, typed by, of)
/// Per pest: type_featuring = { typed_by | of_token }
pub fn parse_typing<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::TYPING);

    // Accept ':' or 'typed by' or 'of'
    if p.at(SyntaxKind::TYPED_KW) {
        bump_and_skip(p);
        p.expect(SyntaxKind::BY_KW);
    } else if p.at(SyntaxKind::OF_KW) {
        p.bump();
    } else {
        p.expect(SyntaxKind::COLON);
    }
    p.skip_trivia();

    consume_if(p, SyntaxKind::TILDE);

    parse_type_with_modifiers(p);

    // Comma-separated types
    while p.at(SyntaxKind::COMMA) {
        bump_and_skip(p);
        parse_type_with_modifiers(p);
    }

    p.finish_node();
}

/// Parse single type with optional multiplicity and ordering modifiers
fn parse_type_with_modifiers<P: KerMLParser>(p: &mut P) {
    parse_qualified_name_and_skip(p);
    parse_optional_multiplicity(p);

    while p.at(SyntaxKind::ORDERED_KW) || p.at(SyntaxKind::NONUNIQUE_KW) {
        bump_and_skip(p);
    }
}

/// Parse a multiplicity bound (lower or upper)
/// Per spec: multiplicity_bound = { inline_expression | number | "*" }
/// Bounds are typed as Expression in the metamodel
fn parse_multiplicity_bound<P: KerMLParser>(p: &mut P) {
    if p.at(SyntaxKind::STAR) {
        p.bump();
    } else if p.at(SyntaxKind::INTEGER) {
        // Integers are literals - parse as expression for consistency
        super::kerml_expressions::parse_expression(p);
    } else if p.at_name_token() || p.at(SyntaxKind::L_PAREN) {
        // Parse as full expression (handles identifiers, function calls, etc.)
        super::kerml_expressions::parse_expression(p);
    }
}

/// Parse multiplicity modifiers (ordered, nonunique)
fn parse_multiplicity_modifiers<P: KerMLParser>(p: &mut P) {
    while p.at(SyntaxKind::ORDERED_KW) || p.at(SyntaxKind::NONUNIQUE_KW) {
        bump_and_skip(p);
    }
}

/// Multiplicity = '[' bounds ']'
/// Per pest: multiplicity_bounds = { "[" ~ multiplicity_bounds_range ~ "]" }
/// Per pest: multiplicity_bounds_range = { multiplicity_bound ~ (".." ~ multiplicity_bound)? }
/// Per pest: multiplicity_bound = { inline_expression | number | "*" }
/// Per pest: ordering_modifiers = { (ordered_token | nonunique_token)* }
pub fn parse_multiplicity<P: KerMLParser>(p: &mut P) {
    if !p.at(SyntaxKind::L_BRACKET) {
        return;
    }

    p.start_node(SyntaxKind::MULTIPLICITY);
    bump_and_skip(p);

    if !p.at(SyntaxKind::R_BRACKET) {
        let is_modifier = p.at(SyntaxKind::ORDERED_KW) || p.at(SyntaxKind::NONUNIQUE_KW);

        if !is_modifier {
            parse_multiplicity_bound(p);
            p.skip_trivia();

            if p.at(SyntaxKind::DOT_DOT) {
                bump_and_skip(p);
                parse_multiplicity_bound(p);
            }
            p.skip_trivia();
        }

        parse_multiplicity_modifiers(p);
    }

    p.skip_trivia();
    p.expect(SyntaxKind::R_BRACKET);
    p.finish_node();
}

/// Multiplicity definition: multiplicity exactlyOne [1..1] { }
/// Per pest: multiplicity = { multiplicity_token ~ identification? ~ multiplicity_bounds? ~ namespace_body }
pub fn parse_multiplicity_definition<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    expect_and_skip(p, SyntaxKind::MULTIPLICITY_KW);

    // Optional identification
    parse_optional_identification(p);

    // Optional multiplicity bounds [1..1]
    parse_optional_multiplicity(p);

    // Body
    p.parse_body();

    p.finish_node();
}

/// Parse a single specialization relationship
/// Per pest: relationship = { visibility_kind? ~ element_reference ~ ...}
/// Per pest: inheritance = { relationship }
/// So many relationship clauses like :>, conjugates, chains, disjoint, etc. accept optional visibility
fn parse_single_specialization<P: KerMLParser>(p: &mut P, keyword: SyntaxKind) {
    p.start_node(SyntaxKind::SPECIALIZATION);
    bump_and_skip(p);

    if (keyword == SyntaxKind::DISJOINT_KW && p.at(SyntaxKind::FROM_KW))
        || (keyword == SyntaxKind::INVERSE_KW && p.at(SyntaxKind::OF_KW))
    {
        bump_and_skip(p);
    }

    // Parse optional visibility before the qualified name
    // Per pest: relationship = { visibility_kind? ~ element_reference }
    parse_optional_visibility(p);

    parse_qualified_name_and_skip(p);

    p.finish_node();
    p.skip_trivia();

    // Handle comma-separated references: specializes A, B, C
    // Each comma-separated item becomes a SEPARATE SPECIALIZATION node
    // (without the keyword, since the keyword only applies to the first)
    while p.at(SyntaxKind::COMMA) {
        p.start_node(SyntaxKind::SPECIALIZATION);
        bump_and_skip(p);
        // Each item in the list can have its own visibility
        parse_optional_visibility(p);
        parse_qualified_name_and_skip(p);
        p.finish_node();
        p.skip_trivia();
    }
}

/// Specializations = (':>' | 'specializes' | etc.) QualifiedName
/// Per pest: heritage = { specialization | reference_subsetting | subsetting | redefinition | cross_subsetting | conjugation }
/// Per pest: specializes_operator = { ":>" | specializes_token }
/// Per pest: redefines_operator = { ":>>" | redefines_token }
/// Per pest: subsets_operator = { ":>" | subsets_token }
pub fn parse_specializations<P: KerMLParser>(p: &mut P) {
    while p.at_any(&[
        SyntaxKind::COLON,
        SyntaxKind::TYPED_KW,
        SyntaxKind::OF_KW,
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
        if p.at(SyntaxKind::COLON) || p.at(SyntaxKind::TYPED_KW) || p.at(SyntaxKind::OF_KW) {
            parse_typing(p);
            p.skip_trivia();
            continue;
        }

        let keyword = p.current_kind();
        parse_single_specialization(p, keyword);
    }
}

/// Package = 'package' | 'namespace' Identification? Body
/// Per pest: package = { prefix_metadata? ~ (package_token | namespace_token) ~ identification? ~ namespace_body }
pub fn parse_package<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::PACKAGE);

    if p.at(SyntaxKind::PACKAGE_KW) || p.at(SyntaxKind::NAMESPACE_KW) {
        p.bump();
    } else {
        p.expect(SyntaxKind::PACKAGE_KW);
    }
    p.skip_trivia();

    parse_optional_identification(p);

    p.skip_trivia();
    p.parse_body();
    p.finish_node();
}

/// LibraryPackage = 'standard'? 'library' 'package' ...
/// Per pest: library_package = { prefix_metadata? ~ (library_token | standard_token) ~ library_token? ~ identification? ~ namespace_body }
pub fn parse_library_package<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::LIBRARY_PACKAGE);

    consume_if(p, SyntaxKind::STANDARD_KW);
    expect_and_skip(p, SyntaxKind::LIBRARY_KW);
    expect_and_skip(p, SyntaxKind::PACKAGE_KW);

    parse_optional_identification(p);

    p.skip_trivia();
    p.parse_body();
    p.finish_node();
}

/// Import = 'import' 'all'? ImportedMembership ... relationship_body
/// Per pest: import = { import_prefix ~ imported_reference ~ filter_package? ~ relationship_body }
/// Per pest: relationship_body = { ";" | ("{" ~ relationship_owned_elements ~ "}") }
pub fn parse_import<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::IMPORT);

    expect_and_skip(p, SyntaxKind::IMPORT_KW);
    consume_if(p, SyntaxKind::ALL_KW);
    parse_qualified_name_and_skip(p);

    parse_import_wildcards(p);

    p.skip_trivia();
    if p.at(SyntaxKind::L_BRACKET) {
        parse_filter_package(p);
    }

    // Per pest: relationship_body = ";" | ("{" ~ relationship_owned_elements ~ "}")
    p.skip_trivia();
    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
    } else if p.at(SyntaxKind::L_BRACE) {
        p.parse_body();
    } else {
        error_missing_body_terminator(p, "import statement");
    }
    p.finish_node();
}

/// Parse import wildcards: ::* or ::** or ::*::**
fn parse_import_wildcards<P: KerMLParser>(p: &mut P) {
    while p.at(SyntaxKind::COLON_COLON) {
        bump_and_skip(p);
        if p.at(SyntaxKind::STAR_STAR) {
            bump_and_skip(p);
        } else if p.at(SyntaxKind::STAR) {
            bump_and_skip(p);
            consume_if(p, SyntaxKind::STAR);
        } else {
            break;
        }
    }
}

fn parse_filter_package<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::FILTER_PACKAGE);

    // Parse one or more [expression] filter members
    while p.at(SyntaxKind::L_BRACKET) {
        bump_and_skip(p); // [

        // Check if it's metadata annotation syntax [@Type] or just filter expression
        if p.at(SyntaxKind::AT) || p.at(SyntaxKind::AT_AT) {
            bump_and_skip(p); // @ or @@
            parse_qualified_name_and_skip(p);
        } else {
            // Parse filter expression
            super::kerml_expressions::parse_expression(p);
        }

        p.skip_trivia();
        expect_and_skip(p, SyntaxKind::R_BRACKET);
    }

    p.finish_node(); // FILTER_PACKAGE
}

/// Alias = 'alias' Identification 'for' QualifiedName ';'
pub fn parse_alias<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::ALIAS_MEMBER);

    expect_and_skip(p, SyntaxKind::ALIAS_KW);
    parse_identification_and_skip(p);
    expect_and_skip(p, SyntaxKind::FOR_KW);
    parse_qualified_name_and_skip(p);

    // Per pest: relationship_body = ";" | ("{" ~ relationship_owned_elements ~ "}")
    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
    } else if p.at(SyntaxKind::L_BRACE) {
        p.parse_body();
    } else {
        error_missing_body_terminator(p, "alias declaration");
    }

    p.finish_node();
}

/// KerML definition (class, struct, etc.)
/// Per pest: class = { prefix_metadata? ~ visibility_kind? ~ abstract_marker? ~ class_token ~ all_token? ~ identification? ~ multiplicity_bounds? ~ classifier_relationships ~ namespace_body }
/// Per pest: structure = { prefix_metadata? ~ visibility_kind? ~ abstract_marker? ~ struct_token ~ identification? ~ all_token? ~ multiplicity_bounds? ~ classifier_relationships ~ namespace_body }
/// Per pest: datatype = { prefix_metadata? ~ visibility_kind? ~ abstract_marker? ~ datatype_token ~ identification? ~ all_token? ~ classifier_relationships ~ multiplicity? ~ namespace_body }
/// Per pest: behavior = { prefix_metadata? ~ visibility_kind? ~ abstract_marker? ~ behavior_token ~ identification? ~ all_token? ~ classifier_relationships ~ multiplicity? ~ namespace_body }
/// Per pest: function = { prefix_metadata? ~ visibility_kind? ~ abstract_marker? ~ function_token ~ identification? ~ all_token? ~ classifier_relationships ~ multiplicity? ~ result_expression_membership? ~ namespace_body }
pub fn parse_definition_impl<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::DEFINITION);

    // Prefixes
    while p.at(SyntaxKind::ABSTRACT_KW) || p.at(SyntaxKind::VARIATION_KW) {
        bump_and_skip(p);
    }

    let is_predicate = p.at(SyntaxKind::PREDICATE_KW);
    let is_function = p.at(SyntaxKind::FUNCTION_KW);

    // KerML keyword
    if p.at_any(&[
        SyntaxKind::CLASS_KW,
        SyntaxKind::STRUCT_KW,
        SyntaxKind::DATATYPE_KW,
        SyntaxKind::BEHAVIOR_KW,
        SyntaxKind::FUNCTION_KW,
        SyntaxKind::CLASSIFIER_KW,
        SyntaxKind::INTERACTION_KW,
        SyntaxKind::PREDICATE_KW,
        SyntaxKind::METACLASS_KW,
        SyntaxKind::TYPE_KW,
    ]) {
        p.bump();
    } else if p.at(SyntaxKind::ASSOC_KW) {
        bump_and_skip(p);
        consume_if(p, SyntaxKind::STRUCT_KW);
    }
    p.skip_trivia();

    consume_if(p, SyntaxKind::ALL_KW);

    parse_optional_identification(p);

    parse_optional_multiplicity(p);

    parse_specializations(p);
    p.skip_trivia();

    // Parse ordering modifiers (ordered, nonunique)
    while p.at(SyntaxKind::ORDERED_KW) || p.at(SyntaxKind::NONUNIQUE_KW) {
        bump_and_skip(p);
    }

    parse_optional_multiplicity(p);

    parse_specializations(p);
    p.skip_trivia();

    // Parse ordering modifiers again (can appear after relationships)
    while p.at(SyntaxKind::ORDERED_KW) || p.at(SyntaxKind::NONUNIQUE_KW) {
        bump_and_skip(p);
    }

    if is_predicate || is_function {
        parse_calc_body(p);
    } else {
        p.parse_body();
    }

    p.finish_node();
}

/// Parse a single element in a calc body (parameter, namespace element, or expression)
fn parse_calc_body_element<P: KerMLParser>(p: &mut P) -> bool {
    if p.at_any(&[
        SyntaxKind::IN_KW,
        SyntaxKind::OUT_KW,
        SyntaxKind::INOUT_KW,
        SyntaxKind::RETURN_KW,
    ]) {
        parse_parameter_impl(p);
        true
    } else if p.at_any(&[
        SyntaxKind::ATTRIBUTE_KW,
        SyntaxKind::PART_KW,
        SyntaxKind::ITEM_KW,
        SyntaxKind::CALC_KW,
        SyntaxKind::CONSTRAINT_KW,
        SyntaxKind::DOC_KW,
        SyntaxKind::COMMENT_KW,
        SyntaxKind::PRIVATE_KW,
        SyntaxKind::PUBLIC_KW,
        SyntaxKind::PROTECTED_KW,
        SyntaxKind::FEATURE_KW,
        SyntaxKind::STEP_KW,
        SyntaxKind::EXPR_KW,
        SyntaxKind::FUNCTION_KW,
    ]) {
        parse_namespace_element(p);
        true
    } else if p.at_name_token()
        || p.at(SyntaxKind::L_PAREN)
        || p.at(SyntaxKind::INTEGER)
        || p.at(SyntaxKind::STRING)
    {
        super::kerml_expressions::parse_expression(p);
        p.skip_trivia();
        if p.at(SyntaxKind::SEMICOLON) {
            p.bump();
        }
        true
    } else {
        parse_namespace_element(p);
        true
    }
}

/// Per pest: Used for function/predicate result expression body
/// Similar to namespace_body but specialized for calculation results
pub fn parse_calc_body<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::NAMESPACE_BODY);

    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
    } else if p.at(SyntaxKind::L_BRACE) {
        bump_and_skip(p);

        while !p.at(SyntaxKind::R_BRACE) && !p.at(SyntaxKind::ERROR) {
            let start_pos = p.get_pos();

            parse_calc_body_element(p);
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
        error_missing_body_terminator(p, "calc/function definition");
    }

    p.finish_node();
}
