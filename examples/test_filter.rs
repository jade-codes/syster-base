use syster::parser::parse_sysml;
use std::fs;

fn main() {
    let source = fs::read_to_string("tests/sysml-examples/Simple Tests/PartTest.sysml").unwrap();
    let parsed = parse_sysml(&source);
    
    if !parsed.errors.is_empty() {
        for err in &parsed.errors {
            let pos: u32 = err.range.start().into();
            let line_num = source[..pos as usize].chars().filter(|c| *c == '\n').count() + 1;
            let line_start = source[..pos as usize].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let line_end = source[pos as usize..].find('\n').map(|i| pos as usize + i).unwrap_or(source.len());
            let line_content = &source[line_start..line_end];
            println!("Error at line {}: {}", line_num, err.message);
            println!("  {}", line_content);
            println!();
        }
    } else {
        println!("No errors!");
    }
}
