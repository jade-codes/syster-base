use syster::parser::{AstNode, NamespaceMember, parse_sysml};

fn main() {
    let source = r#"package PictureTaking {
    part def Exposure;
    
    action def Focus { out xrsl: Exposure; }
    action def Shoot { in xsf: Exposure; } 
        
    action takePicture {
        action focus: Focus[1];
        flow of Exposure from focus.xrsl to shoot.xsf;
        action shoot: Shoot[1];
    }
}"#;

    let tree = parse_sysml(source);
    println!("Parse errors: {:?}", tree.errors);

    // Find the flow statement
    for root_node in tree.syntax().children() {
        if let Some(member) = NamespaceMember::cast(root_node) {
            println!("Top: {:?}", member);
            if let Some(body) = match &member {
                NamespaceMember::Package(p) => p.body(),
                _ => None,
            } {
                for m in body.members() {
                    println!("  Member: {:?}", m);
                    // Check for usage with name "takePicture" (it's an action usage, not definition)
                    if let NamespaceMember::Usage(usage) = &m {
                        let name = usage.name().and_then(|n| n.text());
                        println!("    Usage name: {:?}", name);
                        if name.as_deref() == Some("takePicture") {
                            // Action usages have body via action_body()
                            if let Some(b) = usage.body() {
                                println!("    Has body with {} members", b.members().count());
                                for inner in b.members() {
                                    println!("      Inner: {:?}", inner);
                                    if let NamespaceMember::Usage(inner_usage) = &inner {
                                        let uname = inner_usage.name().and_then(|n| n.text());
                                        println!("        Inner usage name: {:?}", uname);

                                        // Debug: print the raw syntax tree
                                        if uname.is_none() {
                                            println!(
                                                "        RAW SYNTAX: {:?}",
                                                inner_usage.syntax()
                                            );
                                            for child in inner_usage.syntax().children_with_tokens()
                                            {
                                                println!("          child: {:?}", child);
                                            }
                                        }

                                        // Check for from_to_clause on the anonymous flow
                                        if let Some(ftc) = inner_usage.from_to_clause() {
                                            println!("        FROM TO: {:?}", ftc.syntax());
                                            if let Some(src) = ftc.source() {
                                                println!(
                                                    "          Source: {:?}",
                                                    src.target().map(|t| t.to_string())
                                                );
                                                if let Some(qn) = src.target() {
                                                    println!(
                                                        "          Source segments_with_ranges: {:?}",
                                                        qn.segments_with_ranges()
                                                    );
                                                }
                                            }
                                            if let Some(tgt) = ftc.target() {
                                                println!(
                                                    "          Target: {:?}",
                                                    tgt.target().map(|t| t.to_string())
                                                );
                                                if let Some(qn) = tgt.target() {
                                                    println!(
                                                        "          Target segments_with_ranges: {:?}",
                                                        qn.segments_with_ranges()
                                                    );
                                                }
                                            }
                                        } else {
                                            println!("        NO from_to_clause!");
                                        }
                                    }
                                }
                            } else {
                                println!("    No body found!");
                            }
                        }
                    }
                }
            }
        }
    }
}
