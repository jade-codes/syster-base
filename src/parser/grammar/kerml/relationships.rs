use super::*;

// =============================================================================
// Standalone Relationship Parsing
// =============================================================================

/// Parse featuring relationship: featuring [id? of]? feature by type
/// Per pest: type_featuring = { featuring_token ~ (identification ~ of_token)? ~ qualified_reference_chain ~ by_token ~ qualified_reference_chain }
fn parse_featuring_relationship<P: KerMLParser>(p: &mut P) {
    bump_and_skip(p);

    // Check for optional identification + 'of'
    if p.at_name_token() {
        parse_identification_and_skip(p);

        if p.at(SyntaxKind::OF_KW) {
            bump_and_skip(p);
            parse_optional_qualified_name(p);
        }
    }

    // Parse 'by' clause
    if p.at(SyntaxKind::BY_KW) {
        bump_and_skip(p);
        parse_optional_qualified_name(p);
    }
}

/// Parse typing relationship: typing feature (':' | 'typed by') type
/// Per pest: standalone_feature_typing = { typing_token ~ qualified_reference_chain ~ feature_typing }
/// Per pest: feature_typing = { typed_by_operator ~ qualified_reference_chain ~ multiplicity_bounds? ~ ordering_modifiers }
fn parse_typing_relationship<P: KerMLParser>(p: &mut P) {
    bump_and_skip(p);

    parse_optional_qualified_name(p);

    if p.at(SyntaxKind::COLON) || p.at(SyntaxKind::TYPED_KW) {
        parse_typing(p);
    }
}

/// Parse conjugation relationship: conjugate type1 ('~' | 'conjugates') type2
/// Per pest: standalone_conjugation = { conjugation_token ~ identification? ~ conjugate_token? ~ qualified_reference_chain ~ conjugates_operator ~ qualified_reference_chain ~ relationship_body }
/// Per pest: conjugates_operator = { "~" | conjugates_token }
/// Also handles shorthand: conjugate A ~ B;
fn parse_conjugation_relationship<P: KerMLParser>(p: &mut P) {
    // Check if we start with 'conjugation' (full form) or 'conjugate' (shorthand)
    let is_shorthand = p.at(SyntaxKind::CONJUGATE_KW);
    bump_and_skip(p);

    if is_shorthand {
        // Shorthand form: conjugate A ~ B;
        // Parse source type directly
        parse_optional_qualified_name(p);

        if p.at(SyntaxKind::CONJUGATES_KW) || p.at(SyntaxKind::TILDE) {
            bump_and_skip(p);
        }

        parse_optional_qualified_name(p);
    } else {
        // Full form: conjugation [id] conjugate A ~ B;
        // Optional identification
        if p.at_name_token() && !p.at(SyntaxKind::CONJUGATE_KW) {
            parse_identification_and_skip(p);
        }

        consume_if(p, SyntaxKind::CONJUGATE_KW);

        parse_optional_qualified_name(p);

        if p.at(SyntaxKind::CONJUGATES_KW) || p.at(SyntaxKind::TILDE) {
            bump_and_skip(p);
        }

        parse_optional_qualified_name(p);
    }
}

/// Parse generic relationship: keyword source operator target
/// Handles relationships that don't fit other specific patterns
fn parse_generic_relationship<P: KerMLParser>(p: &mut P) {
    if p.at_any(STANDALONE_RELATIONSHIP_KEYWORDS) {
        bump_and_skip(p);
    }

    parse_optional_qualified_name(p);

    if p.at_any(RELATIONSHIP_OPERATORS) {
        bump_and_skip(p);
    }

    parse_optional_qualified_name(p);
}

/// Parse KerML standalone relationship declarations
/// E.g., `specialization Super subclassifier A specializes B;`
/// E.g., `subclassifier C specializes A;`
/// E.g., `redefinition MyRedef redefines x :>> y;`
/// Per Pest grammar: specialization_prefix ~ relationship_keyword ~ from ~ operator ~ to ~ relationship_body
/// Per pest: standalone_specialization | standalone_conjugation | standalone_feature_typing | subclassification | disjoining | feature_inverting | standalone_subsetting | standalone_redefinition | type_featuring
/// Per pest: specialization_prefix = { (specialization_token ~ identification?)? }
pub fn parse_standalone_relationship<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::RELATIONSHIP);

    // Handle optional 'specialization' prefix with optional identification
    if p.at(SyntaxKind::SPECIALIZATION_KW) {
        bump_and_skip(p);

        if p.at_name_token()
            && !p.at_any(&[
                SyntaxKind::SUBCLASSIFIER_KW,
                SyntaxKind::SUBTYPE_KW,
                SyntaxKind::SUBSET_KW,
                SyntaxKind::REDEFINITION_KW,
                SyntaxKind::TYPING_KW,
            ])
        {
            parse_identification_and_skip(p);
        }
    }

    // Dispatch to specific relationship handlers
    if p.at(SyntaxKind::FEATURING_KW) {
        parse_featuring_relationship(p);
    } else if p.at(SyntaxKind::TYPING_KW) {
        parse_typing_relationship(p);
    } else if p.at(SyntaxKind::CONJUGATION_KW) || p.at(SyntaxKind::CONJUGATE_KW) {
        parse_conjugation_relationship(p);
    } else {
        parse_generic_relationship(p);
    }

    p.parse_body();
    p.finish_node();
}

/// Parse dependency relationship
/// Syntax: dependency [identification from]? source (',' source)* to target (',' target)* body
/// Per pest: dependency = { dependency_token ~ (identification ~ from_token)? ~ qualified_reference_chain ~ ("," ~ qualified_reference_chain)* ~ to_token ~ qualified_reference_chain ~ ("," ~ qualified_reference_chain)* ~ relationship_body }
pub fn parse_dependency<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::DEPENDENCY);

    expect_and_skip(p, SyntaxKind::DEPENDENCY_KW);

    // Check for identification (can start with < for short name or an identifier)
    // Identification is followed by 'from', or just 'from' keyword
    if p.at(SyntaxKind::FROM_KW) {
        bump_and_skip(p);
    } else if p.at(SyntaxKind::LT) {
        // Short name like <short>
        parse_identification_and_skip(p);
        if p.at(SyntaxKind::FROM_KW) {
            bump_and_skip(p);
        }
    } else if p.at_name_token() && !p.at(SyntaxKind::TO_KW) {
        // Check if this is an identification followed by 'from'
        // by looking for 'from' after the name(s)
        let peek1 = p.peek_kind(1);
        let peek2 = p.peek_kind(2);
        if peek1 == SyntaxKind::FROM_KW || peek2 == SyntaxKind::FROM_KW {
            parse_identification_and_skip(p);
            if p.at(SyntaxKind::FROM_KW) {
                bump_and_skip(p);
            }
        }
    }

    // Parse source(s)
    if p.at_name_token() && !p.at(SyntaxKind::TO_KW) {
        parse_comma_separated_names(p);
    }

    expect_and_skip(p, SyntaxKind::TO_KW);

    // Parse target(s)
    if p.at_name_token() {
        parse_comma_separated_names(p);
    }

    p.parse_body();
    p.finish_node();
}

/// Parse textual representation
/// Syntax: [rep id?]? language "string" [comment]? ;?
/// Per pest: textual_representation = { (rep_token ~ identification?)? ~ language_token ~ string_value ~ block_comment? ~ ";"? }
pub fn parse_textual_representation<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::TEXTUAL_REP);

    // Optional 'rep' with identification
    if p.at(SyntaxKind::REP_KW) {
        bump_and_skip(p);
        if p.at_name_token() || p.at(SyntaxKind::LT) {
            parse_identification_and_skip(p);
        }
    }

    // Required 'language' keyword
    expect_and_skip(p, SyntaxKind::LANGUAGE_KW);

    // Required string value
    expect_and_skip(p, SyntaxKind::STRING);

    // Optional block comment (already part of trivia, will be skipped)
    // Optional semicolon
    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
    }

    p.finish_node();
}

/// Parse disjoint statement
/// Syntax: disjoint source [from target] ;
/// Per pest: disjoining = { disjoint_token ~ (element_reference ~ from_token ~ element_reference | from_token ~ relationship | visibility_kind? ~ element_reference) }
/// Source and target can be qualified names (::) or feature chains (.)
pub fn parse_disjoint<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::RELATIONSHIP);

    expect_and_skip(p, SyntaxKind::DISJOINT_KW);

    // Parse source - can be a qualified name or feature chain (with .)
    if p.at_name_token() {
        parse_feature_chain_or_qualified_name(p);
    }

    // Optional 'from' keyword followed by target
    if p.at(SyntaxKind::FROM_KW) {
        bump_and_skip(p);

        // Parse target
        if p.at_name_token() {
            parse_feature_chain_or_qualified_name(p);
        }
    }

    // Parse body or semicolon
    p.parse_body();

    p.finish_node();
}

/// Parse a name that could be a qualified name (A::B::C) or feature chain (a.b.c)
pub fn parse_feature_chain_or_qualified_name<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::QUALIFIED_NAME);

    if p.at_name_token() {
        p.bump();
    }
    p.skip_trivia();

    // Handle both :: (qualified) and . (feature chain) separators
    while p.at(SyntaxKind::COLON_COLON) || p.at(SyntaxKind::DOT) {
        p.bump(); // :: or .
        p.skip_trivia();
        if p.at_name_token() {
            p.bump();
        }
        p.skip_trivia();
    }

    p.finish_node();
    p.skip_trivia();
}

/// Parse filter statement
/// Syntax: filter <expression> ;
/// Per pest: filter_package = { filter_token ~ inline_expression ~ ";" }
pub fn parse_filter<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::ELEMENT_FILTER_MEMBER);

    expect_and_skip(p, SyntaxKind::FILTER_KW);

    // Parse the filter expression
    kerml_expressions::parse_expression(p);
    p.skip_trivia();

    // Expect semicolon
    p.expect(SyntaxKind::SEMICOLON);

    p.finish_node();
}

/// Parse inverting/inverse relationship
/// Syntax: [inverting identification?] inverse source of target body
/// Per pest: feature_inverting = { (inverting_token ~ identification?)? ~ inverse_token ~ qualified_reference_chain ~ of_token ~ qualified_reference_chain ~ relationship_body }
pub fn parse_inverting_relationship<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::RELATIONSHIP);

    // Optional 'inverting' keyword with optional identification
    if p.at(SyntaxKind::INVERTING_KW) {
        bump_and_skip(p); // inverting

        // Optional identification after 'inverting'
        if p.at_name_token() && !p.at(SyntaxKind::INVERSE_KW) {
            parse_identification_and_skip(p);
        }
    }

    // Expect 'inverse' keyword
    expect_and_skip(p, SyntaxKind::INVERSE_KW);

    // Parse source (feature or chain)
    parse_optional_qualified_name(p);

    // Expect 'of' keyword
    expect_and_skip(p, SyntaxKind::OF_KW);

    // Parse target (feature or chain)
    parse_optional_qualified_name(p);

    // Parse body or semicolon
    p.parse_body();

    p.finish_node();
}
