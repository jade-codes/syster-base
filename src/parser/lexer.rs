//! Logos-based lexer for SysML v2
//!
//! Fast tokenization using the logos crate.

use super::syntax_kind::SyntaxKind;
use logos::Logos;
use rowan::TextSize;

/// A token with its kind, text, and position
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token<'a> {
    pub kind: SyntaxKind,
    pub text: &'a str,
    pub offset: TextSize,
}

/// Lexer wrapping the logos-generated tokenizer
pub struct Lexer<'a> {
    inner: logos::Lexer<'a, LogosToken>,
    offset: u32,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            inner: LogosToken::lexer(input),
            offset: 0,
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let logos_token = self.inner.next()?;
        let text = self.inner.slice();
        let offset = TextSize::new(self.offset);
        self.offset += text.len() as u32;

        let kind = match logos_token {
            Ok(t) => t.into(),
            Err(()) => SyntaxKind::ERROR,
        };

        Some(Token { kind, text, offset })
    }
}

/// Tokenize an entire string into a Vec
#[allow(dead_code)]
pub fn tokenize(input: &str) -> Vec<Token<'_>> {
    Lexer::new(input).collect()
}

/// Logos token enum - maps to SyntaxKind
#[derive(Logos, Debug, Clone, Copy, PartialEq)]
#[logos(skip r"")] // Don't skip anything, we want all tokens
pub enum LogosToken {
    // =========================================================================
    // TRIVIA
    // =========================================================================
    #[regex(r"[ \t\r\n]+")]
    Whitespace,

    #[regex(r"//[^\n]*")]
    LineComment,

    #[regex(r"/\*([^*]|\*[^/])*\*/")]
    BlockComment,

    // =========================================================================
    // LITERALS
    // =========================================================================
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    Ident,

    #[regex(r"'[^']*'")]
    UnrestrictedName, // Short name like '<name>' - actually uses < > in SysML

    #[regex(r"[0-9]+")]
    Integer,

    #[regex(r"[0-9]*\.[0-9]+([eE][+-]?[0-9]+)?")]
    Decimal,

    #[regex(r#""([^"\\]|\\.)*""#)]
    String,

    // =========================================================================
    // MULTI-CHARACTER PUNCTUATION (must come before single-char)
    // =========================================================================
    #[token("::>")]
    ColonColonGt,

    #[token(":>>")]
    ColonGtGt,

    #[token(":>")]
    ColonGt,

    #[token("::")]
    ColonColon,

    #[token(":=")]
    ColonEq,

    #[token("..")]
    DotDot,

    #[token("===")]
    EqEqEq,

    #[token("!==")]
    BangEqEq,

    #[token("==")]
    EqEq,

    #[token("!=")]
    BangEq,

    #[token("<=")]
    LtEq,

    #[token(">=")]
    GtEq,

    #[token("->")]
    Arrow,

    #[token("=>")]
    FatArrow,

    #[token("@@")]
    AtAt,

    #[token("**")]
    StarStar,

    #[token("??")]
    QuestionQuestion,

    #[token("&&")]
    AmpAmp,

    #[token("||")]
    PipePipe,

    // =========================================================================
    // SINGLE-CHARACTER PUNCTUATION
    // =========================================================================
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
    #[token(":")]
    Colon,
    #[token(".")]
    Dot,
    #[token(",")]
    Comma,
    #[token("=")]
    Eq,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("@")]
    At,
    #[token("#")]
    Hash,
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
    #[token("?")]
    Question,
    #[token("!")]
    Bang,
    #[token("|")]
    Pipe,
    #[token("&")]
    Amp,

    // =========================================================================
    // KEYWORDS (alphabetical, longest match wins in logos)
    // =========================================================================
    #[token("about")]
    AboutKw,
    #[token("abstract")]
    AbstractKw,
    #[token("accept")]
    AcceptKw,
    #[token("action")]
    ActionKw,
    #[token("actor")]
    ActorKw,
    #[token("after")]
    AfterKw,
    #[token("alias")]
    AliasKw,
    #[token("all")]
    AllKw,
    #[token("allocation")]
    AllocationKw,
    #[token("allocate")]
    AllocateKw,
    #[token("analysis")]
    AnalysisKw,
    #[token("and")]
    AndKw,
    #[token("as")]
    AsKw,
    #[token("assert")]
    AssertKw,
    #[token("assign")]
    AssignKw,
    #[token("assoc")]
    AssocKw,
    #[token("assume")]
    AssumeKw,
    #[token("at")]
    AtKw,
    #[token("attribute")]
    AttributeKw,
    #[token("behavior")]
    BehaviorKw,
    #[token("bind")]
    BindKw,
    #[token("binding")]
    BindingKw,
    #[token("by")]
    ByKw,
    #[token("calc")]
    CalcKw,
    #[token("case")]
    CaseKw,
    #[token("class")]
    ClassKw,
    #[token("classifier")]
    ClassifierKw,
    #[token("comment")]
    CommentKw,
    #[token("composite")]
    CompositeKw,
    #[token("concern")]
    ConcernKw,
    #[token("connect")]
    ConnectKw,
    #[token("connection")]
    ConnectionKw,
    #[token("connector")]
    ConnectorKw,
    #[token("constant")]
    ConstantKw,
    #[token("constraint")]
    ConstraintKw,
    #[token("conjugates")]
    ConjugatesKw,
    #[token("crosses")]
    CrossesKw,
    #[token("datatype")]
    DatatypeKw,
    #[token("decide")]
    DecideKw,
    #[token("def")]
    DefKw,
    #[token("default")]
    DefaultKw,
    #[token("defined")]
    DefinedKw,
    #[token("dependency")]
    DependencyKw,
    #[token("derived")]
    DerivedKw,
    #[token("differs")]
    DiffersKw,
    #[token("disjoint")]
    DisjointKw,
    #[token("disjoining")]
    DisjoiningKw,
    #[token("do")]
    DoKw,
    #[token("doc")]
    DocKw,
    #[token("done")]
    DoneKw,
    #[token("else")]
    ElseKw,
    #[token("end")]
    EndKw,
    #[token("entry")]
    EntryKw,
    #[token("enum")]
    EnumKw,
    #[token("enumeration")]
    EnumerationKw,
    #[token("exhibit")]
    ExhibitKw,
    #[token("exit")]
    ExitKw,
    #[token("expose")]
    ExposeKw,
    #[token("event")]
    EventKw,
    #[token("expr")]
    ExprKw,
    #[token("false")]
    FalseKw,
    #[token("feature")]
    FeatureKw,
    #[token("filter")]
    FilterKw,
    #[token("first")]
    FirstKw,
    #[token("flow")]
    FlowKw,
    #[token("for")]
    ForKw,
    #[token("fork")]
    ForkKw,
    #[token("frame")]
    FrameKw,
    #[token("from")]
    FromKw,
    #[token("function")]
    FunctionKw,
    #[token("hastype")]
    HastypeKw,
    #[token("if")]
    IfKw,
    #[token("implies")]
    ImpliesKw,
    #[token("import")]
    ImportKw,
    #[token("in")]
    InKw,
    #[token("include")]
    IncludeKw,
    #[token("individual")]
    IndividualKw,
    #[token("inout")]
    InoutKw,
    #[token("interaction")]
    InteractionKw,
    #[token("interface")]
    InterfaceKw,
    #[token("intersects")]
    IntersectsKw,
    #[token("inv")]
    InvKw,
    #[token("inverse")]
    InverseKw,
    #[token("istype")]
    IstypeKw,
    #[token("item")]
    ItemKw,
    #[token("join")]
    JoinKw,
    #[token("language")]
    LanguageKw,
    #[token("library")]
    LibraryKw,
    #[token("locale")]
    LocaleKw,
    #[token("loop")]
    LoopKw,
    #[token("merge")]
    MergeKw,
    #[token("message")]
    MessageKw,
    #[token("meta")]
    MetaKw,
    #[token("metaclass")]
    MetaclassKw,
    #[token("metadata")]
    MetadataKw,
    #[token("nonunique")]
    NonuniqueKw,
    #[token("not")]
    NotKw,
    #[token("new")]
    NewKw,
    #[token("null")]
    NullKw,
    #[token("objective")]
    ObjectiveKw,
    #[token("occurrence")]
    OccurrenceKw,
    #[token("of")]
    OfKw,
    #[token("or")]
    OrKw,
    #[token("ordered")]
    OrderedKw,
    #[token("out")]
    OutKw,
    #[token("package")]
    PackageKw,
    #[token("part")]
    PartKw,
    #[token("perform")]
    PerformKw,
    #[token("port")]
    PortKw,
    #[token("portion")]
    PortionKw,
    #[token("predicate")]
    PredicateKw,
    #[token("private")]
    PrivateKw,
    #[token("protected")]
    ProtectedKw,
    #[token("public")]
    PublicKw,
    #[token("readonly")]
    ReadonlyKw,
    #[token("redefines")]
    RedefinesKw,
    #[token("ref")]
    RefKw,
    #[token("references")]
    ReferencesKw,
    #[token("render")]
    RenderKw,
    #[token("rendering")]
    RenderingKw,
    #[token("rep")]
    RepKw,
    #[token("require")]
    RequireKw,
    #[token("requirement")]
    RequirementKw,
    #[token("return")]
    ReturnKw,
    #[token("satisfy")]
    SatisfyKw,
    #[token("send")]
    SendKw,
    #[token("specializes")]
    SpecializesKw,
    #[token("stakeholder")]
    StakeholderKw,
    #[token("standard")]
    StandardKw,
    #[token("start")]
    StartKw,
    #[token("state")]
    StateKw,
    #[token("step")]
    StepKw,
    #[token("struct")]
    StructKw,
    #[token("snapshot")]
    SnapshotKw,
    #[token("subject")]
    SubjectKw,
    #[token("subsets")]
    SubsetsKw,
    #[token("succession")]
    SuccessionKw,
    #[token("terminate")]
    TerminateKw,
    #[token("then")]
    ThenKw,
    #[token("this")]
    ThisKw,
    #[token("timeslice")]
    TimesliceKw,
    #[token("to")]
    ToKw,
    #[token("transition")]
    TransitionKw,
    #[token("true")]
    TrueKw,
    #[token("type")]
    TypeKw,
    #[token("typed")]
    TypedKw,
    #[token("unions")]
    UnionsKw,
    #[token("until")]
    UntilKw,
    #[token("use")]
    UseKw,
    #[token("variant")]
    VariantKw,
    #[token("variation")]
    VariationKw,
    #[token("verification")]
    VerificationKw,
    #[token("verify")]
    VerifyKw,
    #[token("via")]
    ViaKw,
    #[token("view")]
    ViewKw,
    #[token("viewpoint")]
    ViewpointKw,
    #[token("when")]
    WhenKw,
    #[token("while")]
    WhileKw,
    #[token("xor")]
    XorKw,
}

impl From<LogosToken> for SyntaxKind {
    fn from(token: LogosToken) -> Self {
        use LogosToken::*;
        match token {
            // Trivia
            Whitespace => SyntaxKind::WHITESPACE,
            LineComment => SyntaxKind::LINE_COMMENT,
            BlockComment => SyntaxKind::BLOCK_COMMENT,

            // Literals
            Ident | UnrestrictedName => SyntaxKind::IDENT,
            Integer => SyntaxKind::INTEGER,
            Decimal => SyntaxKind::DECIMAL,
            String => SyntaxKind::STRING,

            // Multi-char punctuation
            ColonColonGt => SyntaxKind::COLON_COLON_GT,
            ColonGtGt => SyntaxKind::COLON_GT_GT,
            ColonGt => SyntaxKind::COLON_GT,
            ColonColon => SyntaxKind::COLON_COLON,
            ColonEq => SyntaxKind::COLON_EQ,
            DotDot => SyntaxKind::DOT_DOT,
            EqEqEq => SyntaxKind::EQ_EQ_EQ,
            BangEqEq => SyntaxKind::BANG_EQ_EQ,
            EqEq => SyntaxKind::EQ_EQ,
            BangEq => SyntaxKind::BANG_EQ,
            LtEq => SyntaxKind::LT_EQ,
            GtEq => SyntaxKind::GT_EQ,
            Arrow => SyntaxKind::ARROW,
            FatArrow => SyntaxKind::FAT_ARROW,
            AtAt => SyntaxKind::AT_AT,
            StarStar => SyntaxKind::STAR_STAR,
            QuestionQuestion => SyntaxKind::QUESTION_QUESTION,
            AmpAmp => SyntaxKind::AMP_AMP,
            PipePipe => SyntaxKind::PIPE_PIPE,

            // Single-char punctuation
            LBrace => SyntaxKind::L_BRACE,
            RBrace => SyntaxKind::R_BRACE,
            LBracket => SyntaxKind::L_BRACKET,
            RBracket => SyntaxKind::R_BRACKET,
            LParen => SyntaxKind::L_PAREN,
            RParen => SyntaxKind::R_PAREN,
            Semicolon => SyntaxKind::SEMICOLON,
            Colon => SyntaxKind::COLON,
            Dot => SyntaxKind::DOT,
            Comma => SyntaxKind::COMMA,
            Eq => SyntaxKind::EQ,
            Lt => SyntaxKind::LT,
            Gt => SyntaxKind::GT,
            At => SyntaxKind::AT,
            Hash => SyntaxKind::HASH,
            Star => SyntaxKind::STAR,
            Plus => SyntaxKind::PLUS,
            Minus => SyntaxKind::MINUS,
            Slash => SyntaxKind::SLASH,
            Percent => SyntaxKind::PERCENT,
            Caret => SyntaxKind::CARET,
            Tilde => SyntaxKind::TILDE,
            Question => SyntaxKind::QUESTION,
            Bang => SyntaxKind::BANG,
            Pipe => SyntaxKind::PIPE,
            Amp => SyntaxKind::AMP,

            // Keywords
            AboutKw => SyntaxKind::ABOUT_KW,
            AbstractKw => SyntaxKind::ABSTRACT_KW,
            AcceptKw => SyntaxKind::ACCEPT_KW,
            ActionKw => SyntaxKind::ACTION_KW,
            ActorKw => SyntaxKind::ACTOR_KW,
            AfterKw => SyntaxKind::AFTER_KW,
            AliasKw => SyntaxKind::ALIAS_KW,
            AllKw => SyntaxKind::ALL_KW,
            AllocationKw => SyntaxKind::ALLOCATION_KW,
            AllocateKw => SyntaxKind::ALLOCATE_KW,
            AnalysisKw => SyntaxKind::ANALYSIS_KW,
            AndKw => SyntaxKind::AND_KW,
            AsKw => SyntaxKind::AS_KW,
            AssertKw => SyntaxKind::ASSERT_KW,
            AssignKw => SyntaxKind::ASSIGN_KW,
            AssocKw => SyntaxKind::ASSOC_KW,
            AssumeKw => SyntaxKind::ASSUME_KW,
            AtKw => SyntaxKind::AT_KW,
            AttributeKw => SyntaxKind::ATTRIBUTE_KW,
            BehaviorKw => SyntaxKind::BEHAVIOR_KW,
            BindKw => SyntaxKind::BIND_KW,
            BindingKw => SyntaxKind::BINDING_KW,
            ByKw => SyntaxKind::BY_KW,
            CalcKw => SyntaxKind::CALC_KW,
            CaseKw => SyntaxKind::CASE_KW,
            ClassKw => SyntaxKind::CLASS_KW,
            ClassifierKw => SyntaxKind::CLASSIFIER_KW,
            CommentKw => SyntaxKind::COMMENT_KW,
            CompositeKw => SyntaxKind::COMPOSITE_KW,
            ConcernKw => SyntaxKind::CONCERN_KW,
            ConnectKw => SyntaxKind::CONNECT_KW,
            ConnectionKw => SyntaxKind::CONNECTION_KW,
            ConnectorKw => SyntaxKind::CONNECTOR_KW,
            ConstantKw => SyntaxKind::CONSTANT_KW,
            ConstraintKw => SyntaxKind::CONSTRAINT_KW,
            ConjugatesKw => SyntaxKind::CONJUGATES_KW,
            CrossesKw => SyntaxKind::CROSSES_KW,
            DatatypeKw => SyntaxKind::DATATYPE_KW,
            DecideKw => SyntaxKind::DECIDE_KW,
            DefKw => SyntaxKind::DEF_KW,
            DefaultKw => SyntaxKind::DEFAULT_KW,
            DefinedKw => SyntaxKind::DEFINED_KW,
            DependencyKw => SyntaxKind::DEPENDENCY_KW,
            DerivedKw => SyntaxKind::DERIVED_KW,
            DiffersKw => SyntaxKind::DIFFERS_KW,
            DisjointKw => SyntaxKind::DISJOINT_KW,
            DisjoiningKw => SyntaxKind::DISJOINING_KW,
            DoKw => SyntaxKind::DO_KW,
            DocKw => SyntaxKind::DOC_KW,
            DoneKw => SyntaxKind::DONE_KW,
            ElseKw => SyntaxKind::ELSE_KW,
            EndKw => SyntaxKind::END_KW,
            EntryKw => SyntaxKind::ENTRY_KW,
            EnumKw => SyntaxKind::ENUM_KW,
            EnumerationKw => SyntaxKind::ENUMERATION_KW,
            ExhibitKw => SyntaxKind::EXHIBIT_KW,
            ExitKw => SyntaxKind::EXIT_KW,
            ExposeKw => SyntaxKind::EXPOSE_KW,
            EventKw => SyntaxKind::EVENT_KW,
            ExprKw => SyntaxKind::EXPR_KW,
            FalseKw => SyntaxKind::FALSE_KW,
            FeatureKw => SyntaxKind::FEATURE_KW,
            FilterKw => SyntaxKind::FILTER_KW,
            FirstKw => SyntaxKind::FIRST_KW,
            FlowKw => SyntaxKind::FLOW_KW,
            ForKw => SyntaxKind::FOR_KW,
            ForkKw => SyntaxKind::FORK_KW,
            FrameKw => SyntaxKind::FRAME_KW,
            FromKw => SyntaxKind::FROM_KW,
            FunctionKw => SyntaxKind::FUNCTION_KW,
            HastypeKw => SyntaxKind::HASTYPE_KW,
            IfKw => SyntaxKind::IF_KW,
            ImpliesKw => SyntaxKind::IMPLIES_KW,
            ImportKw => SyntaxKind::IMPORT_KW,
            InKw => SyntaxKind::IN_KW,
            IncludeKw => SyntaxKind::INCLUDE_KW,
            IndividualKw => SyntaxKind::INDIVIDUAL_KW,
            InoutKw => SyntaxKind::INOUT_KW,
            InteractionKw => SyntaxKind::INTERACTION_KW,
            InterfaceKw => SyntaxKind::INTERFACE_KW,
            IntersectsKw => SyntaxKind::INTERSECTS_KW,
            InvKw => SyntaxKind::INV_KW,
            InverseKw => SyntaxKind::INVERSE_KW,
            IstypeKw => SyntaxKind::ISTYPE_KW,
            ItemKw => SyntaxKind::ITEM_KW,
            JoinKw => SyntaxKind::JOIN_KW,
            LanguageKw => SyntaxKind::LANGUAGE_KW,
            LibraryKw => SyntaxKind::LIBRARY_KW,
            LocaleKw => SyntaxKind::LOCALE_KW,
            LoopKw => SyntaxKind::LOOP_KW,
            MergeKw => SyntaxKind::MERGE_KW,
            MessageKw => SyntaxKind::MESSAGE_KW,
            MetaKw => SyntaxKind::META_KW,
            MetaclassKw => SyntaxKind::METACLASS_KW,
            MetadataKw => SyntaxKind::METADATA_KW,
            NonuniqueKw => SyntaxKind::NONUNIQUE_KW,
            NotKw => SyntaxKind::NOT_KW,
            NewKw => SyntaxKind::NEW_KW,
            NullKw => SyntaxKind::NULL_KW,
            ObjectiveKw => SyntaxKind::OBJECTIVE_KW,
            OccurrenceKw => SyntaxKind::OCCURRENCE_KW,
            OfKw => SyntaxKind::OF_KW,
            OrKw => SyntaxKind::OR_KW,
            OrderedKw => SyntaxKind::ORDERED_KW,
            OutKw => SyntaxKind::OUT_KW,
            PackageKw => SyntaxKind::PACKAGE_KW,
            PartKw => SyntaxKind::PART_KW,
            PerformKw => SyntaxKind::PERFORM_KW,
            PortKw => SyntaxKind::PORT_KW,
            PortionKw => SyntaxKind::PORTION_KW,
            PredicateKw => SyntaxKind::PREDICATE_KW,
            PrivateKw => SyntaxKind::PRIVATE_KW,
            ProtectedKw => SyntaxKind::PROTECTED_KW,
            PublicKw => SyntaxKind::PUBLIC_KW,
            ReadonlyKw => SyntaxKind::READONLY_KW,
            RedefinesKw => SyntaxKind::REDEFINES_KW,
            RefKw => SyntaxKind::REF_KW,
            ReferencesKw => SyntaxKind::REFERENCES_KW,
            RenderKw => SyntaxKind::RENDER_KW,
            RenderingKw => SyntaxKind::RENDERING_KW,
            RepKw => SyntaxKind::REP_KW,
            RequireKw => SyntaxKind::REQUIRE_KW,
            RequirementKw => SyntaxKind::REQUIREMENT_KW,
            ReturnKw => SyntaxKind::RETURN_KW,
            SatisfyKw => SyntaxKind::SATISFY_KW,
            SendKw => SyntaxKind::SEND_KW,
            SpecializesKw => SyntaxKind::SPECIALIZES_KW,
            StakeholderKw => SyntaxKind::STAKEHOLDER_KW,
            StandardKw => SyntaxKind::STANDARD_KW,
            StartKw => SyntaxKind::START_KW,
            StateKw => SyntaxKind::STATE_KW,
            StepKw => SyntaxKind::STEP_KW,
            StructKw => SyntaxKind::STRUCT_KW,
            SnapshotKw => SyntaxKind::SNAPSHOT_KW,
            SubjectKw => SyntaxKind::SUBJECT_KW,
            SubsetsKw => SyntaxKind::SUBSETS_KW,
            SuccessionKw => SyntaxKind::SUCCESSION_KW,
            TerminateKw => SyntaxKind::TERMINATE_KW,
            ThenKw => SyntaxKind::THEN_KW,
            ThisKw => SyntaxKind::THIS_KW,
            TimesliceKw => SyntaxKind::TIMESLICE_KW,
            ToKw => SyntaxKind::TO_KW,
            TransitionKw => SyntaxKind::TRANSITION_KW,
            TrueKw => SyntaxKind::TRUE_KW,
            TypeKw => SyntaxKind::TYPE_KW,
            TypedKw => SyntaxKind::TYPED_KW,
            UnionsKw => SyntaxKind::UNIONS_KW,
            UntilKw => SyntaxKind::UNTIL_KW,
            UseKw => SyntaxKind::USE_KW,
            VariantKw => SyntaxKind::VARIANT_KW,
            VariationKw => SyntaxKind::VARIATION_KW,
            VerificationKw => SyntaxKind::VERIFICATION_KW,
            VerifyKw => SyntaxKind::VERIFY_KW,
            ViaKw => SyntaxKind::VIA_KW,
            ViewKw => SyntaxKind::VIEW_KW,
            ViewpointKw => SyntaxKind::VIEWPOINT_KW,
            WhenKw => SyntaxKind::WHEN_KW,
            WhileKw => SyntaxKind::WHILE_KW,
            XorKw => SyntaxKind::XOR_KW,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lex_package() {
        let tokens: Vec<_> = Lexer::new("package Test;").collect();
        assert_eq!(tokens.len(), 4); // package, whitespace, Test, ;
        assert_eq!(tokens[0].kind, SyntaxKind::PACKAGE_KW);
        assert_eq!(tokens[1].kind, SyntaxKind::WHITESPACE);
        assert_eq!(tokens[2].kind, SyntaxKind::IDENT);
        assert_eq!(tokens[3].kind, SyntaxKind::SEMICOLON);
    }

    #[test]
    fn test_lex_qualified_name() {
        let tokens: Vec<_> = Lexer::new("A::B::C").collect();
        assert_eq!(tokens[0].kind, SyntaxKind::IDENT);
        assert_eq!(tokens[1].kind, SyntaxKind::COLON_COLON);
        assert_eq!(tokens[2].kind, SyntaxKind::IDENT);
        assert_eq!(tokens[3].kind, SyntaxKind::COLON_COLON);
        assert_eq!(tokens[4].kind, SyntaxKind::IDENT);
    }

    #[test]
    fn test_lex_specializes() {
        let tokens: Vec<_> = Lexer::new("part def A :> B;").collect();
        let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
        assert!(kinds.contains(&SyntaxKind::PART_KW));
        assert!(kinds.contains(&SyntaxKind::DEF_KW));
        assert!(kinds.contains(&SyntaxKind::COLON_GT));
    }

    #[test]
    fn test_lex_comment() {
        let tokens: Vec<_> = Lexer::new("// comment\npackage").collect();
        assert_eq!(tokens[0].kind, SyntaxKind::LINE_COMMENT);
        assert_eq!(tokens[1].kind, SyntaxKind::WHITESPACE);
        assert_eq!(tokens[2].kind, SyntaxKind::PACKAGE_KW);
    }

    #[test]
    fn test_lex_import_wildcard() {
        let tokens: Vec<_> = Lexer::new("import ISQ::*;").collect();
        let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
        assert!(kinds.contains(&SyntaxKind::IMPORT_KW));
        assert!(kinds.contains(&SyntaxKind::COLON_COLON));
        assert!(kinds.contains(&SyntaxKind::STAR));
        assert!(kinds.contains(&SyntaxKind::SEMICOLON));
    }
}
