pub mod cached_stdlib;
pub mod file_loader;
pub mod stdlib_loader;
pub mod workspace_loader;

pub use cached_stdlib::CachedStdLib;
pub use stdlib_loader::StdLibLoader;
pub use workspace_loader::WorkspaceLoader;

// Re-export parse utilities from syntax layer
pub use crate::syntax::parser::{get_extension, load_file, validate_extension};

// Re-export language parsing convenience function
pub use crate::syntax::parser::parse_with_result;
