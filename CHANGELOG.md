# Changelog

All notable changes to syster-base will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.3-alpha] - 2026-02-10

### Fixed

- **Feature Chain Hover Resolution**: Fixed hover/goto-definition for feature chains like `takePicture.focus` in perform/exhibit/include statements
  - Include domain-specific relationships (Performs, Exhibits, Includes, Satisfies, Asserts, Verifies) in supertypes extraction for usages
  - Recursively follow type chains when resolved type is a usage (not definition) to find member definitions
  - Fixes cases like `perform action takePicture :> TakePicture` where hovering on `.focus` now correctly resolves to `TakePicture::focus`

## [Unreleased]

### Changed

- **AST Code Deduplication** (Issue #13): Reduced code duplication in `ast.rs`
  - Added 11 declarative macros: `has_token_method!`, `first_child_method!`, `children_method!`, `children_vec_method!`, `descendants_method!`, `child_after_keyword_method!`, `body_members_method!`, `find_token_kind_method!`, `source_target_pair!`, `token_to_enum_method!`, `prefix_metadata_method!`
  - Added 6 helper functions: `is_name_token()`, `strip_unrestricted_name()`, `has_token()`, `find_name_token()`, `split_at_keyword()`, `collect_prefix_metadata()`
  - Consolidated `ConnectorEnd::target()` and `endpoint_name()` with shared `end_reference_info()` helper
  - Eliminated ~200+ lines of duplicated boilerplate while maintaining full test coverage

## [0.3.2-alpha] - 2026-02-09

### Fixed

- **Semantic Analysis False Positives**: Fixed 21 false positive errors in stdlib and examples
  - Skip anonymous elements (`<anonymous-*>`, `#*`) in duplicate definition detection
  - Fixed `TransitionUsage::name()` to not return accept payload names as transition names
  - Don't propagate anonymous type_refs to Package symbols
  - Added `Redefines` to supertypes filter for proper redefinition inheritance
  - Fixed `resolve_inherited_member` to follow redefinition chains correctly
  - Added implicit supertypes for usage kinds (Part→Parts::Part, Item→Items::Item, etc.)
  - Implemented SemanticMetadata `baseType` resolution for metadata annotations like `#systemdd`

### Added

- **AnalysisHost Diagnostics API**: New methods for semantic diagnostics
  - `diagnostics(file_id)` - Get diagnostics for a specific file
  - `all_diagnostics()` - Get all diagnostics grouped by file path
  - `all_errors()` - Get errors as `(path, diagnostic)` pairs

- **SemanticMetadata baseType Cache**: Cached resolution for metadata annotation inheritance
  - Thread-safe `RwLock` cache in `SymbolIndex` for baseType lookups
  - Automatic cache invalidation when files are removed

- **Semantic Analysis Tests**: New tests for example files
  - Individual tests for Arrowhead Framework, Simple Vehicle, Analysis Examples, etc.
  - Uses `AnalysisHost` API for cleaner test code

### Changed

- **Performance**: Added caching for SemanticMetadata baseType resolution
  - Lazy population with `RwLock` for thread safety
  - Avoids repeated lookups for the same annotation

## [0.3.1-alpha] - 2026-02-04

### Added

- **Interchange Lossless Roundtrip**: Complete overhaul for byte-perfect XMI roundtrip
  - XMI files now roundtrip with identical byte output
  - Namespace declarations preserved via `ModelMetadata.declared_namespaces`
  - Boolean attributes stored as `Option<bool>` properties for explicit false preservation
  - `href` element details (`_href_tag`, `_href_xsi_type`) preserved for reference elements
  - Attribute ordering matches original XMI specification

- **Element Boolean Setters**: New setter methods that sync field and property
  - `set_abstract()`, `set_variation()`, `set_derived()`, `set_readonly()`, `set_parallel()`
  - Properties are now the single source of truth for roundtrip fidelity

- **YAML Format Support**: Full YAML interchange format with lossless roundtrip
  - Uses `@type`, `@id`, `source`, `target` fields like JSON-LD
  - Relationships stored as separate objects with explicit source/target
  - All relationship kinds including `Disjoining` supported

- **JSON-LD Disjoining Support**: Added missing `Disjoining` relationship kind

### Fixed

- **Clippy Compliance**: Fixed all clippy warnings with `-D warnings`
  - Replaced `split(':').last()` with `rsplit(':').next()`
  - Replaced `or_insert_with(Vec::new)` with `or_default()`
  - Changed `&PathBuf` parameters to `&Path`
  - Removed unit struct `::default()` calls
  - Fixed `len() > 0` to `!is_empty()`

### Removed

- Unused `KERML` constant from `jsonld::context`
- Unused `RESOURCES_DIR` constant from `kpar::paths`
- Unused `element_kind_from_xmi`, `element_kind_to_xmi`, `relationship_kind_from_xmi` functions

## [0.3.0-alpha] - 2026-02-03

### Changed

- **Parser Refactor**: Complete refactor to use Rowan-based Concrete Syntax Tree (CST)
  - Replaced Pest-only parser with Rowan for lossless syntax tree representation
  - Enables incremental parsing and better error recovery
  - Preserves whitespace and comments in the syntax tree
  - Foundation for future formatting and refactoring tools

### Fixed

- **Parsing**: All SysML v2 standard library files (114 files) now parse without errors
- **Parsing**: All sample library files now parse without errors
- **Semantic Tokens**: Fixed span calculation for symbols with type references
  - Symbol spans now only cover the name, not extended to include type refs
  - Quoted names like `'vehicle model 1'` now highlight correctly (17 chars instead of 28)
  - Import spans now use `path_range` for precise highlighting
  - Alias spans now use `name_range` for precise highlighting
  - Anonymous/synthetic symbols (names starting with `<`) are now skipped in semantic tokens

- **Parser**: Added `skip_trivia()` before NAME node in SysML `parse_identification` to exclude leading whitespace from name spans

- **Tests**: Fixed incorrect column positions in `hover_ref_in_tuple_expression` test (49/61 → 50/62)

## [0.2.3-alpha] - 2026-01-29

### Fixed

- **Interchange Module**: Added missing `metadata_annotations` field to `HirSymbol` construction in `integrate.rs`, fixing compilation when using the `interchange` feature

## [0.2.2-alpha] - 2026-01-29

### Added

- **SysML v2 Views Support** (Section 7.26):
  - `ViewDefinition` — Represents `view def` with expose relationships, filter conditions, and rendering specs
  - `ViewUsage` — Represents `view` usages with inherited and local filters
  - `ViewpointDefinition` / `ViewpointUsage` — Stakeholder concern definitions
  - `RenderingDefinition` / `RenderingUsage` — View artifact rendering specifications
  - `ExposeRelationship` — Models `expose` relationships with wildcard support (`::*`, `::**`)
  - `FilterCondition` — Metadata-based filtering (`@SysML::PartUsage`)
  - `WildcardKind` — Direct (`::*`) vs Recursive (`::**`) expose patterns

- **View Application Engine**:
  - `ViewDefinition::apply()` — Apply view to symbols, returning filtered results
  - `ExposeRelationship::resolve()` — Resolve expose patterns to matching qualified names
  - `FilterCondition::matches()` — Evaluate metadata filters against symbol annotations
  - `ViewDefinition::passes_filters()` — Check if metadata passes all filter conditions (AND logic)

- **Filter Import Evaluation** (SysML v2 §7.5.4):
  - `metadata_annotations` field on `HirSymbol` — Tracks applied metadata types
  - `ExtractionResult` — Returns symbols + scope filters + import filters
  - `extract_with_filters()` — Unified extraction that captures filter metadata
  - `add_extraction_result()` on `SymbolIndex` — Adds symbols and registers filters
  - Bracket syntax support: `import X::*[@Safety]` filters by metadata
  - Package-level `filter @Type;` statements restrict wildcard imports

- **Normalized Layer Extensions**:
  - `NormalizedFilter<'a>` — Represents filter statements with metadata references
  - `NormalizedExpose<'a>` — Represents expose relationships in views
  - `NormalizedElement::Filter` and `NormalizedElement::Expose` variants

- **View-specific HIR Data**:
  - `HirSymbol.view_data: Option<ViewData>` — Stores view-related data for view symbols
  - `ViewData` enum — Discriminated union for all view-related types

### Changed

- **Symbol Extraction**: Now preserves Filter and Expose elements from normalized layer
- **Analysis Host**: Uses `add_extraction_result()` to properly register scope and import filters

## [0.2.1-alpha] - 2026-01-24

### Added

- **Relationships in HIR**: Symbols now track their relationships to other symbols
  - `HirRelationship` — Represents a relationship between symbols with kind and target
  - `RelationshipKind` — Enum covering Specializes, TypedBy, Subsets, Redefines, References, Satisfies, Performs, Exhibits, Includes, Asserts, Verifies
  - `HirSymbol.relationships` — Vector of relationships extracted during symbol extraction

- **Type Information API** (`ide/type_info.rs`):
  - `type_info_at` — Retrieve type information at a specific cursor position
  - `goto_type_definition` — Navigate directly from usages to their type definitions
  - `TypeInfo` — Struct containing type name, definition location, and span info

- **Resolved Relationships in Hover**:
  - `ResolvedRelationship` — Pre-resolved relationship with target file/line info for clickable links
  - Hover results now include resolved relationships for LSP to render as navigable links

### Changed

- **Hover Result**: Now includes `relationships: Vec<ResolvedRelationship>` with pre-resolved target locations
- **Symbol Extraction**: Extracts relationships from specialization, typing, subsetting, and other relationship constructs

## [0.2.0-alpha] - 2026-01-23

### 🚀 Major Rewrite — Salsa-based Incremental Architecture

This release represents a complete architectural rewrite, moving from an eager/imperative model to a query-based incremental computation system using [Salsa](https://github.com/salsa-rs/salsa).

### Added

- **Salsa Integration**: Full migration to Salsa for incremental, memoized queries
  - `RootDatabase` — The root Salsa database holding all query storage
  - `FileText` — Input query for raw source text
  - `SourceRootInput` — Input query for workspace file configuration
  - `parse_file` — Tracked query that parses source into AST
  - `file_symbols` — Query to extract HIR symbols from parsed AST
  - `file_symbols_from_text` — Combined parsing + symbol extraction query

- **Foundation Types** (`base` module):
  - `FileId` — Lightweight 4-byte interned file identifier (replaces `PathBuf` for O(1) comparisons)
  - `Name` — Interned identifier handle for O(1) string comparisons
  - `Interner` — Thread-safe string interner using `parking_lot` and `smol_str`
  - `TextRange`, `TextSize` — Source position types (re-exported from `text-size`)
  - `LineCol`, `LineIndex` — Line/column conversion utilities

- **Semantic IDs**:
  - `DefId` — Globally unique definition identifier (FileId + LocalDefId)
  - `LocalDefId` — File-local definition ID for efficient per-file invalidation

- **Input Management**:
  - `SourceRoot` — Workspace file registry with efficient insertion/removal

- **Anonymous scope naming**: Anonymous usages get unique qualified names using `<prefix#counter@Lline>` format
  - Relationship prefixes: `:>`, `:`, `:>:`, `:>>`, `about:`, `perform:`, `satisfy:`, `exhibit:`, `include:`, `assert:`, `verify:`, `ref:`, `meta:`, `crosses:`

- **Invocation expression reference extraction**: Function invocations like `EngineEvaluation_6cyl(...)` now extract the function name as a reference

- **Import link resolution for same-file packages**: Document links for imports use scope-aware `Resolver`

- **Implicit Supertypes**: All definitions now automatically inherit from their SysML kernel metaclass
  - `part def` → `Parts::Part`
  - `item def` → `Items::Item`
  - `action def` → `Actions::Action`
  - `state def` → `States::StateAction`
  - `constraint def` → `Constraints::ConstraintCheck`
  - `requirement def` → `Requirements::RequirementCheck`
  - `calc def` → `Calculations::Calculation`
  - `port def` → `Ports::Port`
  - `connection def` → `Connections::Connection`
  - `interface def` → `Interfaces::Interface`
  - `allocation def` → `Allocations::Allocation`
  - `use case def` → `UseCases::UseCase`
  - `analysis case def` → `AnalysisCases::AnalysisCase`
  - `attribute def` → `Attributes::AttributeValue`
  - Usage kinds: `flow` → `Flows::Message`, `connection` → `Connections::Connection`, etc.

- **Semantic Diagnostics System** (`diagnostics` module): Brand new semantic error reporting infrastructure
  - `Diagnostic` — Rich diagnostic type with file, span, severity, code, message, and related info
  - `Severity` — Error, Warning, Info, Hint levels with LSP conversion
  - `RelatedInfo` — Additional context linking to other source locations
  - `DiagnosticCollector` — Accumulator for diagnostics during analysis
  - `SemanticChecker` — Full semantic analysis engine that validates:
    - Undefined references (E0001)
    - Ambiguous references (E0002)
    - Type mismatches (E0003)
    - Duplicate definitions (E0004)
    - Missing required elements (E0005)
    - Invalid specialization (E0006)
    - Circular dependencies (E0007)
    - Unused symbols (W0001)
    - Deprecated usage (W0002)
    - Naming convention violations (W0003)
  - `check_file()` — Per-file semantic validation with duplicate detection
  - Deduplication in `finish()` — Filters duplicate diagnostics (same file, line, col, message)

### Changed

- **Complete HIR rewrite**: All semantic analysis now flows through Salsa queries
  - Automatic memoization — queries only re-run when inputs change
  - Automatic invalidation — change a file, only affected queries recompute
  - Parallel-safe — Salsa's design enables concurrent query execution

- **Memory efficiency**:
  - `FileId` (4 bytes) replaces `PathBuf` (~24+ bytes)
  - `Name` (4 bytes) for interned identifiers
  - `Arc<str>` for shared strings with reference counting

- `ExtractionContext` now includes `anon_counter: u32` and `next_anon_scope()` method

### Removed

- **Old `semantic` module**: Deleted the entire eager/imperative semantic analysis system
  - Removed `semantic/symbol_table/` — replaced by `hir::SymbolIndex`
  - Removed `semantic/workspace/` — replaced by Salsa database
  - Removed `semantic/adapters/` — replaced by `hir::symbols::extract_symbols_unified`
  - Removed `semantic/resolver/` — replaced by `hir::resolve::Resolver`
  - Removed `semantic/graphs/` — reference tracking now built into `SymbolIndex`

### Performance

- **Incremental parsing**: Only re-parse files that actually changed
- **Memoized symbol extraction**: Symbol extraction cached per-file
- **O(1) file/name comparisons**: Interned identifiers enable constant-time equality checks
- **Reduced memory pressure**: Shared string storage via interning

## [0.1.12-alpha] - 2025-01-30

### Added

- Initial feature chain resolution for SysML models
- Basic semantic analysis and name resolution
- HIR symbol extraction with type references
