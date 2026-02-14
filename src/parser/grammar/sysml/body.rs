use super::*;

// =============================================================================
// SysML Body Parsing
// =============================================================================

/// Parse a SysML body (semicolon or braced block with SysML members)
/// Per pest: package_body = { semi_colon | ( "{" ~ package_body_items ~ "}" ) }
pub fn parse_body<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::NAMESPACE_BODY);

    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
    } else if p.at(SyntaxKind::L_BRACE) {
        p.bump();
        p.skip_trivia();

        while !p.at(SyntaxKind::ERROR) && !p.at(SyntaxKind::R_BRACE) {
            let start_pos = p.get_pos();

            parse_package_body_element(p);

            p.skip_trivia();

            // Recovery if no progress made
            if p.get_pos() == start_pos && !p.at(SyntaxKind::ERROR) && !p.at(SyntaxKind::R_BRACE) {
                let got = if let Some(text) = p.current_token_text() {
                    format!("'{}' ({})", text, p.current_kind().display_name())
                } else {
                    p.current_kind().display_name().to_string()
                };
                p.error(format!(
                    "unexpected {} in definition body—expected a member declaration",
                    got
                ));
                p.bump();
            }
        }

        p.expect(SyntaxKind::R_BRACE);
    } else {
        // Provide more context about what we found
        let found = if let Some(text) = p.current_token_text() {
            format!("'{}' ({})", text, p.current_kind().display_name())
        } else {
            "end of file".to_string()
        };
        p.error(format!(
            "expected ';' to end declaration or '{{' to start body, found {}",
            found
        ))
    }

    p.finish_node();
}
