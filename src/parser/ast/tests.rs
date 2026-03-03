use super::*;
use crate::parser::parse_sysml;

#[test]
fn test_ast_package() {
    let parsed = parse_sysml("package Test;");
    let root = SourceFile::cast(parsed.syntax()).unwrap();

    let members: Vec<_> = root.members().collect();
    assert_eq!(members.len(), 1);

    if let NamespaceMember::Package(pkg) = &members[0] {
        let name = pkg.name().unwrap();
        assert_eq!(name.text(), Some("Test".to_string()));
    } else {
        panic!("expected Package");
    }
}

#[test]
fn test_ast_import() {
    let parsed = parse_sysml("import ISQ::*;");
    let root = SourceFile::cast(parsed.syntax()).unwrap();

    let members: Vec<_> = root.members().collect();
    assert_eq!(members.len(), 1);

    if let NamespaceMember::Import(imp) = &members[0] {
        assert!(!imp.is_all());
        assert!(imp.is_wildcard());
        assert!(!imp.is_recursive());
        let target = imp.target().unwrap();
        assert_eq!(target.segments(), vec!["ISQ"]);
    } else {
        panic!("expected Import");
    }
}

#[test]
fn test_ast_import_recursive() {
    let parsed = parse_sysml("import all Library::**;");
    assert!(parsed.ok(), "errors: {:?}", parsed.errors);

    let root = SourceFile::cast(parsed.syntax()).unwrap();

    let members: Vec<_> = root.members().collect();
    if let NamespaceMember::Import(imp) = &members[0] {
        assert!(imp.is_all());
        assert!(imp.is_recursive());
    } else {
        panic!("expected Import");
    }
}

#[test]
fn test_ast_definition() {
    let parsed = parse_sysml("abstract part def Vehicle :> Base;");
    let root = SourceFile::cast(parsed.syntax()).unwrap();

    let members: Vec<_> = root.members().collect();
    if let NamespaceMember::Definition(def) = &members[0] {
        assert!(def.is_abstract());
        assert_eq!(def.definition_kind(), Some(DefinitionKind::Part));
        let name = def.name().unwrap();
        assert_eq!(name.text(), Some("Vehicle".to_string()));

        let specializations: Vec<_> = def.specializations().collect();
        assert_eq!(specializations.len(), 1);
        assert_eq!(
            specializations[0].kind(),
            Some(SpecializationKind::Specializes)
        );
    } else {
        panic!("expected Definition");
    }
}

#[test]
fn test_ast_usage() {
    let parsed = parse_sysml("ref part engine : Engine;");
    let root = SourceFile::cast(parsed.syntax()).unwrap();

    let members: Vec<_> = root.members().collect();
    if let NamespaceMember::Usage(usage) = &members[0] {
        assert!(usage.is_ref());
        let name = usage.name().unwrap();
        assert_eq!(name.text(), Some("engine".to_string()));

        let typing = usage.typing().unwrap();
        let target = typing.target().unwrap();
        assert_eq!(target.segments(), vec!["Engine"]);
    } else {
        panic!("expected Usage");
    }
}

#[test]
fn test_message_usage_name() {
    // Test that message usages extract names correctly
    // Message usages need to be inside a package/part body
    let parsed = parse_sysml("part p { message of ignitionCmd : IgnitionCmd; }");
    let root = SourceFile::cast(parsed.syntax()).unwrap();

    let members: Vec<_> = root.members().collect();

    // Get the usage inside the part
    if let NamespaceMember::Usage(part_usage) = &members[0] {
        if let Some(body) = part_usage.body() {
            let inner_members: Vec<_> = body.members().collect();
            if let NamespaceMember::Usage(usage) = &inner_members[0] {
                let name = usage.name();
                assert!(name.is_some(), "message usage should have a name");
                assert_eq!(name.unwrap().text(), Some("ignitionCmd".to_string()));
                return;
            }
        }
    }
    panic!("expected Usage for part p with message inside");
}
