use super::*;

// =============================================================================
// Requirement Body Elements
// =============================================================================

/// SubjectUsage = 'subject' Identification? Typing? ';'
/// Per pest: requirement_subject_usage = { requirement_subject_usage_declaration ~ (";"|requirement_body) }
/// Per pest: requirement_subject_usage_declaration = { subject_prefix? ~ usage_declaration }
/// Pattern: subject <name>? <typing>? <specializations>? <multiplicity>? <default>? <body|semicolon>
pub fn parse_subject_usage<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::SUBJECT_USAGE);

    p.expect(SyntaxKind::SUBJECT_KW);
    p.skip_trivia();

    parse_optional_identification(p);

    // Typing (optional, can come before or after specializations)
    parse_optional_typing(p);

    // Specializations (redefines, subsets, etc.)
    parse_specializations_with_skip(p);

    // Multiplicity (can come after specializations)
    parse_optional_multiplicity(p);

    // Default value (with 'default' keyword or '=' operator)
    parse_optional_default_value(p);

    p.parse_body();

    p.finish_node();
}

/// ActorUsage = 'actor' Identification? Typing? ';'
/// Per pest: requirement_actor_member = { requirement_actor_member_declaration ~ value_part? ~ multiplicity_part? ~ (";"|requirement_body) }
/// Per pest: requirement_actor_member_declaration = { "actor" ~ usage_declaration? }
/// Pattern: actor <name>? <typing>? <specializations>? <multiplicity>? <default>? semicolon
pub fn parse_actor_usage<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::ACTOR_USAGE);

    p.expect(SyntaxKind::ACTOR_KW);
    p.skip_trivia();

    parse_optional_identification(p);

    parse_optional_typing(p);

    // Specializations (redefines, subsets, etc.)
    parse_specializations_with_skip(p);

    // Multiplicity
    parse_optional_multiplicity(p);

    // Default value
    parse_optional_default_value(p);

    p.expect(SyntaxKind::SEMICOLON);

    p.finish_node();
}

/// StakeholderUsage = 'stakeholder' Identification? Typing? ';'
/// Per pest: requirement_stakeholder_member = { "stakeholder" ~ usage_declaration? ~ (";"|requirement_body) }
/// Pattern: stakeholder <name>? <typing>? semicolon
pub fn parse_stakeholder_usage<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::STAKEHOLDER_USAGE);

    p.expect(SyntaxKind::STAKEHOLDER_KW);
    p.skip_trivia();

    parse_optional_identification(p);

    parse_optional_typing(p);

    p.expect(SyntaxKind::SEMICOLON);

    p.finish_node();
}

/// ObjectiveUsage = 'objective' Identification? [: Type] [:>> ref, ...] Body
/// Per pest: requirement_objective_member = { "objective" ~ usage_declaration? ~ requirement_body }
/// Pattern: objective <name>? <typing>? <specializations>? <multiplicity>? body
pub fn parse_objective_usage<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::OBJECTIVE_USAGE);

    p.expect(SyntaxKind::OBJECTIVE_KW);
    p.skip_trivia();

    parse_optional_identification(p);

    parse_optional_typing(p);

    // Specializations (per pest: constraint_usage_declaration = usage_declaration? ~ value_part?)
    parse_specializations_with_skip(p);

    // Multiplicity
    parse_optional_multiplicity(p);

    p.parse_body();

    p.finish_node();
}

/// RequirementConstraint = ('assert' | 'assume' | 'require') 'constraint'? Identification? ConstraintBody
/// Per pest: requirement_constraint_member = { constraint_prefix? ~ metadata_prefix* ~ "constraint" ~ usage_declaration? ~ value_part? ~ constraint_body }
/// Per pest: constraint_prefix = { ("assert"|"assume"|"require") }
/// Pattern: assert|assume|require [#metadata] [constraint] <name>? <typing|specializations>? <body|semicolon>
pub fn parse_requirement_constraint<P: SysMLParser>(p: &mut P) {
    // Wrap in USAGE node so it gets extracted by NamespaceMember::cast
    p.start_node(SyntaxKind::USAGE);

    // Also wrap in REQUIREMENT_CONSTRAINT for semantic info
    p.start_node(SyntaxKind::REQUIREMENT_CONSTRAINT);

    // assert/assume/require
    p.bump();
    p.skip_trivia();

    // Prefix metadata (e.g., assume #goal constraint)
    while p.at(SyntaxKind::HASH) {
        parse_prefix_metadata(p);
        p.skip_trivia();
    }

    // Optional 'constraint' keyword - bump it to set usage kind
    let has_constraint_kw = p.at(SyntaxKind::CONSTRAINT_KW);
    if has_constraint_kw {
        p.bump();
        p.skip_trivia();
    }

    p.finish_node(); // REQUIREMENT_CONSTRAINT

    // Optional name or reference
    // When 'constraint' keyword present: parse as identification (defining new constraint)
    // When no 'constraint' keyword: parse as qualified name (referencing existing requirement)
    if p.at_name_token() || p.at(SyntaxKind::LT) {
        if has_constraint_kw {
            p.parse_identification();
        } else {
            // Reference to existing requirement (allow feature chains like X.Y)
            p.parse_qualified_name();
        }
        p.skip_trivia();
    }

    // Optional typing/specializations (e.g., "assume constraint c1 : C;" or "require constraint c1 :>> c;")
    if p.at(SyntaxKind::COLON) || p.at(SyntaxKind::COLON_GT) || p.at(SyntaxKind::COLON_GT_GT) {
        if p.at(SyntaxKind::COLON) {
            p.parse_typing();
            p.skip_trivia();
        } else {
            // Handle redefines/subsets as SPECIALIZATION
            p.start_node(SyntaxKind::SPECIALIZATION);
            p.bump(); // :> or :>>
            p.skip_trivia();
            if p.at_name_token() {
                p.parse_qualified_name();
            }
            p.finish_node();
            p.skip_trivia();
        }
    }

    // Body: can be constraint body {...} or just semicolon
    if p.at(SyntaxKind::L_BRACE) || p.at(SyntaxKind::L_PAREN) {
        p.parse_constraint_body();
    } else {
        p.expect(SyntaxKind::SEMICOLON);
    }

    p.finish_node(); // USAGE
}

/// RequirementVerification = ('assert'? 'not'? 'satisfy' | 'verify') 'requirement'? Identification? ('by' QualifiedName)? ';'
/// Per pest: requirement_verification_member = { satisfy_requirement_usage | verify_requirement_usage }
/// Per pest: satisfy_requirement_usage = { "assert"? ~ "not"? ~ "satisfy" ~ "requirement"? ~ usage_declaration? ~ value_part? ~ (";"|requirement_body) }
/// Per pest: verify_requirement_usage = { "verify" ~ "requirement"? ~ usage_declaration? ~ ("by" ~ qualified_name)? ~ (";"|requirement_body) }
/// Pattern: [assert] [not] satisfy|verify [requirement] <name|typing>? [by <verifier>]? <body|semicolon>
pub fn parse_requirement_verification<P: SysMLParser>(p: &mut P) {
    // Wrap in USAGE node so it gets extracted by NamespaceMember::cast
    p.start_node(SyntaxKind::USAGE);

    p.start_node(SyntaxKind::REQUIREMENT_VERIFICATION);

    // Optional 'assert' modifier
    consume_if(p, SyntaxKind::ASSERT_KW);

    // Optional 'not' modifier
    consume_if(p, SyntaxKind::NOT_KW);

    // satisfy/verify
    if p.at(SyntaxKind::SATISFY_KW) || p.at(SyntaxKind::VERIFY_KW) {
        bump_keyword(p);
    }

    // Optional 'requirement' keyword
    consume_if(p, SyntaxKind::REQUIREMENT_KW);

    // Target: can be usage declaration (name : Type), anonymous typing (: Type), or qualified reference
    if p.at(SyntaxKind::COLON) {
        // Anonymous requirement with typing: verify requirement : R;
        p.parse_typing();
        p.skip_trivia();
    } else if p.at_name_token() || p.at(SyntaxKind::LT) {
        // Parse as qualified name to support Requirements::engineSpecification
        parse_qualified_name_and_skip(p);

        // Optional typing (only COLON, not specialization operators)
        if p.at(SyntaxKind::COLON) {
            p.parse_typing();
            p.skip_trivia();
        }
    }

    // Optional 'by' clause
    if consume_if(p, SyntaxKind::BY_KW) {
        p.parse_qualified_name();
        p.skip_trivia();
    }

    p.finish_node(); // REQUIREMENT_VERIFICATION

    // Body or semicolon (body allows binding parameters)
    if p.at(SyntaxKind::L_BRACE) {
        p.parse_constraint_body();
    } else {
        p.expect(SyntaxKind::SEMICOLON);
    }

    p.finish_node(); // USAGE
}

/// ExhibitUsage = 'exhibit' 'state'? QualifiedName ';'
/// Per pest: case_exhibit_member = { "exhibit" ~ (exhibit_state_usage|owned_reference) }
/// Per pest: exhibit_state_usage = { "state" ~ usage_declaration? ~ state_body }
/// Pattern: exhibit [state <declaration> <body>] | exhibit <reference> semicolon
pub fn parse_exhibit_usage<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    expect_and_skip(p, SyntaxKind::EXHIBIT_KW);

    // Check if this is 'exhibit state' (exhibit state usage with full declaration)
    if p.at(SyntaxKind::STATE_KW) {
        bump_keyword(p); // state

        // Parse action_usage_declaration (identification, etc.)
        if p.at_name_token() || p.at(SyntaxKind::LT) {
            p.parse_identification();
        }
        p.skip_trivia();

        // Multiplicity
        if p.at(SyntaxKind::L_BRACKET) {
            p.parse_multiplicity();
        }
        p.skip_trivia();

        // Typing
        if p.at(SyntaxKind::COLON) {
            p.parse_typing();
        }
        p.skip_trivia();

        // Specializations
        parse_specializations(p);
        p.skip_trivia();

        // State body (with optional parallel marker)
        parse_state_body(p);
    } else {
        // Simple exhibit reference: exhibit <name> ;
        if p.at_name_token() {
            p.parse_qualified_name();
        }

        p.skip_trivia();
        p.expect(SyntaxKind::SEMICOLON);
    }

    p.finish_node();
}

/// Parse allocate usage: allocate <source> to <target> ;
pub fn parse_allocate_usage<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    expect_and_skip(p, SyntaxKind::ALLOCATE_KW);

    // Check for n-ary pattern: allocate (a, b, c)
    if p.at(SyntaxKind::L_PAREN) {
        bump_keyword(p); // (

        // Parse first member
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        }

        // Parse additional members
        while p.at(SyntaxKind::COMMA) {
            bump_keyword(p); // ,
            if p.at_name_token() {
                p.parse_qualified_name();
                p.skip_trivia();
            }
        }

        p.expect(SyntaxKind::R_PAREN);
        p.skip_trivia();
    } else {
        // Binary pattern: allocate source to target
        // Source
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        }

        // 'to' keyword
        if p.at(SyntaxKind::TO_KW) {
            p.bump();
            p.skip_trivia();

            // Target
            if p.at_name_token() {
                p.parse_qualified_name();
                p.skip_trivia();
            }
        }
    }

    // Body or semicolon (body can contain nested allocate statements)
    parse_body_or_semicolon(p);

    p.finish_node();
}

/// IncludeUsage = 'include' 'use'? 'case'? (Name | QualifiedName) Typing? Specializations? ';'
/// When followed by typing/specialization, the first identifier is a NAME (defines new element)
/// Otherwise, it's a QualifiedName (references existing element)
pub fn parse_include_usage<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    expect_and_skip(p, SyntaxKind::INCLUDE_KW);

    // Optional 'use case' keywords
    consume_if(p, SyntaxKind::USE_KW);
    consume_if(p, SyntaxKind::CASE_KW);

    // Peek ahead to determine if this is a name (followed by typing/specialization) or a reference
    // If next identifier is followed by : or :> etc, treat it as a NAME
    if p.at_name_token() {
        let peek1 = p.peek_kind(1);
        let has_typing_or_spec = matches!(
            peek1,
            SyntaxKind::COLON
                | SyntaxKind::TYPED_KW
                | SyntaxKind::OF_KW
                | SyntaxKind::COLON_GT
                | SyntaxKind::COLON_GT_GT
                | SyntaxKind::COLON_COLON_GT
                | SyntaxKind::SPECIALIZES_KW
                | SyntaxKind::SUBSETS_KW
                | SyntaxKind::REDEFINES_KW
                | SyntaxKind::REFERENCES_KW
                | SyntaxKind::L_BRACKET  // multiplicity after name
                | SyntaxKind::L_BRACE // body after name
        );

        if has_typing_or_spec {
            // This is a name (defines new element)
            p.parse_identification();
        } else {
            // This is a reference to existing element
            p.parse_qualified_name();
        }
    }

    p.skip_trivia();

    // Optional specializations (e.g., :>, redefines, etc.)
    if p.at_any(&[
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
    ]) {
        parse_specializations(p);
        p.skip_trivia();
    }

    // Optional multiplicity
    if p.at(SyntaxKind::L_BRACKET) {
        p.parse_multiplicity();
        p.skip_trivia();
    }

    // Body or semicolon
    parse_body_or_semicolon(p);

    p.finish_node();
}

/// Expose statement: expose QualifiedName ('::' ('*' | '**'))? ';'
pub fn parse_expose_statement<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::IMPORT);

    p.expect(SyntaxKind::EXPOSE_KW);
    p.skip_trivia();

    // Qualified name with optional wildcard
    if p.at_name_token() {
        p.parse_qualified_name();
        p.skip_trivia();
    }

    // Optional wildcard suffix: :: * or :: **
    if p.at(SyntaxKind::COLON_COLON) {
        p.bump();
        p.skip_trivia();
        if p.at(SyntaxKind::STAR) || p.at(SyntaxKind::STAR_STAR) {
            p.bump();
            p.skip_trivia();
        }
    }

    p.expect(SyntaxKind::SEMICOLON);

    p.finish_node();
}
