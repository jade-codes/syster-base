//! Parse context tracking for context-aware error messages
//!
//! The parser maintains a stack of contexts to generate more helpful
//! error messages that indicate where in the source structure the error occurred.

use crate::parser::SyntaxKind;

/// Represents the current parsing context
///
/// Used to generate context-aware error messages and determine
/// appropriate recovery strategies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseContext {
    /// At the top level of a file
    TopLevel,
    /// Inside a package body
    PackageBody,
    /// Inside a namespace body
    NamespaceBody,

    // Definition contexts
    /// Parsing a part definition
    PartDefinition,
    /// Parsing an action definition
    ActionDefinition,
    /// Parsing a state definition
    StateDefinition,
    /// Parsing a requirement definition
    RequirementDefinition,
    /// Parsing a constraint definition
    ConstraintDefinition,
    /// Parsing a use case definition
    UseCaseDefinition,
    /// Parsing any other definition
    Definition,

    // Body contexts
    /// Inside an action body
    ActionBody,
    /// Inside a state body
    StateBody,
    /// Inside a requirement body
    RequirementBody,
    /// Inside a use case body
    UseCaseBody,
    /// Inside a generic definition body
    DefinitionBody,

    // Other contexts
    /// Parsing an expression
    Expression,
    /// Parsing a type annotation
    TypeAnnotation,
    /// Parsing a multiplicity `[...]`
    Multiplicity,
    /// Parsing an import statement
    Import,
    /// Parsing a parameter list
    ParameterList,
    /// Parsing an argument list
    ArgumentList,
    /// Parsing a transition
    Transition,
    /// Parsing a feature chain
    FeatureChain,
}

impl ParseContext {
    /// Get a human-readable description of this context for error messages
    pub fn description(&self) -> &'static str {
        match self {
            Self::TopLevel => "at top level",
            Self::PackageBody => "in package body",
            Self::NamespaceBody => "in namespace body",

            Self::PartDefinition => "in part definition",
            Self::ActionDefinition => "in action definition",
            Self::StateDefinition => "in state definition",
            Self::RequirementDefinition => "in requirement definition",
            Self::ConstraintDefinition => "in constraint definition",
            Self::UseCaseDefinition => "in use case definition",
            Self::Definition => "in definition",

            Self::ActionBody => "in action body",
            Self::StateBody => "in state body",
            Self::RequirementBody => "in requirement body",
            Self::UseCaseBody => "in use case body",
            Self::DefinitionBody => "in definition body",

            Self::Expression => "in expression",
            Self::TypeAnnotation => "in type annotation",
            Self::Multiplicity => "in multiplicity",
            Self::Import => "in import statement",
            Self::ParameterList => "in parameter list",
            Self::ArgumentList => "in argument list",
            Self::Transition => "in transition",
            Self::FeatureChain => "in feature chain",
        }
    }

    /// Get a description of what tokens are expected in this context
    pub fn expected_description(&self) -> &'static str {
        match self {
            Self::TopLevel => "a package, definition, or import",
            Self::PackageBody | Self::NamespaceBody => {
                "a definition (part, action, etc.), usage, or import"
            }

            Self::PartDefinition => "part definition elements",
            Self::ActionDefinition => "action definition elements",
            Self::StateDefinition => "state definition elements",
            Self::RequirementDefinition => "requirement definition elements",
            Self::ConstraintDefinition => "constraint definition elements",
            Self::UseCaseDefinition => "use case elements (include, actor, etc.)",
            Self::Definition => "definition elements",

            Self::ActionBody => "action elements (accept, send, if, perform, etc.)",
            Self::StateBody => "state elements (entry, exit, do) or transitions",
            Self::RequirementBody => "requirement elements (subject, actor, constraint, etc.)",
            Self::UseCaseBody => "use case elements (include, actor, objective, etc.)",
            Self::DefinitionBody => "definition body elements",

            Self::Expression => "an expression (literal, identifier, or operator)",
            Self::TypeAnnotation => "a type name",
            Self::Multiplicity => "a multiplicity range (e.g., 0..*, 1, 1..5)",
            Self::Import => "an import path",
            Self::ParameterList => "a parameter",
            Self::ArgumentList => "an argument",
            Self::Transition => "transition elements (trigger, guard, effect, then)",
            Self::FeatureChain => "a feature name",
        }
    }

    /// Get the recovery tokens appropriate for this context
    pub fn recovery_tokens(&self) -> &'static [SyntaxKind] {
        match self {
            Self::TopLevel => &[
                SyntaxKind::PACKAGE_KW,
                SyntaxKind::PART_KW,
                SyntaxKind::ACTION_KW,
                SyntaxKind::STATE_KW,
                SyntaxKind::IMPORT_KW,
                SyntaxKind::REQUIREMENT_KW,
                SyntaxKind::SEMICOLON, // For recovery at statement boundaries
            ],
            Self::PackageBody | Self::NamespaceBody => &[
                SyntaxKind::PART_KW,
                SyntaxKind::ACTION_KW,
                SyntaxKind::STATE_KW,
                SyntaxKind::REQUIREMENT_KW,
                SyntaxKind::CONSTRAINT_KW,
                SyntaxKind::IMPORT_KW,
                SyntaxKind::PACKAGE_KW,
                SyntaxKind::ATTRIBUTE_KW,
                SyntaxKind::PORT_KW,
                SyntaxKind::ITEM_KW,
                SyntaxKind::R_BRACE,
                SyntaxKind::PUBLIC_KW,
                SyntaxKind::PRIVATE_KW,
            ],
            Self::ActionBody => &[
                SyntaxKind::ACCEPT_KW,
                SyntaxKind::SEND_KW,
                SyntaxKind::IF_KW,
                SyntaxKind::WHILE_KW,
                SyntaxKind::FOR_KW,
                SyntaxKind::ACTION_KW,
                SyntaxKind::THEN_KW,
                SyntaxKind::ELSE_KW,
                SyntaxKind::PERFORM_KW,
                SyntaxKind::ASSIGN_KW,
                SyntaxKind::R_BRACE,
            ],
            Self::StateBody => &[
                SyntaxKind::ENTRY_KW,
                SyntaxKind::EXIT_KW,
                SyntaxKind::DO_KW,
                SyntaxKind::TRANSITION_KW,
                SyntaxKind::STATE_KW,
                SyntaxKind::R_BRACE,
            ],
            Self::RequirementBody => &[
                SyntaxKind::SUBJECT_KW,
                SyntaxKind::ACTOR_KW,
                SyntaxKind::STAKEHOLDER_KW,
                SyntaxKind::OBJECTIVE_KW,
                SyntaxKind::REQUIRE_KW,
                SyntaxKind::ASSUME_KW,
                SyntaxKind::CONSTRAINT_KW,
                SyntaxKind::R_BRACE,
            ],
            Self::UseCaseBody => &[
                SyntaxKind::INCLUDE_KW,
                SyntaxKind::ACTOR_KW,
                SyntaxKind::SUBJECT_KW,
                SyntaxKind::OBJECTIVE_KW,
                SyntaxKind::R_BRACE,
            ],
            Self::Expression | Self::ArgumentList => &[
                SyntaxKind::SEMICOLON,
                SyntaxKind::R_PAREN,
                SyntaxKind::R_BRACE,
                SyntaxKind::R_BRACKET,
                SyntaxKind::COMMA,
            ],
            Self::Multiplicity => &[SyntaxKind::R_BRACKET, SyntaxKind::SEMICOLON],
            Self::ParameterList => &[SyntaxKind::R_PAREN, SyntaxKind::COMMA, SyntaxKind::L_BRACE],
            Self::Import => &[SyntaxKind::SEMICOLON, SyntaxKind::R_BRACE],
            Self::Transition => &[
                SyntaxKind::THEN_KW,
                SyntaxKind::SEMICOLON,
                SyntaxKind::R_BRACE,
            ],
            Self::FeatureChain => &[
                SyntaxKind::SEMICOLON,
                SyntaxKind::L_BRACE,
                SyntaxKind::COMMA,
            ],
            _ => &[SyntaxKind::SEMICOLON, SyntaxKind::R_BRACE],
        }
    }

    /// Check if this context is inside a definition
    pub fn is_in_definition(&self) -> bool {
        matches!(
            self,
            Self::PartDefinition
                | Self::ActionDefinition
                | Self::StateDefinition
                | Self::RequirementDefinition
                | Self::ConstraintDefinition
                | Self::UseCaseDefinition
                | Self::Definition
        )
    }

    /// Check if this context is inside a body
    pub fn is_in_body(&self) -> bool {
        matches!(
            self,
            Self::ActionBody
                | Self::StateBody
                | Self::RequirementBody
                | Self::UseCaseBody
                | Self::DefinitionBody
                | Self::PackageBody
                | Self::NamespaceBody
        )
    }
}

impl Default for ParseContext {
    fn default() -> Self {
        Self::TopLevel
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_description() {
        assert_eq!(ParseContext::TopLevel.description(), "at top level");
        assert_eq!(ParseContext::PackageBody.description(), "in package body");
        assert_eq!(ParseContext::ActionBody.description(), "in action body");
    }

    #[test]
    fn test_context_expected_description() {
        assert!(ParseContext::PackageBody
            .expected_description()
            .contains("definition"));
        assert!(ParseContext::ActionBody
            .expected_description()
            .contains("accept"));
    }

    #[test]
    fn test_recovery_tokens_not_empty() {
        assert!(!ParseContext::TopLevel.recovery_tokens().is_empty());
        assert!(!ParseContext::PackageBody.recovery_tokens().is_empty());
        assert!(!ParseContext::ActionBody.recovery_tokens().is_empty());
    }

    #[test]
    fn test_is_in_definition() {
        assert!(ParseContext::PartDefinition.is_in_definition());
        assert!(ParseContext::ActionDefinition.is_in_definition());
        assert!(!ParseContext::ActionBody.is_in_definition());
        assert!(!ParseContext::TopLevel.is_in_definition());
    }

    #[test]
    fn test_is_in_body() {
        assert!(ParseContext::ActionBody.is_in_body());
        assert!(ParseContext::PackageBody.is_in_body());
        assert!(!ParseContext::PartDefinition.is_in_body());
        assert!(!ParseContext::Expression.is_in_body());
    }

    #[test]
    fn test_default_context() {
        assert_eq!(ParseContext::default(), ParseContext::TopLevel);
    }
}
