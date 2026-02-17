use super::*;

// =============================================================================
// Usages — features, steps, expressions, parameters, end features
// =============================================================================

/// Parse feature prefix modifiers (var, composite, const, etc.)
/// Per pest: feature_prefix_modifiers = { (abstract_token | composite_token | portion_token | member_token | const_modifier | derived | end_marker | variable_marker)* }
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

/// Parse optional feature keyword (feature, step, expr, inv)
/// Per pest: invariant = { prefix_metadata? ~ inv_token ~ not_token? ~ identification? ~ ... }
fn parse_optional_feature_keyword<P: KerMLParser>(p: &mut P) -> bool {
    if p.at(SyntaxKind::INV_KW) {
        p.bump();
        p.skip_trivia();
        // Per pest: inv_token ~ not_token? - handle optional 'not' after 'inv'
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
    // Per pest: ordering_modifiers can appear before or after specializations
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

/// Parse optional default value (= or := or default)
/// Per pest: feature_value = { ("=" | ":=" | default_token) ~ owning_membership }
fn parse_optional_default_value<P: KerMLParser>(p: &mut P) {
    if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) || p.at(SyntaxKind::DEFAULT_KW) {
        bump_and_skip(p);
        super::kerml_expressions::parse_expression(p);
        p.skip_trivia();
    }
}

/// KerML usage (feature, step, expr)
/// Per pest: feature = { prefix_metadata? ~ visibility_kind? ~ feature_direction_kind? ~ feature_prefix_modifiers ~ feature_token ~ all_token? ~ identification? ~ feature_specialization_part? ~ ordering_modifiers ~ feature_relationship_part* ~ feature_value? ~ namespace_body }
/// Per pest: step = { prefix_metadata? ~ feature_direction_kind? ~ connector_feature_modifiers ~ step_token ~ identification? ~ feature_specialization_part? ~ feature_value? ~ membership? ~ owning_membership? ~ namespace_body }
/// Per pest: expression = similar to feature with expr_token
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

/// KerML invariant (inv [not]? name? { expression })
/// Per pest: invariant = { prefix_metadata? ~ inv_token ~ not_token? ~ identification? ~ invariant_body }
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

/// KerML parameter (in, out, inout, return)
/// Per pest: feature_direction_kind = { inout_token | in_token | out_token }
/// Per pest: parameter_membership = { direction ~ (type_name ~ name | name | ...) ~ ... }
/// Parameters are features with explicit direction keywords
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

    // Per pest grammar, parameters can have: type_name name | name | ...
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

/// End feature or parameter
/// Per pest: end_feature = { prefix_metadata? ~ const_token? ~ end_marker ~ (...various patterns...) ~ feature_value? ~ namespace_body }
/// Per pest: EndFeaturePrefix = ( isConstant ?= 'const')? isEnd ?= 'end'
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
