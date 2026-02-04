# Parser Error Handling Improvement Plan

## Overview

This document outlines a comprehensive plan to improve error messages in the syster-base parser layer, following a Test-Driven Development (TDD) approach.

**Goal**: Provide clear, actionable, and context-aware error messages that help users quickly identify and fix syntax issues in SysML/KerML files.

---

## Current State Analysis

### Existing Error Infrastructure

| Component | Location | Purpose |
|-----------|----------|---------|
| `SyntaxError` | `src/parser/parser.rs:34-47` | Basic error struct with message + range |
| `kind_to_name()` | `src/parser/parser.rs:63-132` | Token to human-readable name mapping |
| `error()` | `src/parser/parser.rs:285-290` | Records error at current position |
| `error_recover()` | `src/parser/parser.rs:292-306` | Error + skip to recovery tokens |
| `ParseError` | `src/parser/result.rs` | Higher-level error with Position |

### Current Limitations

1. **Generic messages**: "expected X, found Y" lacks context
2. **Incomplete `kind_to_name()`**: Many keywords fall through to generic "keyword"
3. **No error codes**: Hard to filter, document, or reference specific errors
4. **No suggestions/hints**: Users must figure out fixes themselves
5. **No related spans**: Can't show "opened here" for unclosed braces
6. **Single recovery strategy**: Same recovery set for all contexts

---

## Architecture

### Proposed Module Structure

```
src/parser/
├── mod.rs                    # Public exports
├── parser.rs                 # Core parser state (exists)
├── lexer.rs                  # Tokenization (exists)
├── syntax_kind.rs            # Token kinds (exists)
├── errors/                   # NEW: Error handling module
│   ├── mod.rs                # Public exports
│   ├── error.rs              # SyntaxError struct + ErrorCode enum
│   ├── codes.rs              # Error code definitions + messages
│   ├── context.rs            # Parse context tracking
│   ├── recovery.rs           # Recovery strategies per context
│   ├── suggestions.rs        # Hint generation for common mistakes
│   └── display.rs            # Human-readable formatting
├── grammar/
│   ├── kerml.rs              # KerML grammar (update error calls)
│   ├── sysml.rs              # SysML grammar (update error calls)
│   └── kerml_expressions.rs  # Expression grammar (update error calls)
└── ast.rs                    # AST types (exists)
```

### Key Design Decisions

1. **Separate `errors/` module**: Isolates error handling logic, makes it testable
2. **Error codes as enum**: Type-safe, exhaustive, easy to document
3. **Context stack**: Parser tracks what it's parsing for better messages
4. **Recovery strategies map**: Context → recovery token set

### Integration Points

```
┌─────────────────────────────────────────────────────────────┐
│                      Grammar Modules                         │
│              (kerml.rs, sysml.rs, expressions.rs)           │
└─────────────────────────┬───────────────────────────────────┘
                          │ calls error methods
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                    Parser (parser.rs)                        │
│  - Holds parse context stack                                 │
│  - Delegates to errors/ module                               │
└─────────────────────────┬───────────────────────────────────┘
                          │ uses
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                    Errors Module (errors/)                   │
│  - ErrorCode definitions                                     │
│  - SyntaxError construction                                  │
│  - Hint generation                                           │
│  - Recovery strategies                                       │
└─────────────────────────────────────────────────────────────┘
                          │ outputs
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                    LSP / Diagnostics                         │
│  (language-server/crates/syster-lsp/src/server/diagnostics.rs)│
└─────────────────────────────────────────────────────────────┘
```

---

## Implementation Phases

### Phase 1: Foundation - Error Codes & Enhanced Structure

**Objective**: Create the error infrastructure without changing existing behavior.

#### Step 1.1: Create Error Module Structure

**Files to create**:
- `src/parser/errors/mod.rs`
- `src/parser/errors/error.rs`
- `src/parser/errors/codes.rs`

#### Step 1.2: Define Error Codes

```rust
// src/parser/errors/codes.rs

/// Error codes for parser diagnostics
/// 
/// Naming convention: E{category}{number}
/// - E01xx: Lexical errors (invalid tokens)
/// - E02xx: Structural errors (braces, semicolons)
/// - E03xx: Declaration errors (definitions, usages)
/// - E04xx: Expression errors
/// - E05xx: Import/namespace errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    // E01xx: Lexical
    E0101, // Invalid character
    E0102, // Unterminated string
    E0103, // Unterminated block comment
    
    // E02xx: Structural
    E0201, // Missing semicolon
    E0202, // Unclosed brace
    E0203, // Unclosed parenthesis
    E0204, // Unclosed bracket
    E0205, // Unexpected closing delimiter
    E0206, // Empty body
    
    // E03xx: Declarations
    E0301, // Missing identifier
    E0302, // Missing 'def' keyword
    E0303, // Invalid definition prefix
    E0304, // Unexpected token in definition
    E0305, // Missing type annotation
    
    // E04xx: Expressions
    E0401, // Invalid expression
    E0402, // Missing operand
    E0403, // Invalid operator
    E0404, // Unclosed function call
    
    // E05xx: Imports/Namespaces
    E0501, // Invalid import path
    E0502, // Missing package name
    E0503, // Invalid alias
    
    // E09xx: Generic/Fallback
    E0901, // Unexpected token
    E0902, // Expected specific token
}
```

#### Step 1.3: Enhanced SyntaxError Struct

```rust
// src/parser/errors/error.rs

use rowan::TextRange;
use super::codes::ErrorCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Hint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelatedInfo {
    pub message: String,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxError {
    /// Human-readable error message
    pub message: String,
    /// Source location
    pub range: TextRange,
    /// Categorized error code
    pub code: ErrorCode,
    /// Error severity
    pub severity: Severity,
    /// Optional suggestion for fixing
    pub hint: Option<String>,
    /// Related locations (e.g., "opened here")
    pub related: Vec<RelatedInfo>,
}
```

#### TDD: Phase 1 Tests

**File**: `src/parser/errors/tests.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_code_display() {
        assert_eq!(ErrorCode::E0201.as_str(), "E0201");
        assert_eq!(ErrorCode::E0201.message(), "missing semicolon");
    }
    
    #[test]
    fn test_syntax_error_creation() {
        let err = SyntaxError::new(
            "expected ';'",
            TextRange::empty(TextSize::new(10)),
            ErrorCode::E0201,
        );
        assert_eq!(err.severity, Severity::Error);
        assert!(err.hint.is_none());
    }
    
    #[test]
    fn test_syntax_error_with_hint() {
        let err = SyntaxError::builder(ErrorCode::E0201)
            .message("expected ';' after part definition")
            .range(TextRange::empty(TextSize::new(10)))
            .hint("add ';' at the end of the definition")
            .build();
        assert!(err.hint.is_some());
    }
}
```

---

### Phase 2: Expand Token Names

**Objective**: Complete coverage of `kind_to_name()` for all keywords.

#### Step 2.1: Audit All SyntaxKind Variants

Review `src/parser/syntax_kind.rs` and ensure every variant has a mapping.

#### Step 2.2: Implement Complete Mapping

```rust
// Add to src/parser/parser.rs or move to src/parser/errors/display.rs

pub fn kind_to_name(kind: SyntaxKind) -> &'static str {
    match kind {
        // ... existing punctuation ...
        
        // SysML Keywords
        SyntaxKind::PART_KW => "'part'",
        SyntaxKind::ACTION_KW => "'action'",
        SyntaxKind::STATE_KW => "'state'",
        SyntaxKind::REQUIREMENT_KW => "'requirement'",
        SyntaxKind::CONSTRAINT_KW => "'constraint'",
        SyntaxKind::ATTRIBUTE_KW => "'attribute'",
        SyntaxKind::PORT_KW => "'port'",
        SyntaxKind::ITEM_KW => "'item'",
        SyntaxKind::PACKAGE_KW => "'package'",
        SyntaxKind::IMPORT_KW => "'import'",
        SyntaxKind::DEF_KW => "'def'",
        // ... all other keywords ...
        
        // Fallback with debug info
        _ => {
            // Log unknown kinds in debug builds
            #[cfg(debug_assertions)]
            eprintln!("WARNING: kind_to_name missing case for {:?}", kind);
            "token"
        }
    }
}
```

#### TDD: Phase 2 Tests

**File**: `tests/parser/tests_error_display.rs`

```rust
use syster::parser::{SyntaxKind, kind_to_name};

#[test]
fn test_all_keywords_have_names() {
    let keywords = [
        (SyntaxKind::PART_KW, "'part'"),
        (SyntaxKind::ACTION_KW, "'action'"),
        (SyntaxKind::DEF_KW, "'def'"),
        (SyntaxKind::PACKAGE_KW, "'package'"),
        // ... add all keywords
    ];
    
    for (kind, expected) in keywords {
        assert_eq!(
            kind_to_name(kind), expected,
            "kind_to_name({:?}) should return {}", kind, expected
        );
    }
}

#[test]
fn test_no_generic_keyword_fallback() {
    // Ensure no keyword falls through to generic "keyword"
    let all_keywords: Vec<SyntaxKind> = SyntaxKind::all_keywords();
    for kw in all_keywords {
        let name = kind_to_name(kw);
        assert_ne!(name, "keyword", "{:?} should have specific name", kw);
    }
}
```

---

### Phase 3: Context-Aware Messages

**Objective**: Parser tracks context and generates specific messages.

#### Step 3.1: Define Parse Contexts

```rust
// src/parser/errors/context.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseContext {
    TopLevel,
    PackageBody,
    PartDefinition,
    ActionDefinition,
    ActionBody,
    StateDefinition,
    StateBody,
    RequirementBody,
    Expression,
    TypeAnnotation,
    Multiplicity,
    Import,
}

impl ParseContext {
    /// Human-readable description for error messages
    pub fn description(&self) -> &'static str {
        match self {
            Self::TopLevel => "at top level",
            Self::PackageBody => "in package body",
            Self::PartDefinition => "in part definition",
            Self::ActionDefinition => "in action definition",
            Self::ActionBody => "in action body",
            Self::StateBody => "in state body",
            Self::RequirementBody => "in requirement body",
            Self::Expression => "in expression",
            Self::TypeAnnotation => "in type annotation",
            Self::Multiplicity => "in multiplicity",
            Self::Import => "in import statement",
        }
    }
    
    /// What tokens are expected in this context?
    pub fn expected_description(&self) -> &'static str {
        match self {
            Self::PackageBody => "a definition (part, action, etc.), usage, or import",
            Self::ActionBody => "an action element (accept, send, if, etc.) or nested action",
            Self::StateBody => "a state element (entry, exit, do) or transition",
            Self::Expression => "an expression (literal, identifier, or operator)",
            // ...
        }
    }
}
```

#### Step 3.2: Add Context Stack to Parser

```rust
// In src/parser/parser.rs

struct Parser<'a> {
    tokens: &'a [Token<'a>],
    pos: usize,
    builder: GreenNodeBuilder<'static>,
    errors: Vec<SyntaxError>,
    source: &'a str,
    depth: usize,
    context_stack: Vec<ParseContext>,  // NEW
}

impl<'a> Parser<'a> {
    fn push_context(&mut self, ctx: ParseContext) {
        self.context_stack.push(ctx);
    }
    
    fn pop_context(&mut self) {
        self.context_stack.pop();
    }
    
    fn current_context(&self) -> ParseContext {
        self.context_stack.last().copied().unwrap_or(ParseContext::TopLevel)
    }
}
```

#### Step 3.3: Context-Aware Error Method

```rust
fn error_in_context(&mut self, code: ErrorCode) {
    let ctx = self.current_context();
    let found = self.current()
        .map(|t| format!("'{}'", t.text))
        .unwrap_or_else(|| "end of file".to_string());
    
    let message = format!(
        "unexpected {} {}—expected {}",
        found,
        ctx.description(),
        ctx.expected_description()
    );
    
    self.errors.push(SyntaxError {
        message,
        range: self.current_range(),
        code,
        severity: Severity::Error,
        hint: None,
        related: vec![],
    });
}
```

#### TDD: Phase 3 Tests

**File**: `tests/parser/tests_context_errors.rs`

```rust
use syster::parser::parse_sysml;

#[test]
fn test_error_in_package_body() {
    let source = "package Foo { @@@ }";
    let parse = parse_sysml(source);
    
    assert!(!parse.ok());
    let err = &parse.errors[0];
    assert!(err.message.contains("in package body"));
    assert!(err.message.contains("expected"));
}

#[test]
fn test_error_in_action_body() {
    let source = "action def Foo { ??? }";
    let parse = parse_sysml(source);
    
    assert!(!parse.ok());
    let err = &parse.errors[0];
    assert!(err.message.contains("in action") || err.message.contains("action body"));
}

#[test]
fn test_error_at_top_level() {
    let source = "@@@";
    let parse = parse_sysml(source);
    
    assert!(!parse.ok());
    let err = &parse.errors[0];
    assert!(err.message.contains("top level"));
}
```

---

### Phase 4: Hints & Suggestions

**Objective**: Provide actionable suggestions for common mistakes.

#### Step 4.1: Common Mistake Patterns

```rust
// src/parser/errors/suggestions.rs

use super::codes::ErrorCode;
use crate::parser::SyntaxKind;

pub struct MistakePattern {
    /// What we found
    pub found: &'static [SyntaxKind],
    /// What context we're in
    pub context: ParseContext,
    /// The likely intended pattern
    pub likely_intent: &'static str,
    /// Suggestion to show user
    pub hint: &'static str,
}

pub const COMMON_MISTAKES: &[MistakePattern] = &[
    // Missing 'def' keyword
    MistakePattern {
        found: &[SyntaxKind::PART_KW, SyntaxKind::IDENT, SyntaxKind::L_BRACE],
        context: ParseContext::PackageBody,
        likely_intent: "part def Name { }",
        hint: "add 'def' after 'part' to define a part type",
    },
    // Missing semicolon before next definition
    MistakePattern {
        found: &[SyntaxKind::PART_KW],  // after a complete definition
        context: ParseContext::PackageBody,
        likely_intent: "previous statement needs ';'",
        hint: "add ';' at the end of the previous definition",
    },
    // Using = instead of :
    MistakePattern {
        found: &[SyntaxKind::IDENT, SyntaxKind::EQ, SyntaxKind::IDENT],
        context: ParseContext::PartDefinition,
        likely_intent: "feature : Type",
        hint: "use ':' for type annotation instead of '='",
    },
];

/// Try to find a matching mistake pattern and return a hint
pub fn find_hint(
    tokens: &[SyntaxKind],
    context: ParseContext,
) -> Option<&'static str> {
    for pattern in COMMON_MISTAKES {
        if pattern.context == context && tokens_match(tokens, pattern.found) {
            return Some(pattern.hint);
        }
    }
    None
}
```

#### TDD: Phase 4 Tests

**File**: `tests/parser/tests_error_hints.rs`

```rust
use syster::parser::parse_sysml;

#[test]
fn test_hint_missing_def_keyword() {
    // User wrote "part Foo { }" instead of "part def Foo { }"
    let source = "package P { part Foo { } }";
    let parse = parse_sysml(source);
    
    // Should either parse as usage OR give helpful error
    if !parse.ok() {
        let hints: Vec<_> = parse.errors.iter()
            .filter_map(|e| e.hint.as_ref())
            .collect();
        // Check that we suggest adding 'def'
        assert!(
            hints.iter().any(|h| h.contains("def")),
            "Should suggest adding 'def' keyword"
        );
    }
}

#[test]
fn test_hint_missing_semicolon() {
    let source = "part def A { } part def B { }";
    let parse = parse_sysml(source);
    
    if !parse.ok() {
        let err = &parse.errors[0];
        assert!(
            err.hint.as_ref().map_or(false, |h| h.contains(";")),
            "Should suggest adding semicolon"
        );
    }
}

#[test]
fn test_hint_wrong_operator() {
    let source = "part def Car { wheels = Integer; }";
    let parse = parse_sysml(source);
    
    if !parse.ok() {
        let hints: Vec<_> = parse.errors.iter()
            .filter_map(|e| e.hint.as_ref())
            .collect();
        assert!(
            hints.iter().any(|h| h.contains(":") || h.contains("type")),
            "Should suggest using ':' for type annotation"
        );
    }
}
```

---

### Phase 5: Smart Recovery

**Objective**: Context-specific recovery to minimize cascading errors.

#### Step 5.1: Recovery Sets per Context

```rust
// src/parser/errors/recovery.rs

use crate::parser::SyntaxKind;
use super::context::ParseContext;

/// Get recovery tokens for a parse context
pub fn recovery_set(context: ParseContext) -> &'static [SyntaxKind] {
    match context {
        ParseContext::TopLevel => &[
            SyntaxKind::PACKAGE_KW,
            SyntaxKind::PART_KW,
            SyntaxKind::ACTION_KW,
            SyntaxKind::IMPORT_KW,
        ],
        ParseContext::PackageBody => &[
            SyntaxKind::PART_KW,
            SyntaxKind::ACTION_KW,
            SyntaxKind::STATE_KW,
            SyntaxKind::REQUIREMENT_KW,
            SyntaxKind::IMPORT_KW,
            SyntaxKind::PACKAGE_KW,
            SyntaxKind::R_BRACE,
            SyntaxKind::PUBLIC_KW,
            SyntaxKind::PRIVATE_KW,
        ],
        ParseContext::ActionBody => &[
            SyntaxKind::ACCEPT_KW,
            SyntaxKind::SEND_KW,
            SyntaxKind::IF_KW,
            SyntaxKind::WHILE_KW,
            SyntaxKind::FOR_KW,
            SyntaxKind::ACTION_KW,
            SyntaxKind::THEN_KW,
            SyntaxKind::R_BRACE,
        ],
        ParseContext::StateBody => &[
            SyntaxKind::ENTRY_KW,
            SyntaxKind::EXIT_KW,
            SyntaxKind::DO_KW,
            SyntaxKind::TRANSITION_KW,
            SyntaxKind::STATE_KW,
            SyntaxKind::R_BRACE,
        ],
        ParseContext::Expression => &[
            SyntaxKind::SEMICOLON,
            SyntaxKind::R_PAREN,
            SyntaxKind::R_BRACE,
            SyntaxKind::R_BRACKET,
            SyntaxKind::COMMA,
        ],
        // ... other contexts
        _ => &[SyntaxKind::SEMICOLON, SyntaxKind::R_BRACE],
    }
}
```

#### Step 5.2: Enhanced Recovery Method

```rust
fn error_recover_contextual(&mut self, code: ErrorCode) {
    let ctx = self.current_context();
    let recovery = recovery_set(ctx);
    
    // Generate context-aware message
    self.error_in_context(code);
    
    // Create ERROR node and skip to recovery point
    self.builder.start_node(SyntaxKind::ERROR.into());
    
    let mut depth = 0;
    while !self.at_eof() {
        // Track brace depth to avoid skipping too much
        match self.current_kind() {
            SyntaxKind::L_BRACE | SyntaxKind::L_PAREN | SyntaxKind::L_BRACKET => {
                depth += 1;
            }
            SyntaxKind::R_BRACE | SyntaxKind::R_PAREN | SyntaxKind::R_BRACKET => {
                if depth > 0 {
                    depth -= 1;
                } else if recovery.contains(&self.current_kind()) {
                    break;
                }
            }
            k if depth == 0 && recovery.contains(&k) => break,
            _ => {}
        }
        self.bump_any();
    }
    
    self.builder.finish_node();
}
```

#### TDD: Phase 5 Tests

**File**: `tests/parser/tests_error_recovery.rs`

```rust
use syster::parser::parse_sysml;

#[test]
fn test_recovery_in_package_continues_parsing() {
    let source = r#"
        package P {
            part def A { @@@ }  // Error here
            part def B { }      // Should still parse this
        }
    "#;
    let parse = parse_sysml(source);
    
    // Should have error but also parse B
    assert!(!parse.ok());
    let tree_text = format!("{:?}", parse.syntax());
    assert!(tree_text.contains("PART_DEFINITION")); // At least one parsed
}

#[test]
fn test_recovery_doesnt_skip_too_much() {
    let source = r#"
        package P {
            part def A {
                feature x { @@@ }  // Error in nested context
            }
            part def B { }
        }
    "#;
    let parse = parse_sysml(source);
    
    // Both A and B should be in the tree
    let tree_text = format!("{:?}", parse.syntax());
    // Count PART_DEFINITION nodes (should be 2)
}

#[test]
fn test_unclosed_brace_reports_opener_location() {
    let source = "package P { part def A {";
    let parse = parse_sysml(source);
    
    assert!(!parse.ok());
    let err = &parse.errors[0];
    // Should have related info pointing to opening brace
    assert!(!err.related.is_empty(), "Should have related location info");
}
```

---

### Phase 6: Related Span Tracking

**Objective**: Track opening delimiters to report "opened here" in unclosed errors.

#### Step 6.1: Delimiter Stack

```rust
// Add to Parser struct

struct DelimiterInfo {
    kind: SyntaxKind,  // L_BRACE, L_PAREN, L_BRACKET
    range: TextRange,
}

struct Parser<'a> {
    // ... existing fields ...
    delimiter_stack: Vec<DelimiterInfo>,
}

impl<'a> Parser<'a> {
    fn push_delimiter(&mut self, kind: SyntaxKind) {
        let range = self.current_range();
        self.delimiter_stack.push(DelimiterInfo { kind, range });
    }
    
    fn pop_delimiter(&mut self, expected: SyntaxKind) -> Option<DelimiterInfo> {
        let closing = match expected {
            SyntaxKind::R_BRACE => SyntaxKind::L_BRACE,
            SyntaxKind::R_PAREN => SyntaxKind::L_PAREN,
            SyntaxKind::R_BRACKET => SyntaxKind::L_BRACKET,
            _ => return None,
        };
        
        if self.delimiter_stack.last().map(|d| d.kind) == Some(closing) {
            self.delimiter_stack.pop()
        } else {
            None
        }
    }
    
    fn report_unclosed_delimiters(&mut self) {
        for delim in self.delimiter_stack.drain(..).rev() {
            let (code, name) = match delim.kind {
                SyntaxKind::L_BRACE => (ErrorCode::E0202, "'{'"),
                SyntaxKind::L_PAREN => (ErrorCode::E0203, "'('"),
                SyntaxKind::L_BRACKET => (ErrorCode::E0204, "'['"),
                _ => continue,
            };
            
            self.errors.push(SyntaxError {
                message: format!("unclosed {}", name),
                range: delim.range,
                code,
                severity: Severity::Error,
                hint: Some(format!("add matching closing delimiter")),
                related: vec![],
            });
        }
    }
}
```

#### TDD: Phase 6 Tests

**File**: `tests/parser/tests_delimiter_tracking.rs`

```rust
use syster::parser::parse_sysml;

#[test]
fn test_unclosed_brace_at_eof() {
    let source = "package P {";
    let parse = parse_sysml(source);
    
    assert!(!parse.ok());
    assert!(parse.errors.iter().any(|e| 
        e.code == ErrorCode::E0202 && e.message.contains("unclosed")
    ));
}

#[test]
fn test_unclosed_nested_braces() {
    let source = "package P { part def A {";
    let parse = parse_sysml(source);
    
    // Should report both unclosed braces
    let unclosed: Vec<_> = parse.errors.iter()
        .filter(|e| e.code == ErrorCode::E0202)
        .collect();
    assert_eq!(unclosed.len(), 2);
}

#[test]
fn test_mismatched_delimiter() {
    let source = "package P { action def A ( } )";
    let parse = parse_sysml(source);
    
    assert!(!parse.ok());
    // Should report mismatched delimiters
}
```

---

## Integration with LSP

### Update Diagnostics Conversion

**File**: `language-server/crates/syster-lsp/src/server/diagnostics.rs`

```rust
// Convert enhanced SyntaxError to LSP Diagnostic
fn syntax_error_to_diagnostic(error: &SyntaxError) -> Diagnostic {
    Diagnostic {
        range: text_range_to_lsp_range(error.range),
        severity: Some(match error.severity {
            Severity::Error => DiagnosticSeverity::ERROR,
            Severity::Warning => DiagnosticSeverity::WARNING,
            Severity::Hint => DiagnosticSeverity::HINT,
        }),
        code: Some(NumberOrString::String(error.code.as_str().to_string())),
        source: Some("syster-parse".to_string()),
        message: error.message.clone(),
        related_information: if error.related.is_empty() {
            None
        } else {
            Some(error.related.iter().map(|r| {
                DiagnosticRelatedInformation {
                    location: Location {
                        uri: current_uri.clone(),
                        range: text_range_to_lsp_range(r.range),
                    },
                    message: r.message.clone(),
                }
            }).collect())
        },
        ..Default::default()
    }
}
```

---

## Test Strategy Summary

| Phase | Test File | Key Tests |
|-------|-----------|-----------|
| 1 | `src/parser/errors/tests.rs` | Error struct creation, codes |
| 2 | `tests/parser/tests_error_display.rs` | All keywords have names |
| 3 | `tests/parser/tests_context_errors.rs` | Context in messages |
| 4 | `tests/parser/tests_error_hints.rs` | Suggestions appear |
| 5 | `tests/parser/tests_error_recovery.rs` | Parsing continues |
| 6 | `tests/parser/tests_delimiter_tracking.rs` | Unclosed delimiters |

### Running Tests

```bash
# Run all parser error tests
cargo test parser::errors
cargo test tests_error

# Run with output to see error messages
cargo test tests_error -- --nocapture
```

---

## Success Criteria

1. **No generic "keyword" fallback**: All keywords return specific names
2. **Error codes on all errors**: Can filter/search by code
3. **Context in 80%+ of errors**: Messages mention where the error occurred
4. **Hints for top 10 mistakes**: Common errors have suggestions
5. **Recovery limits cascading**: Max 3 errors per "region"
6. **Related spans for delimiters**: Unclosed braces show open location

---

## Timeline Estimate

| Phase | Effort | Dependencies |
|-------|--------|--------------|
| Phase 1 | 2-3 days | None |
| Phase 2 | 1 day | Phase 1 |
| Phase 3 | 3-4 days | Phase 1 |
| Phase 4 | 2-3 days | Phase 3 |
| Phase 5 | 2-3 days | Phase 3 |
| Phase 6 | 2-3 days | Phase 1 |

**Total**: ~2-3 weeks for full implementation

Phases 3-6 can be parallelized after Phase 1 is complete.
