use std::path::PathBuf;
use syster::semantic::Workspace;
use syster::syntax::file::SyntaxFile;
use syster::syntax::sysml::ast::parse_file;
use syster::parser::sysml::{SysMLParser, Rule};
use pest::Parser;

fn main() {
    let source = r#"package Test {
    use case transportPassenger_1{
        action trigger accept ignitionCmd:IgnitionCmd;
        first join1 then trigger;
    }
}"#;

    println!("=== PARSING ACCEPT ACTION ===\n");
    println!("SOURCE:\n{}\n", source);
    
    // First, let's see the parse tree
    println!("=== PARSE TREE ===");
    let pairs = SysMLParser::parse(Rule::file, source).expect("Parse should succeed");
    fn print_tree(pair: &pest::iterators::Pair<Rule>, indent: usize) {
        let spaces = "  ".repeat(indent);
        let text: String = pair.as_str().chars().take(50).collect();
        if matches!(pair.as_rule(), Rule::action_usage | Rule::accept_node | Rule::identifier | Rule::behavior_usage_member | Rule::case_action_body_item | Rule::action_node_member | Rule::action_node) {
            println!("{}{:?} '{}'", spaces, pair.as_rule(), text.replace('\n', "\\n"));
        }
        for inner in pair.clone().into_inner() {
            print_tree(&inner, indent + 1);
        }
    }
    for pair in pairs.clone() {
        print_tree(&pair, 0);
    }
    
    let mut workspace: Workspace<SyntaxFile> = Workspace::new();
    let mut pairs2 = SysMLParser::parse(Rule::file, source).expect("Parse should succeed");
    let sysml_file = parse_file(&mut pairs2).expect("AST parse should succeed");
    let syntax_file = SyntaxFile::SysML(sysml_file);
    workspace.add_file(PathBuf::from("/test.sysml"), syntax_file);
    workspace.populate_all().expect("Should populate");
    
    println!("\n=== ALL SYMBOLS ===");
    for sym in workspace.symbol_table().iter_symbols() {
        println!("  {}", sym.qualified_name());
    }
    
    println!("\n=== ALL REFERENCES ===");
    let ref_index = workspace.reference_index();
    for target in ref_index.targets() {
        let refs = ref_index.get_references(target);
        for r in refs {
            println!("  line={} col={}-{} target='{}' source='{}'",
                   r.span.start.line, r.span.start.column, r.span.end.column,
                   target, r.source_qname);
        }
    }
}
