//! Lexer for SysML/KerML using Logos
//!
//! Tokenizes source code into tokens including whitespace and comments.

use crate::parser::SyntaxKind;
use logos::Logos;

/// Token with text and kind
pub struct Token<'a> {
    pub kind: SyntaxKind,
    pub text: &'a str,
}

/// Logos-based token types
#[derive(Logos, Debug, Clone, Copy, PartialEq)]
pub enum LogosToken {
    // === Trivia ===
    #[regex(r"[ \t\r\n]+")]
    Whitespace,

    #[regex(r"//[^\n]*", priority = 2)]
    LineComment,

    #[regex(r"/\*([^*]|\*[^/])*\*/")]
    BlockComment,

    // === Keywords (longest first for proper matching) ===
    #[token("specializes")]
    SpecializesKw,
    #[token("verification")]
    VerificationKw,
    #[token("requirement")]
    RequirementKw,
    #[token("connection")]
    ConnectionKw,
    #[token("allocation")]
    AllocationKw,
    #[token("constraint")]
    ConstraintKw,
    #[token("occurrence")]
    OccurrenceKw,
    #[token("individual")]
    IndividualKw,
    #[token("viewpoint")]
    ViewpointKw,
    #[token("rendering")]
    RenderingKw,
    #[token("interface")]
    InterfaceKw,
    #[token("redefines")]
    RedefinesKw,
    #[token("references")]
    ReferencesKw,
    #[token("metadata")]
    MetadataKw,
    #[token("analysis")]
    AnalysisKw,
    #[token("attribute")]
    AttributeKw,
    #[token("abstract")]
    AbstractKw,
    #[token("const")]
    ConstKw,
    #[token("behavior")]
    BehaviorKw,
    #[token("function")]
    FunctionKw,
    #[token("datatype")]
    DataTypeKw,
    #[token("language")]
    LanguageKw,
    #[token("standard")]
    StandardKw,
    #[token("protected")]
    ProtectedKw,
    #[token("nonunique")]
    NonuniqueKw,
    #[token("unordered")]
    UnorderedKw,
    #[token("succession")]
    SuccessionKw,
    #[token("connector")]
    ConnectorKw,
    #[token("use case")]
    UseCaseKw,
    #[token("typed by")]
    TypedByKw,
    #[token("has type")]
    HasTypeKw,
    #[token("is type")]
    IsTypeKw,
    #[token("package")]
    PackageKw,
    #[token("private")]
    PrivateKw,
    #[token("concern")]
    ConcernKw,
    #[token("binding")]
    BindingKw,
    #[token("ordered")]
    OrderedKw,
    #[token("implies")]
    ImpliesKw,
    #[token("satisfy")]
    SatisfyKw,
    #[token("library")]
    LibraryKw,
    #[token("include")]
    IncludeKw,
    #[token("exhibit")]
    ExhibitKw,
    #[token("perform")]
    PerformKw,
    #[token("require")]
    RequireKw,
    #[token("subsets")]
    SubsetsKw,
    #[token("derived")]
    DerivedKw,
    #[token("feature")]
    FeatureKw,
    #[token("comment")]
    CommentKw,
    #[token("struct")]
    StructKw,
    #[token("public")]
    PublicKw,
    #[token("expose")]
    ExposeKw,
    #[token("filter")]
    FilterKw,
    #[token("decide")]
    DecideKw,
    #[token("accept")]
    AcceptKw,
    #[token("import")]
    ImportKw,
    #[token("action")]
    ActionKw,
    #[token("assert")]
    AssertKw,
    #[token("assume")]
    AssumeKw,
    #[token("class")]
    ClassKw,
    #[token("assoc")]
    AssocKw,
    #[token("state")]
    StateKw,
    #[token("about")]
    AboutKw,
    #[token("entry")]
    EntryKw,
    #[token("until")]
    UntilKw,
    #[token("while")]
    WhileKw,
    #[token("model")]
    ModelKw,
    #[token("first")]
    FirstKw,
    #[token("alias")]
    AliasKw,
    #[token("inout")]
    InoutKw,
    #[token("false")]
    FalseKw,
    #[token("merge")]
    MergeKw,
    #[token("part")]
    PartKw,
    #[token("port")]
    PortKw,
    #[token("item")]
    ItemKw,
    #[token("view")]
    ViewKw,
    #[token("flow")]
    FlowKw,
    #[token("enum")]
    EnumKw,
    #[token("calc")]
    CalcKw,
    #[token("case")]
    CaseKw,
    #[token("type")]
    TypeKw,
    #[token("step")]
    StepKw,
    #[token("expr")]
    ExprKw,
    #[token("else")]
    ElseKw,
    #[token("then")]
    ThenKw,
    #[token("loop")]
    LoopKw,
    #[token("fork")]
    ForkKw,
    #[token("join")]
    JoinKw,
    #[token("send")]
    SendKw,
    #[token("from")]
    FromKw,
    #[token("null")]
    NullKw,
    #[token("true")]
    TrueKw,
    #[token("meta")]
    MetaKw,
    #[token("exit")]
    ExitKw,
    #[token("def")]
    DefKw,
    #[token("ref")]
    RefKw,
    #[token("doc")]
    DocKw,
    #[token("rep")]
    RepKw,
    #[token("inv")]
    InvKw,
    #[token("end")]
    EndKw,
    #[token("out")]
    OutKw,
    #[token("for")]
    ForKw,
    #[token("via")]
    ViaKw,
    #[token("all")]
    AllKw,
    #[token("and")]
    AndKw,
    #[token("xor")]
    XorKw,
    #[token("not")]
    NotKw,
    #[token("if")]
    IfKw,
    #[token("in")]
    InKw,
    #[token("do")]
    DoKw,
    #[token("to")]
    ToKw,
    #[token("or")]
    OrKw,
    #[token("as")]
    AsKw,
    #[token("dependency")]
    DependencyKw,

    // === Punctuation ===
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token(";")]
    Semicolon,
    #[token("::")]
    ColonColon,
    #[token(":")]
    Colon,
    #[token("..")]
    DotDot,
    #[token(".")]
    Dot,
    #[token(",")]
    Comma,
    #[token("==")]
    EqEq,
    #[token("!=")]
    NotEq,
    #[token("=")]
    Eq,
    #[token("<=")]
    LtEq,
    #[token(">=")]
    GtEq,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("->")]
    Arrow,
    #[token(":>>")]
    RedefinesToken,
    #[token(":>")]
    SubsetsToken,
    #[token("@")]
    At,
    #[token("**")]
    StarStar,
    #[token("*")]
    Star,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("^")]
    Caret,
    #[token("~")]
    Tilde,
    #[token("??")]
    QuestionQuestion,
    #[token("?")]
    Question,
    #[token("!")]
    Bang,
    #[token("||")]
    PipePipe,
    #[token("|")]
    Pipe,
    #[token("&&")]
    AmpAmp,
    #[token("&")]
    Ampersand,
    #[token("#")]
    Hash,

    // === Literals ===
    #[regex(r"[0-9]+(\.[0-9]+)?([eE][+-]?[0-9]+)?")]
    Number,

    #[regex(r#""([^"\\]|\\.)*""#)]
    #[regex(r#"'([^'\\]|\\.)*'"#)]
    String,

    // === Identifiers (must come after keywords) ===
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    Identifier,
}

/// Convert Logos token to the canonical parser SyntaxKind
fn to_syntax_kind(token: LogosToken) -> SyntaxKind {
    match token {
        LogosToken::Whitespace => SyntaxKind::WHITESPACE,
        LogosToken::LineComment => SyntaxKind::LINE_COMMENT,
        LogosToken::BlockComment => SyntaxKind::BLOCK_COMMENT,
        LogosToken::PackageKw => SyntaxKind::PACKAGE_KW,
        LogosToken::PartKw => SyntaxKind::PART_KW,
        LogosToken::DefKw => SyntaxKind::DEF_KW,
        LogosToken::ImportKw => SyntaxKind::IMPORT_KW,
        LogosToken::AttributeKw => SyntaxKind::ATTRIBUTE_KW,
        LogosToken::PortKw => SyntaxKind::PORT_KW,
        LogosToken::ItemKw => SyntaxKind::ITEM_KW,
        LogosToken::ActionKw => SyntaxKind::ACTION_KW,
        LogosToken::StateKw => SyntaxKind::STATE_KW,
        LogosToken::RequirementKw => SyntaxKind::REQUIREMENT_KW,
        LogosToken::ConstraintKw => SyntaxKind::CONSTRAINT_KW,
        LogosToken::ConnectionKw => SyntaxKind::CONNECTION_KW,
        LogosToken::AllocationKw => SyntaxKind::ALLOCATION_KW,
        LogosToken::InterfaceKw => SyntaxKind::INTERFACE_KW,
        LogosToken::FlowKw => SyntaxKind::FLOW_KW,
        // "use case" is lexed as a single token; map to USE_KW (text preserved)
        LogosToken::UseCaseKw => SyntaxKind::USE_KW,
        LogosToken::ViewKw => SyntaxKind::VIEW_KW,
        LogosToken::ViewpointKw => SyntaxKind::VIEWPOINT_KW,
        LogosToken::RenderingKw => SyntaxKind::RENDERING_KW,
        LogosToken::MetadataKw => SyntaxKind::METADATA_KW,
        LogosToken::OccurrenceKw => SyntaxKind::OCCURRENCE_KW,
        LogosToken::AnalysisKw => SyntaxKind::ANALYSIS_KW,
        LogosToken::VerificationKw => SyntaxKind::VERIFICATION_KW,
        LogosToken::ConcernKw => SyntaxKind::CONCERN_KW,
        LogosToken::EnumKw => SyntaxKind::ENUM_KW,
        LogosToken::CalcKw => SyntaxKind::CALC_KW,
        LogosToken::CaseKw => SyntaxKind::CASE_KW,
        LogosToken::IndividualKw => SyntaxKind::INDIVIDUAL_KW,
        LogosToken::AbstractKw => SyntaxKind::ABSTRACT_KW,
        LogosToken::RefKw => SyntaxKind::REF_KW,
        LogosToken::ConstKw => SyntaxKind::CONST_KW,
        LogosToken::DerivedKw => SyntaxKind::DERIVED_KW,
        LogosToken::EndKw => SyntaxKind::END_KW,
        LogosToken::InKw => SyntaxKind::IN_KW,
        LogosToken::OutKw => SyntaxKind::OUT_KW,
        LogosToken::InoutKw => SyntaxKind::INOUT_KW,
        LogosToken::AliasKw => SyntaxKind::ALIAS_KW,
        LogosToken::DocKw => SyntaxKind::DOC_KW,
        LogosToken::CommentKw => SyntaxKind::COMMENT_KW,
        LogosToken::AboutKw => SyntaxKind::ABOUT_KW,
        LogosToken::RepKw => SyntaxKind::REP_KW,
        LogosToken::LanguageKw => SyntaxKind::LANGUAGE_KW,
        LogosToken::SpecializesKw => SyntaxKind::SPECIALIZES_KW,
        LogosToken::SubsetsKw => SyntaxKind::SUBSETS_KW,
        LogosToken::RedefinesKw => SyntaxKind::REDEFINES_KW,
        // "typed by" is lexed as a single token; map to TYPED_KW (text preserved)
        LogosToken::TypedByKw => SyntaxKind::TYPED_KW,
        LogosToken::ReferencesKw => SyntaxKind::REFERENCES_KW,
        LogosToken::AssertKw => SyntaxKind::ASSERT_KW,
        LogosToken::AssumeKw => SyntaxKind::ASSUME_KW,
        LogosToken::RequireKw => SyntaxKind::REQUIRE_KW,
        LogosToken::PerformKw => SyntaxKind::PERFORM_KW,
        LogosToken::ExhibitKw => SyntaxKind::EXHIBIT_KW,
        LogosToken::IncludeKw => SyntaxKind::INCLUDE_KW,
        LogosToken::SatisfyKw => SyntaxKind::SATISFY_KW,
        LogosToken::EntryKw => SyntaxKind::ENTRY_KW,
        LogosToken::ExitKw => SyntaxKind::EXIT_KW,
        LogosToken::DoKw => SyntaxKind::DO_KW,
        LogosToken::IfKw => SyntaxKind::IF_KW,
        LogosToken::ElseKw => SyntaxKind::ELSE_KW,
        LogosToken::ThenKw => SyntaxKind::THEN_KW,
        LogosToken::LoopKw => SyntaxKind::LOOP_KW,
        LogosToken::WhileKw => SyntaxKind::WHILE_KW,
        LogosToken::UntilKw => SyntaxKind::UNTIL_KW,
        LogosToken::ForKw => SyntaxKind::FOR_KW,
        LogosToken::ForkKw => SyntaxKind::FORK_KW,
        LogosToken::JoinKw => SyntaxKind::JOIN_KW,
        LogosToken::MergeKw => SyntaxKind::MERGE_KW,
        LogosToken::DecideKw => SyntaxKind::DECIDE_KW,
        LogosToken::AcceptKw => SyntaxKind::ACCEPT_KW,
        LogosToken::SendKw => SyntaxKind::SEND_KW,
        LogosToken::ViaKw => SyntaxKind::VIA_KW,
        LogosToken::ToKw => SyntaxKind::TO_KW,
        LogosToken::FromKw => SyntaxKind::FROM_KW,
        LogosToken::DependencyKw => SyntaxKind::DEPENDENCY_KW,
        LogosToken::FilterKw => SyntaxKind::FILTER_KW,
        LogosToken::ExposeKw => SyntaxKind::EXPOSE_KW,
        LogosToken::AllKw => SyntaxKind::ALL_KW,
        LogosToken::FirstKw => SyntaxKind::FIRST_KW,
        LogosToken::ModelKw => SyntaxKind::IDENT, // not a parser keyword
        LogosToken::LibraryKw => SyntaxKind::LIBRARY_KW,
        LogosToken::StandardKw => SyntaxKind::STANDARD_KW,
        LogosToken::PrivateKw => SyntaxKind::PRIVATE_KW,
        LogosToken::ProtectedKw => SyntaxKind::PROTECTED_KW,
        LogosToken::PublicKw => SyntaxKind::PUBLIC_KW,
        LogosToken::TrueKw => SyntaxKind::TRUE_KW,
        LogosToken::FalseKw => SyntaxKind::FALSE_KW,
        LogosToken::NullKw => SyntaxKind::NULL_KW,
        LogosToken::AndKw => SyntaxKind::AND_KW,
        LogosToken::OrKw => SyntaxKind::OR_KW,
        LogosToken::NotKw => SyntaxKind::NOT_KW,
        LogosToken::XorKw => SyntaxKind::XOR_KW,
        LogosToken::ImpliesKw => SyntaxKind::IMPLIES_KW,
        // "has type" / "is type" are lexed as single tokens
        LogosToken::HasTypeKw => SyntaxKind::HASTYPE_KW,
        LogosToken::IsTypeKw => SyntaxKind::ISTYPE_KW,
        LogosToken::AsKw => SyntaxKind::AS_KW,
        LogosToken::MetaKw => SyntaxKind::META_KW,
        LogosToken::StructKw => SyntaxKind::STRUCT_KW,
        LogosToken::ClassKw => SyntaxKind::CLASS_KW,
        LogosToken::DataTypeKw => SyntaxKind::DATATYPE_KW,
        LogosToken::AssocKw => SyntaxKind::ASSOC_KW,
        LogosToken::BehaviorKw => SyntaxKind::BEHAVIOR_KW,
        LogosToken::FunctionKw => SyntaxKind::FUNCTION_KW,
        LogosToken::TypeKw => SyntaxKind::TYPE_KW,
        LogosToken::FeatureKw => SyntaxKind::FEATURE_KW,
        LogosToken::StepKw => SyntaxKind::STEP_KW,
        LogosToken::ExprKw => SyntaxKind::EXPR_KW,
        LogosToken::BindingKw => SyntaxKind::BINDING_KW,
        LogosToken::SuccessionKw => SyntaxKind::SUCCESSION_KW,
        LogosToken::ConnectorKw => SyntaxKind::CONNECTOR_KW,
        LogosToken::InvKw => SyntaxKind::INV_KW,
        LogosToken::NonuniqueKw => SyntaxKind::NONUNIQUE_KW,
        LogosToken::OrderedKw => SyntaxKind::ORDERED_KW,
        LogosToken::UnorderedKw => SyntaxKind::IDENT, // not a parser keyword
        // Punctuation
        LogosToken::LBrace => SyntaxKind::L_BRACE,
        LogosToken::RBrace => SyntaxKind::R_BRACE,
        LogosToken::LBracket => SyntaxKind::L_BRACKET,
        LogosToken::RBracket => SyntaxKind::R_BRACKET,
        LogosToken::LParen => SyntaxKind::L_PAREN,
        LogosToken::RParen => SyntaxKind::R_PAREN,
        LogosToken::Semicolon => SyntaxKind::SEMICOLON,
        LogosToken::Colon => SyntaxKind::COLON,
        LogosToken::ColonColon => SyntaxKind::COLON_COLON,
        LogosToken::Dot => SyntaxKind::DOT,
        LogosToken::DotDot => SyntaxKind::DOT_DOT,
        LogosToken::Comma => SyntaxKind::COMMA,
        LogosToken::Eq => SyntaxKind::EQ,
        LogosToken::EqEq => SyntaxKind::EQ_EQ,
        LogosToken::NotEq => SyntaxKind::BANG_EQ,
        LogosToken::Lt => SyntaxKind::LT,
        LogosToken::Gt => SyntaxKind::GT,
        LogosToken::LtEq => SyntaxKind::LT_EQ,
        LogosToken::GtEq => SyntaxKind::GT_EQ,
        LogosToken::Arrow => SyntaxKind::ARROW,
        LogosToken::SubsetsToken => SyntaxKind::COLON_GT,
        LogosToken::RedefinesToken => SyntaxKind::COLON_GT_GT,
        LogosToken::At => SyntaxKind::AT,
        LogosToken::Star => SyntaxKind::STAR,
        LogosToken::StarStar => SyntaxKind::STAR_STAR,
        LogosToken::Plus => SyntaxKind::PLUS,
        LogosToken::Minus => SyntaxKind::MINUS,
        LogosToken::Slash => SyntaxKind::SLASH,
        LogosToken::Percent => SyntaxKind::PERCENT,
        LogosToken::Caret => SyntaxKind::CARET,
        LogosToken::Tilde => SyntaxKind::TILDE,
        LogosToken::Question => SyntaxKind::QUESTION,
        LogosToken::QuestionQuestion => SyntaxKind::QUESTION_QUESTION,
        LogosToken::Bang => SyntaxKind::BANG,
        LogosToken::Pipe => SyntaxKind::PIPE,
        LogosToken::PipePipe => SyntaxKind::PIPE_PIPE,
        LogosToken::Ampersand => SyntaxKind::AMP,
        LogosToken::AmpAmp => SyntaxKind::AMP_AMP,
        LogosToken::Hash => SyntaxKind::HASH,
        // Literals
        LogosToken::Number => SyntaxKind::INTEGER,
        LogosToken::String => SyntaxKind::STRING,
        LogosToken::Identifier => SyntaxKind::IDENT,
    }
}

/// Tokenize source code into a vector of tokens with their text
pub fn tokenize(source: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    let mut lexer = LogosToken::lexer(source);

    while let Some(result) = lexer.next() {
        let text = lexer.slice();
        let kind = match result {
            Ok(token) => to_syntax_kind(token),
            Err(()) => SyntaxKind::ERROR,
        };
        tokens.push(Token { kind, text });
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_simple() {
        let tokens = tokenize("package Test { }");
        assert!(tokens.iter().any(|t| t.kind == SyntaxKind::PACKAGE_KW));
        assert!(tokens.iter().any(|t| t.kind == SyntaxKind::IDENT));
        assert!(tokens.iter().any(|t| t.kind == SyntaxKind::L_BRACE));
        assert!(tokens.iter().any(|t| t.kind == SyntaxKind::R_BRACE));
    }

    #[test]
    fn test_tokenize_with_comments() {
        let tokens = tokenize("// comment\npackage Test;");
        assert!(tokens.iter().any(|t| t.kind == SyntaxKind::LINE_COMMENT));
        assert!(tokens.iter().any(|t| t.kind == SyntaxKind::PACKAGE_KW));
    }

    #[test]
    fn test_tokenize_block_comment() {
        let tokens = tokenize("/* block */ package Test;");
        assert!(tokens.iter().any(|t| t.kind == SyntaxKind::BLOCK_COMMENT));
    }

    #[test]
    fn test_whitespace_preserved() {
        let tokens = tokenize("package   Test");
        assert!(tokens.iter().any(|t| t.kind == SyntaxKind::WHITESPACE));
    }
}
