//! Converter from rowan CST to the existing SysML/KerML AST types.
//!
//! This module bridges the new rowan-based parser with the existing
//! AST infrastructure (SysMLFile, KerMLFile, etc.) enabling a gradual migration.
//!
//! Requires `pest-parser` feature since it converts to Pest AST types.

#![cfg(feature = "pest-parser")]

use super::ast::{self, AstNode, NamespaceMember, SourceFile};
use super::{parse_kerml, parse_sysml, Parse, SyntaxNode};
use crate::parser::ParseError;
use crate::syntax::sysml::ast::{
    Alias as SysMLAlias, Comment as SysMLComment, Definition as SysMLDefinition,
    DefinitionKind as SysMLDefinitionKind, Dependency as SysMLDependency, Element,
    Filter as SysMLFilter, Import as SysMLImport, NamespaceDeclaration, Package as SysMLPackage,
    SysMLFile, Usage as SysMLUsage, UsageKind as SysMLUsageKind, Relationships,
};
use crate::syntax::kerml::ast::{
    KerMLFile, NamespaceDeclaration as KerMLNamespace, Element as KerMLElement, 
    Classifier as KerMLClassifier, ClassifierKind as KerMLClassifierKind,
    Feature as KerMLFeature, Import as KerMLImport, ImportKind as KerMLImportKind,
    Package as KerMLPackage,
};
use crate::syntax::Span;
use crate::base::position::Position;
use rowan::TextRange;
use std::path::Path;

/// Result of converting rowan output
pub struct ConvertResult<T> {
    pub content: Option<T>,
    pub errors: Vec<ParseError>,
}

impl<T> ConvertResult<T> {
    pub fn success(content: T) -> Self {
        Self {
            content: Some(content),
            errors: vec![],
        }
    }

    pub fn with_errors(errors: Vec<ParseError>) -> Self {
        Self {
            content: None,
            errors,
        }
    }
}

/// Convert rowan TextRange to our Span type
fn range_to_span(range: TextRange, source: &str) -> Span {
    let start_offset: usize = range.start().into();
    let end_offset: usize = range.end().into();
    
    // Calculate line and column from offset
    let (start_line, start_col) = offset_to_line_col(source, start_offset);
    let (end_line, end_col) = offset_to_line_col(source, end_offset);
    
    Span {
        start: Position::new(start_line, start_col),
        end: Position::new(end_line, end_col),
    }
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 0;
    let mut col = 0;
    for (i, c) in source.char_indices() {
        if i >= offset {
            break;
        }
        if c == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    (line, col)
}

// ============================================================================
// SysML Conversion
// ============================================================================

/// Parse SysML content using rowan and convert to SysMLFile
pub fn parse_sysml_to_ast(content: &str, path: &Path) -> ConvertResult<SysMLFile> {
    let parse = parse_sysml(content);
    
    // Convert syntax errors to ParseErrors
    let errors: Vec<ParseError> = parse
        .errors
        .iter()
        .map(|e| {
            let (line, col) = offset_to_line_col(content, e.range.start().into());
            ParseError::syntax_error(e.message.clone(), line, col)
        })
        .collect();
    
    // Even with errors, try to convert what we have
    let source = SourceFile::cast(parse.syntax()).expect("root should be SOURCE_FILE");
    
    let file = convert_source_to_sysml(source, content);
    
    ConvertResult {
        content: Some(file),
        errors,
    }
}

fn convert_source_to_sysml(source: SourceFile, content: &str) -> SysMLFile {
    let mut namespace: Option<NamespaceDeclaration> = None;
    let mut namespaces: Vec<NamespaceDeclaration> = Vec::new();
    let mut elements: Vec<Element> = Vec::new();
    
    for member in source.members() {
        match member {
            NamespaceMember::Package(pkg) => {
                let converted = convert_package(&pkg, content);
                
                // First package becomes the namespace
                if namespace.is_none() {
                    if let Some(ref name) = converted.name {
                        namespace = Some(NamespaceDeclaration {
                            name: name.clone(),
                            span: converted.span.clone(),
                        });
                        namespaces.push(NamespaceDeclaration {
                            name: name.clone(),
                            span: converted.span.clone(),
                        });
                    }
                } else if let Some(ref name) = converted.name {
                    namespaces.push(NamespaceDeclaration {
                        name: name.clone(),
                        span: converted.span.clone(),
                    });
                }
                
                elements.push(Element::Package(converted));
            }
            NamespaceMember::LibraryPackage(pkg) => {
                let converted = convert_library_package(&pkg, content);
                
                if namespace.is_none() {
                    if let Some(ref name) = converted.name {
                        namespace = Some(NamespaceDeclaration {
                            name: name.clone(),
                            span: converted.span.clone(),
                        });
                        namespaces.push(NamespaceDeclaration {
                            name: name.clone(),
                            span: converted.span.clone(),
                        });
                    }
                }
                
                elements.push(Element::Package(converted));
            }
            NamespaceMember::Import(imp) => {
                elements.push(Element::Import(convert_import(&imp, content)));
            }
            NamespaceMember::Alias(alias) => {
                elements.push(Element::Alias(convert_alias(&alias, content)));
            }
            NamespaceMember::Definition(def) => {
                elements.push(Element::Definition(convert_definition(&def, content)));
            }
            NamespaceMember::Usage(usage) => {
                elements.push(Element::Usage(convert_usage(&usage, content)));
            }
            NamespaceMember::Dependency(dep) => {
                elements.push(Element::Dependency(convert_dependency(&dep, content)));
            }
            NamespaceMember::Filter(filter) => {
                elements.push(Element::Filter(convert_filter(&filter, content)));
            }
            NamespaceMember::Metadata(_meta) => {
                // TODO: Convert metadata
            }
            NamespaceMember::Comment(comment) => {
                elements.push(Element::Comment(convert_comment(&comment, content)));
            }
        }
    }
    
    SysMLFile {
        namespace,
        namespaces,
        elements,
    }
}

fn convert_package(pkg: &ast::Package, content: &str) -> SysMLPackage {
    let name = pkg.name().map(|n| n.text().to_string());
    let span = pkg.name().map(|n| range_to_span(n.syntax().text_range(), content));
    
    let mut elements = Vec::new();
    for member in pkg.members() {
        if let Some(elem) = convert_member_to_element(&member, content) {
            elements.push(elem);
        }
    }
    
    SysMLPackage {
        name,
        short_name: None, // TODO: Extract short name
        elements,
        span,
    }
}

fn convert_library_package(pkg: &ast::LibraryPackage, content: &str) -> SysMLPackage {
    let name = pkg.name().map(|n| n.text().to_string());
    let span = pkg.name().map(|n| range_to_span(n.syntax().text_range(), content));
    
    let mut elements = Vec::new();
    if let Some(body) = pkg.body() {
        for member in body.members() {
            if let Some(elem) = convert_member_to_element(&member, content) {
                elements.push(elem);
            }
        }
    }
    
    SysMLPackage {
        name,
        short_name: None,
        elements,
        span,
    }
}

fn convert_member_to_element(member: &NamespaceMember, content: &str) -> Option<Element> {
    match member {
        NamespaceMember::Package(pkg) => Some(Element::Package(convert_package(pkg, content))),
        NamespaceMember::LibraryPackage(pkg) => Some(Element::Package(convert_library_package(pkg, content))),
        NamespaceMember::Import(imp) => Some(Element::Import(convert_import(imp, content))),
        NamespaceMember::Alias(alias) => Some(Element::Alias(convert_alias(alias, content))),
        NamespaceMember::Definition(def) => Some(Element::Definition(convert_definition(def, content))),
        NamespaceMember::Usage(usage) => Some(Element::Usage(convert_usage(usage, content))),
        NamespaceMember::Dependency(dep) => Some(Element::Dependency(convert_dependency(dep, content))),
        NamespaceMember::Filter(filter) => Some(Element::Filter(convert_filter(filter, content))),
        NamespaceMember::Comment(comment) => Some(Element::Comment(convert_comment(comment, content))),
        NamespaceMember::Metadata(_) => None, // TODO
    }
}

fn convert_import(imp: &ast::Import, content: &str) -> SysMLImport {
    let target_name = imp.target().map(|t| t.text()).unwrap_or_default();
    let span = imp.target().map(|t| range_to_span(t.syntax().text_range(), content));
    
    // Determine import path
    let path = if imp.is_recursive() {
        format!("{}::**", target_name)
    } else if imp.is_wildcard() {
        format!("{}::*", target_name)
    } else {
        target_name.to_string()
    };
    
    SysMLImport {
        path,
        is_all: imp.is_all(),
        is_recursive: imp.is_recursive(),
        is_wildcard: imp.is_wildcard(),
        alias: None,
        span,
        filter: imp.filter().and_then(|f| f.target()).map(|t| t.text()),
    }
}

fn convert_alias(alias: &ast::Alias, content: &str) -> SysMLAlias {
    let name = alias.name().map(|n| n.text().to_string()).unwrap_or_default();
    let target = alias.target().map(|t| t.text()).unwrap_or_default();
    let span = alias.name().map(|n| range_to_span(n.syntax().text_range(), content));
    
    SysMLAlias {
        name,
        target,
        span,
    }
}

fn convert_definition(def: &ast::Definition, content: &str) -> SysMLDefinition {
    let name = def.name().map(|n| n.text().to_string());
    let span = def.name().map(|n| range_to_span(n.syntax().text_range(), content));
    
    let kind = def.definition_kind().map(|k| convert_definition_kind(k)).unwrap_or_default();
    
    // Convert members
    let mut members = Vec::new();
    for member in def.members() {
        match member {
            NamespaceMember::Usage(u) => {
                let usage = convert_usage(&u, content);
                members.push(crate::syntax::sysml::ast::DefinitionMember::Usage(Box::new(usage)));
            }
            NamespaceMember::Import(i) => {
                let import = convert_import(&i, content);
                members.push(crate::syntax::sysml::ast::DefinitionMember::Import(Box::new(import)));
            }
            NamespaceMember::Comment(c) => {
                let comment = convert_comment(&c, content);
                members.push(crate::syntax::sysml::ast::DefinitionMember::Comment(Box::new(comment)));
            }
            _ => {} // Skip other member types for now
        }
    }
    
    // Extract relationships from the definition
    let relationships = extract_relationships(def, content);
    
    SysMLDefinition {
        kind,
        name,
        short_name: None, // TODO: Extract short name
        is_abstract: def.is_abstract(),
        is_variation: def.is_variation(),
        is_individual: false, // TODO
        relationships,
        members,
        span,
    }
}

fn convert_definition_kind(kind: ast::DefinitionKind) -> SysMLDefinitionKind {
    match kind {
        ast::DefinitionKind::Part => SysMLDefinitionKind::Part,
        ast::DefinitionKind::Attribute => SysMLDefinitionKind::Attribute,
        ast::DefinitionKind::Port => SysMLDefinitionKind::Port,
        ast::DefinitionKind::Item => SysMLDefinitionKind::Item,
        ast::DefinitionKind::Action => SysMLDefinitionKind::Action,
        ast::DefinitionKind::State => SysMLDefinitionKind::State,
        ast::DefinitionKind::Constraint => SysMLDefinitionKind::Constraint,
        ast::DefinitionKind::Requirement => SysMLDefinitionKind::Requirement,
        ast::DefinitionKind::Case => SysMLDefinitionKind::Case,
        ast::DefinitionKind::Calc => SysMLDefinitionKind::Calculation,
        ast::DefinitionKind::Connection => SysMLDefinitionKind::Connection,
        ast::DefinitionKind::Interface => SysMLDefinitionKind::Interface,
        ast::DefinitionKind::Allocation => SysMLDefinitionKind::Allocation,
        ast::DefinitionKind::Flow => SysMLDefinitionKind::Flow,
        ast::DefinitionKind::View => SysMLDefinitionKind::View,
        ast::DefinitionKind::Viewpoint => SysMLDefinitionKind::Viewpoint,
        ast::DefinitionKind::Rendering => SysMLDefinitionKind::Rendering,
        ast::DefinitionKind::Metadata => SysMLDefinitionKind::Metadata,
        ast::DefinitionKind::Occurrence => SysMLDefinitionKind::Occurrence,
        ast::DefinitionKind::Enum => SysMLDefinitionKind::Enumeration,
        ast::DefinitionKind::Analysis => SysMLDefinitionKind::AnalysisCase,
        ast::DefinitionKind::Verification => SysMLDefinitionKind::VerificationCase,
        ast::DefinitionKind::UseCase => SysMLDefinitionKind::UseCase,
        ast::DefinitionKind::Concern => SysMLDefinitionKind::Concern,
        ast::DefinitionKind::Individual => SysMLDefinitionKind::Individual,
    }
}

fn convert_usage(usage: &ast::Usage, content: &str) -> SysMLUsage {
    let name = usage.name().map(|n| n.text().to_string());
    let span = usage.name().map(|n| range_to_span(n.syntax().text_range(), content));
    
    let kind = usage.usage_kind().map(|k| convert_usage_kind(k)).unwrap_or_default();
    
    // Convert nested members
    let mut members = Vec::new();
    for member in usage.members() {
        match member {
            NamespaceMember::Usage(u) => {
                let nested = convert_usage(&u, content);
                members.push(crate::syntax::sysml::ast::UsageMember::Usage(Box::new(nested)));
            }
            NamespaceMember::Import(i) => {
                let import = convert_import(&i, content);
                members.push(crate::syntax::sysml::ast::UsageMember::Import(Box::new(import)));
            }
            NamespaceMember::Comment(c) => {
                let comment = convert_comment(&c, content);
                members.push(crate::syntax::sysml::ast::UsageMember::Comment(Box::new(comment)));
            }
            _ => {}
        }
    }
    
    // Extract relationships
    let relationships = extract_usage_relationships(usage, content);
    
    SysMLUsage {
        kind,
        name,
        short_name: None,
        is_variation: usage.is_variation(),
        is_ref: usage.is_ref(),
        relationships,
        members,
        span,
        value: None, // TODO: Extract value expression
        value_span: None,
        multiplicity: None, // TODO: Extract multiplicity
        direction: None, // TODO: Extract direction
    }
}

fn convert_usage_kind(kind: ast::UsageKind) -> SysMLUsageKind {
    match kind {
        ast::UsageKind::Part => SysMLUsageKind::Part,
        ast::UsageKind::Attribute => SysMLUsageKind::Attribute,
        ast::UsageKind::Port => SysMLUsageKind::Port,
        ast::UsageKind::Item => SysMLUsageKind::Item,
        ast::UsageKind::Action => SysMLUsageKind::Action,
        ast::UsageKind::State => SysMLUsageKind::State { is_parallel: false },
        ast::UsageKind::Constraint => SysMLUsageKind::Constraint,
        ast::UsageKind::Requirement => SysMLUsageKind::Requirement,
        ast::UsageKind::Case => SysMLUsageKind::Case,
        ast::UsageKind::Calc => SysMLUsageKind::Calculation,
        ast::UsageKind::Connection => SysMLUsageKind::Connection,
        ast::UsageKind::Interface => SysMLUsageKind::Interface,
        ast::UsageKind::Allocation => SysMLUsageKind::Allocation,
        ast::UsageKind::Flow => SysMLUsageKind::Flow,
        ast::UsageKind::View => SysMLUsageKind::View,
        ast::UsageKind::Viewpoint => SysMLUsageKind::Viewpoint,
        ast::UsageKind::Rendering => SysMLUsageKind::Rendering,
        ast::UsageKind::Occurrence => SysMLUsageKind::Occurrence,
        ast::UsageKind::Enum => SysMLUsageKind::Enumeration,
        ast::UsageKind::Concern => SysMLUsageKind::Concern,
        ast::UsageKind::Ref => SysMLUsageKind::Reference,
        ast::UsageKind::Individual => SysMLUsageKind::Individual,
        ast::UsageKind::Message => SysMLUsageKind::Message,
        ast::UsageKind::Event => SysMLUsageKind::Event,
    }
}

fn convert_dependency(dep: &ast::Dependency, content: &str) -> SysMLDependency {
    let span = Some(range_to_span(dep.syntax().text_range(), content));
    
    SysMLDependency {
        name: None,
        client: String::new(),
        supplier: String::new(),
        span,
    }
}

fn convert_filter(filter: &ast::ElementFilter, content: &str) -> SysMLFilter {
    let span = Some(range_to_span(filter.syntax().text_range(), content));
    
    SysMLFilter {
        expression: String::new(), // TODO: Extract expression
        span,
    }
}

fn convert_comment(comment: &ast::Comment, content: &str) -> SysMLComment {
    let span = Some(range_to_span(comment.syntax().text_range(), content));
    
    // Extract comment text from the syntax node
    let text = comment
        .syntax()
        .children_with_tokens()
        .filter_map(|e| e.into_token())
        .filter(|t| {
            matches!(
                t.kind(),
                super::syntax_kind::SyntaxKind::STRING_LITERAL
                    | super::syntax_kind::SyntaxKind::MULTILINE_STRING
            )
        })
        .map(|t| t.text().to_string())
        .next()
        .unwrap_or_default();
    
    SysMLComment {
        text,
        about: None,
        span,
    }
}

fn extract_relationships(def: &ast::Definition, content: &str) -> Relationships {
    use crate::syntax::sysml::ast::parsers::ExtractedRef;
    
    let mut relationships = Relationships::default();
    
    // Extract typed_by
    if let Some(typing) = def.typing() {
        if let Some(target) = typing.target() {
            relationships.typed_by = Some(target.text());
            relationships.typed_by_span = Some(range_to_span(target.syntax().text_range(), content));
        }
    }
    
    // Extract specializations
    for spec in def.specializations() {
        if let Some(target) = spec.target() {
            let extracted = ExtractedRef::Simple {
                name: target.text(),
                span: Some(range_to_span(target.syntax().text_range(), content)),
            };
            relationships.specializes.push(crate::syntax::sysml::ast::SpecializationRel::new(extracted));
        }
    }
    
    relationships
}

fn extract_usage_relationships(usage: &ast::Usage, content: &str) -> Relationships {
    use crate::syntax::sysml::ast::parsers::ExtractedRef;
    
    let mut relationships = Relationships::default();
    
    // Extract typed_by
    if let Some(typing) = usage.typing() {
        if let Some(target) = typing.target() {
            relationships.typed_by = Some(target.text());
            relationships.typed_by_span = Some(range_to_span(target.syntax().text_range(), content));
        }
    }
    
    // Extract specializations
    for spec in usage.specializations() {
        if let Some(target) = spec.target() {
            let extracted = ExtractedRef::Simple {
                name: target.text(),
                span: Some(range_to_span(target.syntax().text_range(), content)),
            };
            relationships.specializes.push(crate::syntax::sysml::ast::SpecializationRel::new(extracted));
        }
    }
    
    relationships
}

// ============================================================================
// KerML Conversion
// ============================================================================

/// Parse KerML content using rowan and convert to KerMLFile
pub fn parse_kerml_to_ast(content: &str, _path: &Path) -> ConvertResult<KerMLFile> {
    let parse = parse_kerml(content);
    
    // Convert syntax errors to ParseErrors
    let errors: Vec<ParseError> = parse
        .errors
        .iter()
        .map(|e| {
            let (line, col) = offset_to_line_col(content, e.range.start().into());
            ParseError::syntax_error(e.message.clone(), line, col)
        })
        .collect();
    
    let source = SourceFile::cast(parse.syntax()).expect("root should be SOURCE_FILE");
    
    let file = convert_source_to_kerml(source, content);
    
    ConvertResult {
        content: Some(file),
        errors,
    }
}

fn convert_source_to_kerml(source: SourceFile, content: &str) -> KerMLFile {
    let mut namespace: Option<KerMLNamespace> = None;
    let mut elements: Vec<KerMLElement> = Vec::new();
    
    for member in source.members() {
        match member {
            NamespaceMember::Package(pkg) => {
                if namespace.is_none() {
                    if let Some(name) = pkg.name() {
                        namespace = Some(KerMLNamespace {
                            name: name.text().to_string(),
                            span: Some(range_to_span(name.syntax().text_range(), content)),
                        });
                    }
                }
                // Convert package elements
                for inner in pkg.members() {
                    if let Some(elem) = convert_kerml_member(&inner, content) {
                        elements.push(elem);
                    }
                }
            }
            NamespaceMember::LibraryPackage(pkg) => {
                if namespace.is_none() {
                    if let Some(name) = pkg.name() {
                        namespace = Some(KerMLNamespace {
                            name: name.text().to_string(),
                            span: Some(range_to_span(name.syntax().text_range(), content)),
                        });
                    }
                }
            }
            NamespaceMember::Import(imp) => {
                elements.push(KerMLElement::Import(convert_kerml_import(&imp, content)));
            }
            NamespaceMember::Definition(def) => {
                elements.push(KerMLElement::Classifier(convert_kerml_definition(&def, content)));
            }
            NamespaceMember::Usage(usage) => {
                elements.push(KerMLElement::Feature(convert_kerml_usage(&usage, content)));
            }
            _ => {}
        }
    }
    
    KerMLFile {
        namespace,
        elements,
    }
}

fn convert_kerml_member(member: &NamespaceMember, content: &str) -> Option<KerMLElement> {
    match member {
        NamespaceMember::Import(imp) => Some(KerMLElement::Import(convert_kerml_import(imp, content))),
        NamespaceMember::Definition(def) => Some(KerMLElement::Classifier(convert_kerml_definition(def, content))),
        NamespaceMember::Usage(usage) => Some(KerMLElement::Feature(convert_kerml_usage(usage, content))),
        _ => None,
    }
}

fn convert_kerml_import(imp: &ast::Import, content: &str) -> KerMLImport {
    let target_name = imp.target().map(|t| t.text()).unwrap_or_default();
    let path_span = imp.target().map(|t| range_to_span(t.syntax().text_range(), content));
    let span = Some(range_to_span(imp.syntax().text_range(), content));
    
    let path = if imp.is_recursive() {
        format!("{}::**", target_name)
    } else if imp.is_wildcard() {
        format!("{}::*", target_name)
    } else {
        target_name.to_string()
    };
    
    KerMLImport {
        path,
        path_span,
        is_recursive: imp.is_recursive(),
        is_public: false, // TODO: detect public modifier
        kind: if imp.is_all() {
            KerMLImportKind::All
        } else {
            KerMLImportKind::Normal
        },
        span,
    }
}

fn convert_kerml_definition(def: &ast::Definition, content: &str) -> KerMLClassifier {
    let name = def.name().map(|n| n.text().to_string());
    let span = def.name().map(|n| range_to_span(n.syntax().text_range(), content));
    
    // Map definition kind to KerML ClassifierKind
    let kind = def.definition_kind().map(|k| match k {
        ast::DefinitionKind::Part | ast::DefinitionKind::Item => KerMLClassifierKind::Class,
        ast::DefinitionKind::Attribute => KerMLClassifierKind::DataType,
        ast::DefinitionKind::Action => KerMLClassifierKind::Behavior,
        ast::DefinitionKind::Calc => KerMLClassifierKind::Function,
        _ => KerMLClassifierKind::Classifier,
    }).unwrap_or(KerMLClassifierKind::Type);
    
    KerMLClassifier {
        kind,
        is_abstract: def.is_abstract(),
        name,
        body: vec![], // TODO: convert members
        span,
    }
}

fn convert_kerml_usage(usage: &ast::Usage, content: &str) -> KerMLFeature {
    let name = usage.name().map(|n| n.text().to_string());
    let span = usage.name().map(|n| range_to_span(n.syntax().text_range(), content));
    
    KerMLFeature {
        name,
        direction: None, // TODO: extract direction
        is_const: false,
        is_derived: false,
        body: vec![], // TODO: convert members
        span,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_sysml_simple_package() {
        let content = r#"
            package Vehicle {
                part def Engine;
                part engine : Engine;
            }
        "#;
        let path = PathBuf::from("test.sysml");
        let result = parse_sysml_to_ast(content, &path);
        
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        let file = result.content.unwrap();
        assert_eq!(file.namespace.as_ref().map(|n| n.name.as_str()), Some("Vehicle"));
        assert_eq!(file.elements.len(), 1); // Just the package
    }

    #[test]
    fn test_parse_sysml_with_imports() {
        let content = r#"
            package Test {
                import ISQ::*;
                import SI::*;
                part def Component;
            }
        "#;
        let path = PathBuf::from("test.sysml");
        let result = parse_sysml_to_ast(content, &path);
        
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        let file = result.content.unwrap();
        assert!(file.namespace.is_some());
    }

    #[test]
    fn test_parse_kerml_simple() {
        let content = r#"
            package Types {
                class Vehicle;
                feature mass : Real;
            }
        "#;
        let path = PathBuf::from("test.kerml");
        let result = parse_kerml_to_ast(content, &path);
        
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        let file = result.content.unwrap();
        assert!(file.namespace.is_some());
    }
}
