#[path = "parser/file_io.rs"]
pub mod file_io;
#[path = "parser/kerml.rs"]
pub mod kerml;
#[path = "parser/keywords.rs"]
pub mod keywords;
#[path = "parser/result.rs"]
pub mod result;
#[path = "parser/sysml.rs"]
pub mod sysml;

// Re-export for convenience
pub use file_io::{get_extension, load_file, validate_extension};
pub use kerml::KerMLParser;
pub use result::{ParseError, ParseErrorKind, ParseResult};
pub use sysml::SysMLParser;
