//! Rowan-based formatter for SysML/KerML
//!
//! This module provides lossless formatting that preserves comments and trivia.
//! It uses Rowan for CST (Concrete Syntax Tree) representation and Logos for lexing.

mod lexer;
mod options;

#[cfg(test)]
mod tests;

use crate::parser::{SyntaxElement, SyntaxKind, SyntaxNode};
use lexer::{Token, tokenize};
pub use options::FormatOptions;
use rowan::GreenNodeBuilder;
use tokio_util::sync::CancellationToken;

/// Format SysML/KerML source code with cancellation support.
/// Returns `None` if the cancellation token is signalled.
pub fn format_async(
    source: &str,
    options: &FormatOptions,
    cancel: &CancellationToken,
) -> Option<String> {
    let tokens = tokenize(source);
    let cst = parse_to_cst(&tokens, cancel)?;
    render(&cst, options, cancel)
}

/// Parse tokens into a CST with cancellation support
fn parse_to_cst(tokens: &[Token], cancel: &CancellationToken) -> Option<SyntaxNode> {
    let mut builder = GreenNodeBuilder::new();

    builder.start_node(SyntaxKind::SOURCE_FILE.into());

    let mut pos = 0;
    while pos < tokens.len() {
        if cancel.is_cancelled() {
            return None;
        }
        pos = parse_element(tokens, pos, &mut builder);
    }

    builder.finish_node();

    Some(SyntaxNode::new_root(builder.finish()))
}

/// Parse a single element (package, definition, usage, import, comment, etc.)
fn parse_element(tokens: &[Token], mut pos: usize, builder: &mut GreenNodeBuilder) -> usize {
    // Consume leading trivia
    pos = consume_trivia(tokens, pos, builder);

    if pos >= tokens.len() {
        return pos;
    }

    let token = &tokens[pos];

    match token.kind {
        SyntaxKind::PACKAGE_KW => parse_package(tokens, pos, builder),
        SyntaxKind::PART_KW
        | SyntaxKind::ATTRIBUTE_KW
        | SyntaxKind::PORT_KW
        | SyntaxKind::ITEM_KW
        | SyntaxKind::ACTION_KW
        | SyntaxKind::STATE_KW
        | SyntaxKind::REQUIREMENT_KW
        | SyntaxKind::CONSTRAINT_KW
        | SyntaxKind::CONNECTION_KW
        | SyntaxKind::ALLOCATION_KW
        | SyntaxKind::INTERFACE_KW
        | SyntaxKind::FLOW_KW
        | SyntaxKind::USE_KW
        | SyntaxKind::VIEW_KW
        | SyntaxKind::VIEWPOINT_KW
        | SyntaxKind::RENDERING_KW
        | SyntaxKind::METADATA_KW
        | SyntaxKind::OCCURRENCE_KW
        | SyntaxKind::ANALYSIS_KW
        | SyntaxKind::VERIFICATION_KW
        | SyntaxKind::CONCERN_KW
        | SyntaxKind::ENUM_KW
        | SyntaxKind::CALC_KW
        | SyntaxKind::CASE_KW
        | SyntaxKind::INDIVIDUAL_KW => parse_definition_or_usage(tokens, pos, builder),
        SyntaxKind::ABSTRACT_KW | SyntaxKind::REF_KW | SyntaxKind::CONST_KW => {
            parse_definition_or_usage(tokens, pos, builder)
        }
        SyntaxKind::IMPORT_KW => parse_import(tokens, pos, builder),
        SyntaxKind::ALIAS_KW => parse_alias(tokens, pos, builder),
        SyntaxKind::DOC_KW | SyntaxKind::COMMENT_KW => parse_annotation(tokens, pos, builder),
        _ => {
            // Unknown token, just add it and move on
            builder.token(token.kind.into(), token.text);
            pos + 1
        }
    }
}

/// Consume trivia (whitespace, comments) and add to tree
fn consume_trivia(tokens: &[Token], mut pos: usize, builder: &mut GreenNodeBuilder) -> usize {
    while pos < tokens.len() {
        let token = &tokens[pos];
        match token.kind {
            SyntaxKind::WHITESPACE | SyntaxKind::LINE_COMMENT | SyntaxKind::BLOCK_COMMENT => {
                builder.token(token.kind.into(), token.text);
                pos += 1;
            }
            _ => break,
        }
    }
    pos
}

/// Parse a package declaration
fn parse_package(tokens: &[Token], mut pos: usize, builder: &mut GreenNodeBuilder) -> usize {
    builder.start_node(SyntaxKind::PACKAGE.into());

    // 'package' keyword
    builder.token(tokens[pos].kind.into(), tokens[pos].text);
    pos += 1;

    pos = consume_trivia(tokens, pos, builder);

    // Optional name
    if pos < tokens.len() && tokens[pos].kind == SyntaxKind::IDENT {
        builder.start_node(SyntaxKind::NAME.into());
        builder.token(tokens[pos].kind.into(), tokens[pos].text);
        builder.finish_node();
        pos += 1;
    }

    pos = consume_trivia(tokens, pos, builder);

    // Body or semicolon
    if pos < tokens.len() {
        if tokens[pos].kind == SyntaxKind::L_BRACE {
            pos = parse_body(tokens, pos, builder);
        } else if tokens[pos].kind == SyntaxKind::SEMICOLON {
            builder.token(tokens[pos].kind.into(), tokens[pos].text);
            pos += 1;
        }
    }

    builder.finish_node();
    pos
}

/// Parse a block body { ... }
fn parse_body(tokens: &[Token], mut pos: usize, builder: &mut GreenNodeBuilder) -> usize {
    builder.start_node(SyntaxKind::NAMESPACE_BODY.into());

    // Opening brace
    builder.token(tokens[pos].kind.into(), tokens[pos].text);
    pos += 1;

    // Parse elements until closing brace
    while pos < tokens.len() && tokens[pos].kind != SyntaxKind::R_BRACE {
        let prev_pos = pos;
        pos = parse_element(tokens, pos, builder);
        if pos == prev_pos {
            // Avoid infinite loop - consume unknown token
            builder.token(tokens[pos].kind.into(), tokens[pos].text);
            pos += 1;
        }
    }

    // Closing brace
    if pos < tokens.len() && tokens[pos].kind == SyntaxKind::R_BRACE {
        pos = consume_trivia(tokens, pos, builder);
        if pos < tokens.len() && tokens[pos].kind == SyntaxKind::R_BRACE {
            builder.token(tokens[pos].kind.into(), tokens[pos].text);
            pos += 1;
        }
    }

    builder.finish_node();
    pos
}

/// Parse a definition or usage (part def, part, attribute, etc.)
fn parse_definition_or_usage(
    tokens: &[Token],
    mut pos: usize,
    builder: &mut GreenNodeBuilder,
) -> usize {
    // Determine if this is a definition (has 'def' keyword) or usage
    let is_definition = has_def_keyword(tokens, pos);

    if is_definition {
        builder.start_node(SyntaxKind::DEFINITION.into());
    } else {
        builder.start_node(SyntaxKind::USAGE.into());
    }

    // Consume modifiers (abstract, ref, const)
    while pos < tokens.len() {
        match tokens[pos].kind {
            SyntaxKind::ABSTRACT_KW | SyntaxKind::REF_KW | SyntaxKind::CONST_KW => {
                builder.token(tokens[pos].kind.into(), tokens[pos].text);
                pos += 1;
                pos = consume_trivia(tokens, pos, builder);
            }
            _ => break,
        }
    }

    // Keyword (part, attribute, etc.)
    if pos < tokens.len() && is_element_keyword(&tokens[pos].kind) {
        builder.token(tokens[pos].kind.into(), tokens[pos].text);
        pos += 1;
        pos = consume_trivia(tokens, pos, builder);
    }

    // 'def' keyword if definition
    if pos < tokens.len() && tokens[pos].kind == SyntaxKind::DEF_KW {
        builder.token(tokens[pos].kind.into(), tokens[pos].text);
        pos += 1;
        pos = consume_trivia(tokens, pos, builder);
    }

    // Name
    if pos < tokens.len() && tokens[pos].kind == SyntaxKind::IDENT {
        builder.start_node(SyntaxKind::NAME.into());
        builder.token(tokens[pos].kind.into(), tokens[pos].text);
        builder.finish_node();
        pos += 1;
    }

    pos = consume_trivia(tokens, pos, builder);

    // Relationships and type annotations (consume until { or ;)
    while pos < tokens.len() {
        match tokens[pos].kind {
            SyntaxKind::L_BRACE | SyntaxKind::SEMICOLON => break,
            _ => {
                builder.token(tokens[pos].kind.into(), tokens[pos].text);
                pos += 1;
            }
        }
    }

    // Body or semicolon
    if pos < tokens.len() {
        if tokens[pos].kind == SyntaxKind::L_BRACE {
            pos = parse_body(tokens, pos, builder);
        } else if tokens[pos].kind == SyntaxKind::SEMICOLON {
            builder.token(tokens[pos].kind.into(), tokens[pos].text);
            pos += 1;
        }
    }

    builder.finish_node();
    pos
}

/// Check if a sequence starting at pos has a 'def' keyword before { or ;
fn has_def_keyword(tokens: &[Token], mut pos: usize) -> bool {
    while pos < tokens.len() {
        match tokens[pos].kind {
            SyntaxKind::DEF_KW => return true,
            SyntaxKind::L_BRACE | SyntaxKind::SEMICOLON => return false,
            _ => pos += 1,
        }
    }
    false
}

/// Check if a kind is an element keyword
fn is_element_keyword(kind: &SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::PART_KW
            | SyntaxKind::ATTRIBUTE_KW
            | SyntaxKind::PORT_KW
            | SyntaxKind::ITEM_KW
            | SyntaxKind::ACTION_KW
            | SyntaxKind::STATE_KW
            | SyntaxKind::REQUIREMENT_KW
            | SyntaxKind::CONSTRAINT_KW
            | SyntaxKind::CONNECTION_KW
            | SyntaxKind::ALLOCATION_KW
            | SyntaxKind::INTERFACE_KW
            | SyntaxKind::FLOW_KW
            | SyntaxKind::USE_KW
            | SyntaxKind::VIEW_KW
            | SyntaxKind::VIEWPOINT_KW
            | SyntaxKind::RENDERING_KW
            | SyntaxKind::METADATA_KW
            | SyntaxKind::OCCURRENCE_KW
            | SyntaxKind::ANALYSIS_KW
            | SyntaxKind::VERIFICATION_KW
            | SyntaxKind::CONCERN_KW
            | SyntaxKind::ENUM_KW
            | SyntaxKind::CALC_KW
            | SyntaxKind::CASE_KW
            | SyntaxKind::INDIVIDUAL_KW
    )
}

/// Parse an import statement
fn parse_import(tokens: &[Token], mut pos: usize, builder: &mut GreenNodeBuilder) -> usize {
    builder.start_node(SyntaxKind::IMPORT.into());

    // 'import' keyword
    builder.token(tokens[pos].kind.into(), tokens[pos].text);
    pos += 1;

    // Consume until semicolon
    while pos < tokens.len() && tokens[pos].kind != SyntaxKind::SEMICOLON {
        builder.token(tokens[pos].kind.into(), tokens[pos].text);
        pos += 1;
    }

    // Semicolon
    if pos < tokens.len() && tokens[pos].kind == SyntaxKind::SEMICOLON {
        builder.token(tokens[pos].kind.into(), tokens[pos].text);
        pos += 1;
    }

    builder.finish_node();
    pos
}

/// Parse an alias declaration
fn parse_alias(tokens: &[Token], mut pos: usize, builder: &mut GreenNodeBuilder) -> usize {
    builder.start_node(SyntaxKind::ALIAS_MEMBER.into());

    // 'alias' keyword
    builder.token(tokens[pos].kind.into(), tokens[pos].text);
    pos += 1;

    // Consume until semicolon
    while pos < tokens.len() && tokens[pos].kind != SyntaxKind::SEMICOLON {
        builder.token(tokens[pos].kind.into(), tokens[pos].text);
        pos += 1;
    }

    // Semicolon
    if pos < tokens.len() && tokens[pos].kind == SyntaxKind::SEMICOLON {
        builder.token(tokens[pos].kind.into(), tokens[pos].text);
        pos += 1;
    }

    builder.finish_node();
    pos
}

/// Parse a doc or comment annotation
fn parse_annotation(tokens: &[Token], mut pos: usize, builder: &mut GreenNodeBuilder) -> usize {
    builder.start_node(SyntaxKind::COMMENT_ELEMENT.into());

    // 'doc' or 'comment' keyword
    builder.token(tokens[pos].kind.into(), tokens[pos].text);
    pos += 1;

    // Consume until end of annotation (block comment or semicolon)
    while pos < tokens.len() {
        let kind = tokens[pos].kind;
        builder.token(tokens[pos].kind.into(), tokens[pos].text);
        pos += 1;

        if kind == SyntaxKind::BLOCK_COMMENT || kind == SyntaxKind::SEMICOLON {
            break;
        }
    }

    builder.finish_node();
    pos
}

/// Render the CST back to formatted source code with cancellation support
fn render(
    node: &SyntaxNode,
    options: &FormatOptions,
    cancel: &CancellationToken,
) -> Option<String> {
    let mut output = String::new();
    let mut indent_level: usize = 0;
    let mut at_line_start = true;

    render_node(
        node,
        options,
        &mut output,
        &mut indent_level,
        &mut at_line_start,
        cancel,
    )?;

    Some(output)
}

fn render_node(
    node: &SyntaxNode,
    options: &FormatOptions,
    output: &mut String,
    indent_level: &mut usize,
    at_line_start: &mut bool,
    cancel: &CancellationToken,
) -> Option<()> {
    // Collect children for lookahead
    let children: Vec<SyntaxElement> = node.children_with_tokens().collect();

    for (i, child) in children.iter().enumerate() {
        if cancel.is_cancelled() {
            return None;
        }

        match child {
            rowan::NodeOrToken::Token(token) => {
                let kind: SyntaxKind = token.kind();
                let text = token.text();

                // Look ahead to next non-whitespace token
                let next_significant = children[i + 1..].iter().find_map(|c| match c {
                    rowan::NodeOrToken::Token(t) if t.kind() != SyntaxKind::WHITESPACE => {
                        Some(t.kind())
                    }
                    _ => None,
                });

                match kind {
                    SyntaxKind::WHITESPACE => {
                        // Don't preserve newlines before opening brace - keep it on same line
                        if next_significant == Some(SyntaxKind::L_BRACE) {
                            // Just add a single space, brace will be on same line
                            if !*at_line_start && !output.ends_with(' ') && !output.is_empty() {
                                output.push(' ');
                            }
                        } else if text.contains('\n') {
                            // Preserve newlines for other cases
                            let newline_count = text.matches('\n').count();
                            for _ in 0..newline_count.min(2) {
                                output.push('\n');
                            }
                            *at_line_start = true;
                        } else if !*at_line_start && !output.ends_with(' ') && !output.is_empty() {
                            // Single space between tokens
                            output.push(' ');
                        }
                    }
                    SyntaxKind::LINE_COMMENT => {
                        if *at_line_start {
                            output.push_str(&options.indent(*indent_level));
                            *at_line_start = false;
                        }
                        output.push_str(text);
                    }
                    SyntaxKind::BLOCK_COMMENT => {
                        if *at_line_start {
                            output.push_str(&options.indent(*indent_level));
                            *at_line_start = false;
                        }
                        output.push_str(text);
                    }
                    SyntaxKind::L_BRACE => {
                        // Ensure space before brace if not at line start
                        if !*at_line_start && !output.ends_with(' ') && !output.ends_with('\n') {
                            output.push(' ');
                        }
                        // If at line start but we want brace on same line, remove trailing newlines
                        if *at_line_start && !output.is_empty() {
                            // Remove trailing newlines to put brace on same line
                            while output.ends_with('\n') {
                                output.pop();
                            }
                            if !output.ends_with(' ') {
                                output.push(' ');
                            }
                            *at_line_start = false;
                        }
                        output.push('{');
                        *indent_level += 1;
                    }
                    SyntaxKind::R_BRACE => {
                        *indent_level = indent_level.saturating_sub(1);
                        if *at_line_start {
                            output.push_str(&options.indent(*indent_level));
                        }
                        output.push('}');
                        *at_line_start = false;
                    }
                    SyntaxKind::SEMICOLON => {
                        output.push(';');
                        *at_line_start = false;
                    }
                    SyntaxKind::COLON | SyntaxKind::COLON_COLON | SyntaxKind::DOT => {
                        // No space before colons and dots
                        output.push_str(text);
                        *at_line_start = false;
                    }
                    _ => {
                        if *at_line_start {
                            output.push_str(&options.indent(*indent_level));
                            *at_line_start = false;
                        }
                        // Don't add automatic spaces - let whitespace tokens handle spacing
                        output.push_str(text);
                    }
                }
            }
            rowan::NodeOrToken::Node(child_node) => {
                render_node(
                    child_node,
                    options,
                    output,
                    indent_level,
                    at_line_start,
                    cancel,
                )?;
            }
        }
    }
    Some(())
}
