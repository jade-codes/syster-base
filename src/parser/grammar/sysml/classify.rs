use super::*;

/// Per pest: package_member = { (definition | usage | alias_member | ...)
/// Pattern: Determines whether to parse as definition (has 'def') or usage (no 'def')
pub fn parse_definition_or_usage<P: SysMLParser>(p: &mut P) {
    let classification = classify_definition_or_usage(p);

    match classification {
        DefinitionClassification::SysmlDefinition => parse_definition(p),
        DefinitionClassification::KermlDefinition => parse_kerml_definition(p),
        DefinitionClassification::Usage => parse_usage(p),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DefinitionClassification {
    SysmlDefinition,
    KermlDefinition,
    Usage,
}

/// Check if kind is a KerML-only definition keyword (without 'def')
fn is_kerml_definition_keyword(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::CLASS_KW
            | SyntaxKind::STRUCT_KW
            | SyntaxKind::DATATYPE_KW
            | SyntaxKind::BEHAVIOR_KW
            | SyntaxKind::FUNCTION_KW
            | SyntaxKind::ASSOC_KW
            | SyntaxKind::INTERACTION_KW
            | SyntaxKind::PREDICATE_KW
            | SyntaxKind::METACLASS_KW
            | SyntaxKind::CLASSIFIER_KW
            | SyntaxKind::TYPE_KW
    )
}

fn classify_definition_or_usage<P: SysMLParser>(p: &P) -> DefinitionClassification {
    // Scan ahead (skipping trivia) to determine what we have
    // KerML definition: struct, class, datatype, etc. (no 'def' keyword)
    // SysML definition: has 'def' keyword
    // Usage: everything else (no 'def' keyword)

    // Check first token for KerML definition keywords
    // Skip over ABSTRACT_KW if present
    let first_non_prefix = if p.peek_kind(0) == SyntaxKind::ABSTRACT_KW {
        p.peek_kind(1)
    } else {
        p.peek_kind(0)
    };

    if is_kerml_definition_keyword(first_non_prefix) {
        return DefinitionClassification::KermlDefinition;
    }

    for i in 0..20 {
        // Look ahead up to 20 tokens
        let kind = p.peek_kind(i);

        // SysML definition: has 'def' keyword
        if kind == SyntaxKind::DEF_KW {
            return DefinitionClassification::SysmlDefinition;
        }

        // Stop scanning at statement-ending tokens
        if kind == SyntaxKind::SEMICOLON
            || kind == SyntaxKind::L_BRACE
            || kind == SyntaxKind::COLON
            || kind == SyntaxKind::COLON_GT
            || kind == SyntaxKind::COLON_GT_GT
            || kind == SyntaxKind::EQ
            || kind == SyntaxKind::ERROR
        {
            return DefinitionClassification::Usage;
        }
    }
    DefinitionClassification::Usage
}

fn parse_definition<P: SysMLParser>(p: &mut P) {
    // Per pest: definition = { prefix* ~ definition_declaration ~ definition_body }
    // Per pest: definition_declaration = { keyword ~ "def"? ~ (identifier ~ ";") | (usage_prefix ~ definition_declaration) }
    // Pattern: [abstract|variation|individual] <keyword> def <name> <specializations> <body>
    p.start_node(SyntaxKind::DEFINITION);

    // Prefixes (variation point and individual markers)
    while p.at(SyntaxKind::ABSTRACT_KW)
        || p.at(SyntaxKind::VARIATION_KW)
        || p.at(SyntaxKind::INDIVIDUAL_KW)
    {
        bump_keyword(p);
    }

    let is_constraint = p.at(SyntaxKind::CONSTRAINT_KW);
    let is_calc = p.at(SyntaxKind::CALC_KW);
    let is_action = p.at(SyntaxKind::ACTION_KW);
    let is_state = p.at(SyntaxKind::STATE_KW);
    let is_analysis = p.at(SyntaxKind::ANALYSIS_KW);
    let is_verification = p.at(SyntaxKind::VERIFICATION_KW);
    let is_metadata = p.at(SyntaxKind::METADATA_KW);
    let is_usecase = p.at(SyntaxKind::USE_KW); // use case def

    // Definition keyword
    parse_definition_keyword(p);
    p.skip_trivia();

    // 'def' keyword (or 'case def' for analysis/verification)
    consume_if(p, SyntaxKind::CASE_KW);
    expect_and_skip(p, SyntaxKind::DEF_KW);

    // Identification
    if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::LT) {
        p.parse_identification();
    }
    p.skip_trivia();

    // Specializations
    parse_specializations_with_skip(p);

    // Body
    if is_constraint {
        parse_constraint_body(p);
    } else if is_calc {
        parse_sysml_calc_body(p);
    } else if is_action {
        parse_action_body(p);
    } else if is_state {
        parse_state_body(p);
    } else if is_analysis || is_verification || is_usecase {
        parse_case_body(p);
    } else if is_metadata {
        parse_metadata_body(p);
    } else {
        p.parse_body();
    }

    p.finish_node();
}

/// Parse a KerML definition (class, struct, datatype, etc.)
/// These definitions don't use 'def' keyword like SysML definitions
/// Per pest: structure = { abstract? ~ struct_token ~ identification? ~ specializations ~ namespace_body }
fn parse_kerml_definition<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::DEFINITION);

    // Optional abstract prefix
    consume_if(p, SyntaxKind::ABSTRACT_KW);

    // KerML definition keyword (class, struct, datatype, etc.)
    if is_kerml_definition_keyword(p.current_kind()) {
        bump_keyword(p);
    }
    p.skip_trivia();

    // Identification (name)
    if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::LT) {
        p.parse_identification();
    }
    p.skip_trivia();

    // Specializations
    parse_specializations_with_skip(p);

    // Body
    p.parse_body();

    p.finish_node();
}

