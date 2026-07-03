use super::*;

// =============================================================================
// Usages — features, steps, expressions, parameters, end features
// =============================================================================

// tag::parse_feature_prefix_modifiers[]
/// Parse feature prefix modifiers (var, composite, const, etc.)
/// Grammar: see docs/grammar-mapping.adoc#parse_feature_prefix_modifiers
pub fn parse_feature_prefix_modifiers<P: KerMLParser>(p: &mut P) {
    while p.at_any(&[
        SyntaxKind::VAR_KW,
        SyntaxKind::COMPOSITE_KW,
        SyntaxKind::PORTION_KW,
        SyntaxKind::MEMBER_KW,
        SyntaxKind::ABSTRACT_KW,
        SyntaxKind::DERIVED_KW,
        SyntaxKind::CONST_KW,
        SyntaxKind::END_KW,
        SyntaxKind::VARIATION_KW,
        SyntaxKind::READONLY_KW,
        SyntaxKind::REF_KW,
    ]) {
        p.bump();
        p.skip_trivia();
    }

    while p.at(SyntaxKind::HASH) {
        parse_prefix_metadata(p);
        p.skip_trivia();
    }
}
// end::parse_feature_prefix_modifiers[]

// tag::parse_optional_feature_keyword[]
/// Parse optional feature keyword (feature, step, expr, inv)
/// Grammar: see docs/grammar-mapping.adoc#parse_optional_feature_keyword
fn parse_optional_feature_keyword<P: KerMLParser>(p: &mut P) -> bool {
    if p.at(SyntaxKind::INV_KW) {
        p.bump();
        p.skip_trivia();
        // Optional 'not' after 'inv' (negated invariant)
        if p.at(SyntaxKind::NOT_KW) {
            p.bump();
            p.skip_trivia();
        }
        true
    } else if p.at_any(&[
        SyntaxKind::FEATURE_KW,
        SyntaxKind::STEP_KW,
        SyntaxKind::EXPR_KW,
    ]) {
        p.bump();
        true
    } else if p.at(SyntaxKind::IDENT) {
        if let Some(text) = p.current_token_text() {
            if text == "feature" || text == "step" || text == "expr" {
                p.bump();
                true
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    }
}
// end::parse_optional_feature_keyword[]

/// Parse usage identification or specialization shortcuts
fn parse_usage_name_or_shorthand<P: KerMLParser>(p: &mut P) {
    if p.at(SyntaxKind::REDEFINES_KW)
        || p.at(SyntaxKind::COLON_GT_GT)
        || p.at(SyntaxKind::SUBSETS_KW)
        || p.at(SyntaxKind::COLON_GT)
    {
        // Wrap in SPECIALIZATION node so AST can extract the relationship
        p.start_node(SyntaxKind::SPECIALIZATION);
        bump_and_skip(p);
        parse_optional_qualified_name(p);
        p.finish_node();
        while p.at(SyntaxKind::COMMA) {
            bump_and_skip(p);
            p.start_node(SyntaxKind::SPECIALIZATION);
            parse_qualified_name_and_skip(p);
            p.finish_node();
        }
    } else if p.at_name_token() || p.at(SyntaxKind::LT) {
        // Check for "type name" pattern: two identifiers in a row
        // e.g., "bool signalCondition { }" = feature signalCondition : bool
        if p.at(SyntaxKind::IDENT) {
            let peek1 = p.peek_kind(1);
            if peek1 == SyntaxKind::IDENT || peek1 == SyntaxKind::LT {
                // First identifier is the type, create typing node
                p.start_node(SyntaxKind::TYPING);
                p.bump(); // type name
                p.skip_trivia();
                p.finish_node();
                // Second identifier is the feature name
                p.parse_identification();
                return;
            }
        }
        p.parse_identification();
    }
}

/// Parse usage details (multiplicity, typing, specializations, relationships)
fn parse_usage_details<P: KerMLParser>(p: &mut P) {
    p.skip_trivia();
    parse_optional_multiplicity(p);
    parse_optional_typing(p);
    parse_optional_multiplicity(p);
    // Ordering modifiers can appear before or after specializations
    parse_ordering_modifiers(p);
    parse_specializations(p);
    p.skip_trivia();
    // Per SysML v2, multiplicity can also appear after specializations
    // e.g., `feature x redefines y [*] nonunique;`
    parse_optional_multiplicity(p);
    // Parse ordering modifiers again (can appear after specializations too)
    parse_ordering_modifiers(p);
    parse_feature_relationships(p);
    p.skip_trivia();
}

/// Parse ordering modifiers (ordered, nonunique)
fn parse_ordering_modifiers<P: KerMLParser>(p: &mut P) {
    while p.at(SyntaxKind::ORDERED_KW) || p.at(SyntaxKind::NONUNIQUE_KW) {
        bump_and_skip(p);
    }
}

// tag::parse_optional_default_value[]
/// Parse optional default value (= or := or default)
/// Grammar: see docs/grammar-mapping.adoc#parse_optional_default_value
fn parse_optional_default_value<P: KerMLParser>(p: &mut P) {
    if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) || p.at(SyntaxKind::DEFAULT_KW) {
        bump_and_skip(p);
        super::kerml_expressions::parse_expression(p);
        p.skip_trivia();
    }
}
// end::parse_optional_default_value[]

// tag::parse_usage_impl[]
/// KerML usage (feature, step, expr)
/// Grammar: see docs/grammar-mapping.adoc#parse_usage_impl
pub fn parse_usage_impl<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    parse_feature_prefix_modifiers(p);

    let _consumed_feature_keyword = parse_optional_feature_keyword(p);
    p.skip_trivia();

    // Handle 'all' keyword after feature keyword (universal quantification)
    // Per SysML v2 Spec §7.3.3.4: feature all x means x covers all instances
    if p.at(SyntaxKind::ALL_KW) {
        p.bump();
        p.skip_trivia();
    }

    parse_usage_name_or_shorthand(p);

    parse_usage_details(p);

    parse_optional_default_value(p);

    p.parse_body();
    p.finish_node();
}
// end::parse_usage_impl[]

// tag::parse_invariant[]
/// KerML invariant (inv [not]? name? { expression })
/// Grammar: see docs/grammar-mapping.adoc#parse_invariant
pub fn parse_invariant<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    p.expect(SyntaxKind::INV_KW);
    p.skip_trivia();

    // Optional 'not'
    if p.at(SyntaxKind::NOT_KW) {
        p.bump();
        p.skip_trivia();
    }

    // Optional identification
    if p.at_name_token() || p.at(SyntaxKind::LT) {
        p.parse_identification();
        p.skip_trivia();
    }

    // Body: { expression }
    if p.at(SyntaxKind::L_BRACE) {
        p.start_node(SyntaxKind::NAMESPACE_BODY);
        p.bump(); // {
        p.skip_trivia();

        // Handle doc comment before expression (common in stdlib)
        if p.at(SyntaxKind::DOC_KW) || p.at(SyntaxKind::COMMENT_KW) {
            parse_annotation(p);
            p.skip_trivia();
        }

        // Parse the invariant expression
        if !p.at(SyntaxKind::R_BRACE) {
            super::kerml_expressions::parse_expression(p);
        }

        p.skip_trivia();
        p.expect(SyntaxKind::R_BRACE);
        p.finish_node();
    } else {
        p.parse_body();
    }

    p.finish_node();
}
// end::parse_invariant[]

// tag::parse_parameter_impl[]
/// Parameters are features with explicit direction keywords
/// Grammar: see docs/grammar-mapping.adoc#parse_parameter_impl
pub fn parse_parameter_impl<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    // Parse parameter direction keyword
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

    parse_feature_prefix_modifiers(p);

    // Optional usage keyword
    if p.at_any(&[
        SyntaxKind::ATTRIBUTE_KW,
        SyntaxKind::PART_KW,
        SyntaxKind::ITEM_KW,
        SyntaxKind::CALC_KW,
        SyntaxKind::ACTION_KW,
        SyntaxKind::STATE_KW,
        SyntaxKind::PORT_KW,
        SyntaxKind::FEATURE_KW,
        SyntaxKind::STEP_KW,
        SyntaxKind::EXPR_KW,
    ]) {
        bump_and_skip(p);
    }

    // Parameters can have: type_name name | name | ...
    // Check for two identifiers in a row (type + name pattern)
    if p.at(SyntaxKind::IDENT) {
        let peek1 = p.peek_kind(1);
        if peek1 == SyntaxKind::IDENT {
            // First identifier is the type, bump it
            p.bump();
            p.skip_trivia();
            // Second identifier is the name
            if p.at_name_token() {
                p.parse_identification();
            }
        } else {
            // Just a name (or starts with shorthand)
            parse_usage_name_or_shorthand(p);
        }
    } else {
        parse_usage_name_or_shorthand(p);
    }

    parse_usage_details(p);

    parse_optional_default_value(p);

    p.parse_body();
    p.finish_node();
}
// end::parse_parameter_impl[]

/// Parse multiplicity with ordering modifiers
fn parse_multiplicity_with_ordering<P: KerMLParser>(p: &mut P) {
    if p.at(SyntaxKind::L_BRACKET) {
        parse_multiplicity(p);
        p.skip_trivia();

        while p.at(SyntaxKind::ORDERED_KW) || p.at(SyntaxKind::NONUNIQUE_KW) {
            bump_and_skip(p);
        }
    }
}

/// Parse all prefix modifiers (ref, readonly, derived, etc.)
fn parse_prefix_modifiers<P: KerMLParser>(p: &mut P) {
    while p.at_any(&[
        SyntaxKind::REF_KW,
        SyntaxKind::READONLY_KW,
        SyntaxKind::DERIVED_KW,
        SyntaxKind::ABSTRACT_KW,
        SyntaxKind::VARIATION_KW,
        SyntaxKind::VAR_KW,
        SyntaxKind::COMPOSITE_KW,
        SyntaxKind::PORTION_KW,
        SyntaxKind::INDIVIDUAL_KW,
    ]) {
        bump_and_skip(p);
    }
}

/// Parse optional SysML usage keyword (item, part, action, etc.)
fn parse_optional_sysml_usage_keyword<P: KerMLParser>(p: &mut P) {
    if p.at_any(&[
        SyntaxKind::ITEM_KW,
        SyntaxKind::PART_KW,
        SyntaxKind::ACTION_KW,
        SyntaxKind::STATE_KW,
        SyntaxKind::ATTRIBUTE_KW,
        SyntaxKind::PORT_KW,
    ]) {
        bump_and_skip(p);
    }
}

/// Parse end feature prefix (metadata and modifiers)
fn parse_end_feature_prefix<P: KerMLParser>(p: &mut P) {
    while p.at(SyntaxKind::HASH) {
        parse_prefix_metadata(p);
        p.skip_trivia();
    }

    parse_prefix_modifiers(p);
    parse_multiplicity_with_ordering(p);
    parse_optional_sysml_usage_keyword(p);
}

/// Parse feature details (identification, typing, specializations)
fn parse_feature_details<P: KerMLParser>(p: &mut P, parse_id: bool) {
    if parse_id {
        parse_optional_identification(p);
    }

    parse_multiplicity_with_ordering(p);

    parse_optional_typing(p);

    parse_specializations(p);
    p.skip_trivia();

    parse_feature_relationships(p);
    p.skip_trivia();
}

/// Parse end feature when FEATURE_KW is present
fn parse_end_feature_with_keyword<P: KerMLParser>(p: &mut P) {
    bump_and_skip(p);

    // Check for specialization-first pattern or identification
    if p.at(SyntaxKind::REDEFINES_KW)
        || p.at(SyntaxKind::COLON_GT_GT)
        || p.at(SyntaxKind::SUBSETS_KW)
        || p.at(SyntaxKind::COLON_GT)
        || p.at(SyntaxKind::REFERENCES_KW)
        || p.at(SyntaxKind::COLON_COLON_GT)
    {
        parse_feature_details(p, false);
    } else {
        parse_feature_details(p, true);
    }
}

/// Parse typing and relationships without FEATURE keyword
fn parse_typing_and_relationships<P: KerMLParser>(p: &mut P) {
    parse_optional_typing(p);
    parse_specializations(p);
    p.skip_trivia();
    parse_feature_relationships(p);
    p.skip_trivia();
}

/// Parse end feature when starting with name/identification
fn parse_end_feature_with_name<P: KerMLParser>(p: &mut P) {
    parse_identification_and_skip(p);
    parse_multiplicity_with_ordering(p);
    parse_specializations(p);
    p.skip_trivia();
    parse_feature_relationships(p);
    p.skip_trivia();

    if p.at(SyntaxKind::FEATURE_KW) {
        bump_and_skip(p);
        parse_feature_details(p, true);
    } else {
        parse_typing_and_relationships(p);
    }
}

/// Parse minimal end feature (no name, no FEATURE_KW initially)
fn parse_end_feature_minimal<P: KerMLParser>(p: &mut P) {
    parse_multiplicity_with_ordering(p);
    parse_specializations(p);
    p.skip_trivia();

    if p.at(SyntaxKind::FEATURE_KW) {
        bump_and_skip(p);
        parse_feature_details(p, true);
    }
}

// tag::parse_end_feature_or_parameter[]
/// End feature or parameter
/// Grammar: see docs/grammar-mapping.adoc#parse_end_feature_or_parameter
pub fn parse_end_feature_or_parameter<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    consume_if(p, SyntaxKind::CONST_KW);
    expect_and_skip(p, SyntaxKind::END_KW);

    parse_end_feature_prefix(p);

    if p.at(SyntaxKind::FEATURE_KW) {
        parse_end_feature_with_keyword(p);
    } else if p.at_name_token() || p.at(SyntaxKind::LT) {
        // Check for "type name" shorthand pattern: `end bool constrainedGuard;`
        // Two identifiers in a row means first is type, second is name
        if p.at(SyntaxKind::IDENT) {
            let peek1 = p.peek_kind(1);
            if peek1 == SyntaxKind::IDENT || peek1 == SyntaxKind::LT {
                // First identifier is the type, create typing node
                p.start_node(SyntaxKind::TYPING);
                p.bump(); // type name
                p.skip_trivia();
                p.finish_node();
                // Second identifier is the feature name
                p.parse_identification();
                // Continue with end feature details
                parse_multiplicity_with_ordering(p);
                parse_specializations(p);
                p.skip_trivia();
                parse_feature_relationships(p);
                p.skip_trivia();
            } else {
                parse_end_feature_with_name(p);
            }
        } else {
            parse_end_feature_with_name(p);
        }
    } else {
        parse_end_feature_minimal(p);
    }

    parse_optional_default_value(p);

    p.parse_body();
    p.finish_node();
}
// end::parse_end_feature_or_parameter[]
