//! Rule-based parser for testing individual grammar rules
//!
//! This module provides a Pest-like API for parsing individual grammar constructs
//! without requiring full file context. Useful for testing specific rules in isolation.
//!
//! # Example
//!
//! ```ignore
//! use syster::parser::rule_parser::{Rule, parse_rule};
//!
//! let result = parse_rule(Rule::ItemFlow, "flow myFlow from a to b;");
//! assert!(result.is_ok());
//! ```

use super::lexer::Lexer;
use super::parser::{Parse, Parser, LanguageMode};
use super::grammar::{kerml, sysml};

/// Grammar rules that can be parsed individually
///
/// These map to the Pest grammar rules from the original parser.
/// Each rule corresponds to a specific parsing function in the grammar modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    // === KerML Namespace Elements ===
    /// package = { ... }
    Package,
    /// library_package = { ... }
    LibraryPackage,
    /// import = { ... }
    Import,
    /// namespace = { ... }
    Namespace,
    /// dependency = { ... }
    Dependency,
    
    // === KerML Definitions ===
    /// class = { ... }
    Class,
    /// datatype = { data_type_token ~ ... }
    DataType,
    /// struct = { ... }
    Structure,
    /// association = { assoc ~ ... }
    Association,
    /// association_structure = { assoc ~ struct ~ ... }
    AssociationStructure,
    /// behavior = { ... }
    Behavior,
    /// function = { ... }
    Function,
    /// predicate = { ... }
    Predicate,
    /// interaction = { ... }
    Interaction,
    /// metaclass = { ... }
    Metaclass,
    /// classifier = { ... }
    Classifier,
    /// type_def = { type ~ ... }
    TypeDef,
    
    // === KerML Usages/Features ===
    /// feature = { ... }
    Feature,
    /// step = { ... }
    Step,
    /// expression = { expr ~ ... }
    Expression,
    /// boolean_expression = { ... }
    BooleanExpression,
    /// invariant = { inv ~ ... }
    Invariant,
    /// multiplicity = { ... }
    Multiplicity,
    /// multiplicity_range = { ... }
    MultiplicityRange,
    
    // === KerML Connectors/Flows ===
    /// connector = { ... }
    Connector,
    /// binding_connector = { binding ~ ... }
    BindingConnector,
    /// succession = { ... }
    Succession,
    /// item_flow = { flow ~ ... }
    ItemFlow,
    /// succession_item_flow = { succession ~ flow ~ ... }
    SuccessionItemFlow,
    
    // === KerML Relationships ===
    /// specialization = { :> | specializes }
    Specialization,
    /// subsetting = { subsets ~ ... }
    Subsetting,
    /// redefinition = { :>> | redefines }
    Redefinition,
    /// feature_typing = { : | typed by }
    FeatureTyping,
    /// conjugation = { ~ | conjugates }
    Conjugation,
    /// disjoining = { disjoint ~ ... }
    Disjoining,
    /// feature_inverting = { inverse ~ ... of ~ ... }
    FeatureInverting,
    /// feature_chaining = { chains ~ ... }
    FeatureChaining,
    /// subclassification = { subclassifier ~ ... }
    Subclassification,
    
    // === KerML Annotations ===
    /// comment_annotation = { comment ~ ... }
    CommentAnnotation,
    /// documentation = { doc ~ ... }
    Documentation,
    /// metadata_feature = { metadata ~ ... }
    MetadataFeature,
    
    // === KerML Parameters ===
    /// parameter_membership = { direction? ~ feature ... }
    ParameterMembership,
    /// return_parameter_membership = { return ~ ... }
    ReturnParameterMembership,
    
    // === KerML Expressions ===
    /// operator_expression = { ... }
    OperatorExpression,
    /// literal_expression = { ... }
    LiteralExpression,
    /// invocation_expression = { ... }
    InvocationExpression,
    /// feature_chain_expression = { ... }
    FeatureChainExpression,
    /// conditional_expression = { if ... ? ... else ... }
    ConditionalExpression,
    
    // === KerML Fragments ===
    /// qualified_reference_chain = { name (:: name)* }
    QualifiedReferenceChain,
    /// identification = { short_name? name? }
    Identification,
    /// visibility = { public | private | protected }
    Visibility,
    /// namespace_body = { { ... } }
    NamespaceBody,
    /// namespace_body_element = { ... }
    NamespaceBodyElement,
    /// namespace_body_elements = { namespace_body_element+ }
    NamespaceBodyElements,
    
    // === SysML Definitions ===
    /// part_def = { part def ~ ... }
    PartDef,
    /// attribute_def = { attribute def ~ ... }
    AttributeDef,
    /// item_def = { item def ~ ... }
    ItemDef,
    /// port_def = { port def ~ ... }
    PortDef,
    /// action_def = { action def ~ ... }
    ActionDef,
    /// state_def = { state def ~ ... }
    StateDef,
    /// calc_def = { calc def ~ ... }
    CalcDef,
    /// constraint_def = { constraint def ~ ... }
    ConstraintDef,
    /// requirement_def = { requirement def ~ ... }
    RequirementDef,
    /// connection_def = { connection def ~ ... }
    ConnectionDef,
    /// interface_def = { interface def ~ ... }
    InterfaceDef,
    /// allocation_def = { allocation def ~ ... }
    AllocationDef,
    
    // === SysML Usages ===
    /// part_usage = { part ~ ... }
    PartUsage,
    /// attribute_usage = { attribute ~ ... }
    AttributeUsage,
    /// item_usage = { item ~ ... }
    ItemUsage,
    /// port_usage = { port ~ ... }
    PortUsage,
    /// action_usage = { action ~ ... }
    ActionUsage,
    /// state_usage = { state ~ ... }
    StateUsage,
    /// calc_usage = { calc ~ ... }
    CalcUsage,
    /// constraint_usage = { constraint ~ ... }
    ConstraintUsage,
    /// requirement_usage = { requirement ~ ... }
    RequirementUsage,
    /// connection_usage = { connection ~ ... }
    ConnectionUsage,
    /// interface_usage = { interface ~ ... }
    InterfaceUsage,
    /// allocation_usage = { allocation ~ ... }
    AllocationUsage,
    
    // === SysML Action Body Elements ===
    /// perform_action_usage = { perform ~ ... }
    PerformActionUsage,
    /// send_action_usage = { send ~ ... }
    SendActionUsage,
    /// accept_action_usage = { accept ~ ... }
    AcceptActionUsage,
    /// assign_action_usage = { assign ~ ... | :=  }
    AssignActionUsage,
    /// if_action_usage = { if ~ ... }
    IfActionUsage,
    /// while_loop_action_usage = { while ~ ... }
    WhileLoopActionUsage,
    /// for_loop_action_usage = { for ~ ... }
    ForLoopActionUsage,
    
    // === SysML Connectors ===
    /// binding_connector_as_usage = { bind ~ ... = ... } (SysML shorthand)
    BindingConnectorAsUsage,
    /// succession_as_usage = { first ... then ... } (SysML shorthand)
    SuccessionAsUsage,
    
    // === Primitives ===
    /// regular_name = identifier or unrestricted name
    RegularName,
    /// unrestricted_name = 'quoted name'
    UnrestrictedName,
    /// short_name = <name>
    ShortName,
    /// literal_number = integer or decimal
    LiteralNumber,
    
    // === Full File ===
    /// KerML file
    KerMLFile,
    /// SysML file  
    SysMLFile,
}

/// Parse result for a single rule
#[derive(Debug)]
pub struct RuleParseResult {
    /// The parse result with green tree and errors
    pub parse: Parse,
    /// The rule that was parsed
    pub rule: Rule,
    /// The original input
    pub input: String,
}

impl RuleParseResult {
    /// Check if parsing succeeded without errors
    pub fn is_ok(&self) -> bool {
        self.parse.ok()
    }
    
    /// Get the errors from parsing
    pub fn errors(&self) -> &[super::parser::SyntaxError] {
        &self.parse.errors
    }
    
    /// Get the syntax tree root
    pub fn syntax(&self) -> super::SyntaxNode {
        self.parse.syntax()
    }
    
    /// Check if the entire input was consumed
    pub fn fully_consumed(&self) -> bool {
        // The tree should contain all the input text
        let root = self.parse.syntax();
        let tree_text = root.text().to_string();
        tree_text.trim() == self.input.trim()
    }
}

/// Parse a specific grammar rule
///
/// This provides a Pest-like API where you can parse just a specific construct
/// without wrapping it in a full file context.
///
/// # Arguments
///
/// * `rule` - The grammar rule to parse
/// * `input` - The input text to parse
///
/// # Returns
///
/// A `RuleParseResult` containing the parse tree and any errors.
///
/// # Example
///
/// ```ignore
/// let result = parse_rule(Rule::ItemFlow, "flow myFlow from a to b;");
/// assert!(result.is_ok());
/// ```
pub fn parse_rule(rule: Rule, input: &str) -> RuleParseResult {
    // Wrap input in appropriate context based on rule
    let (wrapped_input, mode) = wrap_for_rule(rule, input);
    
    let tokens: Vec<_> = Lexer::new(&wrapped_input).collect();
    let mut parser = Parser::new(&tokens, mode);
    
    // Parse the wrapped input
    match mode {
        LanguageMode::KerML => kerml::parse_kerml_file(&mut parser),
        LanguageMode::SysML => sysml::parse_sysml_file(&mut parser),
    }
    
    RuleParseResult {
        parse: parser.finish(),
        rule,
        input: input.to_string(),
    }
}

/// Wrap input in appropriate context for the given rule
fn wrap_for_rule(rule: Rule, input: &str) -> (String, LanguageMode) {
    match rule {
        // Full file rules - no wrapping needed
        Rule::KerMLFile => (input.to_string(), LanguageMode::KerML),
        Rule::SysMLFile => (input.to_string(), LanguageMode::SysML),
        
        // Package-level KerML rules
        Rule::Package | Rule::LibraryPackage | Rule::Namespace => {
            (input.to_string(), LanguageMode::KerML)
        }
        
        // Import needs package context
        Rule::Import | Rule::Dependency => {
            (format!("package __Test__ {{ {} }}", input), LanguageMode::KerML)
        }
        
        // KerML definitions - wrap in package
        Rule::Class | Rule::DataType | Rule::Structure | Rule::Association |
        Rule::AssociationStructure | Rule::Behavior | Rule::Function |
        Rule::Predicate | Rule::Interaction | Rule::Metaclass |
        Rule::Classifier | Rule::TypeDef => {
            (format!("package __Test__ {{ {} }}", input), LanguageMode::KerML)
        }
        
        // KerML features/usages - wrap in class
        Rule::Feature | Rule::Step | Rule::Expression | Rule::BooleanExpression |
        Rule::Invariant | Rule::Multiplicity | Rule::MultiplicityRange => {
            (format!("class __Test__ {{ {} }}", input), LanguageMode::KerML)
        }
        
        // KerML connectors/flows - wrap in class
        Rule::Connector | Rule::BindingConnector | Rule::Succession |
        Rule::ItemFlow | Rule::SuccessionItemFlow => {
            (format!("class __Test__ {{ {} }}", input), LanguageMode::KerML)
        }
        
        // KerML relationships that are inline in features - test in feature context
        Rule::Specialization | Rule::Subsetting | Rule::Redefinition |
        Rule::FeatureTyping | Rule::Conjugation | Rule::FeatureChaining => {
            // These are typically part of declarations, test in class context
            (format!("class __Test__ {{ feature x {} ; }}", input), LanguageMode::KerML)
        }
        
        // Disjoining can be inline (disjoint Type) or standalone (disjoint X from Y)
        // Standalone form with 'from' needs package context
        Rule::Disjoining => {
            if input.contains("from") {
                // Standalone disjoining with from keyword - input already has semicolon
                (format!("package __Test__ {{ {} }}", input), LanguageMode::KerML)
            } else {
                // Inline disjoining in feature
                (format!("class __Test__ {{ feature x {} ; }}", input), LanguageMode::KerML)
            }
        }
        
        // Subclassification is a standalone relationship declaration
        Rule::Subclassification => {
            (format!("package __Test__ {{ {} }}", input), LanguageMode::KerML)
        }
        
        Rule::FeatureInverting => {
            (format!("package __Test__ {{ {} }}", input), LanguageMode::KerML)
        }
        
        // KerML annotations - wrap in package
        Rule::CommentAnnotation | Rule::Documentation | Rule::MetadataFeature => {
            (format!("package __Test__ {{ {} }}", input), LanguageMode::KerML)
        }
        
        // KerML parameters - wrap in function
        Rule::ParameterMembership | Rule::ReturnParameterMembership => {
            (format!("function __Test__ {{ {} }}", input), LanguageMode::KerML)
        }
        
        // KerML expressions - wrap in feature value
        Rule::OperatorExpression | Rule::LiteralExpression |
        Rule::InvocationExpression | Rule::FeatureChainExpression |
        Rule::ConditionalExpression => {
            (format!("class __Test__ {{ feature x = {}; }}", input), LanguageMode::KerML)
        }
        
        // KerML fragments
        Rule::QualifiedReferenceChain => {
            (format!("import {};", input), LanguageMode::KerML)
        }
        Rule::Identification => {
            (format!("class {} {{}}", input), LanguageMode::KerML)
        }
        Rule::Visibility => {
            (format!("{} class X {{}}", input), LanguageMode::KerML)
        }
        Rule::NamespaceBody => {
            (format!("class __Test__ {}", input), LanguageMode::KerML)
        }
        Rule::NamespaceBodyElement | Rule::NamespaceBodyElements => {
            // Wrap element in braces since body elements need to be inside a body
            (format!("class __Test__ {{ {} }}", input), LanguageMode::KerML)
        }
        
        // SysML definitions
        Rule::PartDef | Rule::AttributeDef | Rule::ItemDef |
        Rule::PortDef | Rule::ActionDef | Rule::StateDef |
        Rule::CalcDef | Rule::ConstraintDef | Rule::RequirementDef |
        Rule::ConnectionDef | Rule::InterfaceDef | Rule::AllocationDef => {
            (format!("package __Test__ {{ {} }}", input), LanguageMode::SysML)
        }
        
        // SysML usages
        Rule::PartUsage | Rule::AttributeUsage | Rule::ItemUsage |
        Rule::PortUsage | Rule::ActionUsage | Rule::StateUsage |
        Rule::CalcUsage | Rule::ConstraintUsage | Rule::RequirementUsage |
        Rule::ConnectionUsage | Rule::InterfaceUsage | Rule::AllocationUsage => {
            (format!("part def __Test__ {{ {} }}", input), LanguageMode::SysML)
        }
        
        // SysML action body elements
        Rule::PerformActionUsage | Rule::SendActionUsage |
        Rule::AcceptActionUsage | Rule::AssignActionUsage |
        Rule::IfActionUsage | Rule::WhileLoopActionUsage |
        Rule::ForLoopActionUsage => {
            (format!("action def __Test__ {{ {} }}", input), LanguageMode::SysML)
        }
        
        // SysML connectors (bind/succession shorthand)
        Rule::BindingConnectorAsUsage | Rule::SuccessionAsUsage => {
            (format!("part def __Test__ {{ {} }}", input), LanguageMode::SysML)
        }
        
        // Primitives - test in appropriate context
        Rule::RegularName | Rule::UnrestrictedName => {
            (format!("class {} {{}}", input), LanguageMode::KerML)
        }
        Rule::ShortName => {
            (format!("class {} X {{}}", input), LanguageMode::KerML)
        }
        Rule::LiteralNumber => {
            (format!("class __Test__ {{ feature x = {}; }}", input), LanguageMode::KerML)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_item_flow() {
        let result = parse_rule(Rule::ItemFlow, "flow myFlow from a to b;");
        assert!(result.is_ok(), "Failed to parse item_flow: {:?}", result.errors());
    }
    
    #[test]
    fn test_parse_connector() {
        let result = parse_rule(Rule::Connector, "connector from x to y;");
        assert!(result.is_ok(), "Failed to parse connector: {:?}", result.errors());
    }
    
    #[test]
    fn test_parse_class() {
        let result = parse_rule(Rule::Class, "class MyClass specializes Base;");
        assert!(result.is_ok(), "Failed to parse class: {:?}", result.errors());
    }
    
    #[test]
    fn test_parse_feature() {
        let result = parse_rule(Rule::Feature, "feature x : Integer[1];");
        assert!(result.is_ok(), "Failed to parse feature: {:?}", result.errors());
    }
    
    #[test]
    fn test_parse_operator_expression() {
        let result = parse_rule(Rule::OperatorExpression, "a + b * c");
        assert!(result.is_ok(), "Failed to parse expression: {:?}", result.errors());
    }
    
    #[test]
    fn test_parse_succession() {
        let result = parse_rule(Rule::Succession, "succession a then b;");
        assert!(result.is_ok(), "Failed to parse succession: {:?}", result.errors());
    }
}
