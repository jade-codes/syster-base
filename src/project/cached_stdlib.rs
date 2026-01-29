//! Cached stdlib for fast test setup.
//!
//! The stdlib contains ~90+ files that need to be parsed and indexed. This is expensive
//! (~15-20 seconds per test). This module provides a cached version that parses and builds
//! the index once, then clones the entire AnalysisHost for each test.
//!
//! # Usage
//!
//! ```ignore
//! use syster::project::CachedStdLib;
//!
//! // First call parses and indexes stdlib (slow), subsequent calls clone (~100ms)
//! let host = CachedStdLib::analysis_host();
//! ```

use crate::ide::AnalysisHost;
use crate::project::file_loader;
use rayon::prelude::*;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock};

/// Pre-built AnalysisHost with stdlib fully indexed, wrapped in Arc for cheap cloning.
struct CachedHost {
    host: Arc<AnalysisHost>,
}

impl CachedHost {
    /// Parse all stdlib files, build index, and cache the result.
    fn load(stdlib_path: &PathBuf) -> Self {
        let mut host = AnalysisHost::new();

        if !stdlib_path.exists() || !stdlib_path.is_dir() {
            return Self {
                host: Arc::new(host),
            };
        }

        // Collect all file paths
        let file_paths = file_loader::collect_file_paths(stdlib_path).unwrap_or_default();

        // Parse files in parallel
        let files: Vec<_> = file_paths
            .par_iter()
            .filter_map(|path| {
                file_loader::load_and_parse(path)
                    .ok()
                    .map(|file| (path.clone(), file))
            })
            .collect();

        // Add all files to host
        for (path, file) in files {
            host.set_file(path, file);
        }

        // Build the index once (this is the expensive part!)
        host.mark_dirty();
        let _ = host.analysis(); // Forces index rebuild

        Self {
            host: Arc::new(host),
        }
    }
}

/// Discover the stdlib path (same logic as StdLibLoader).
fn discover_stdlib_path() -> PathBuf {
    use crate::base::constants::STDLIB_DIR;

    // Try next to the executable first
    if let Some(exe_dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
    {
        let stdlib_next_to_exe = exe_dir.join(STDLIB_DIR);
        if stdlib_next_to_exe.exists() && stdlib_next_to_exe.is_dir() {
            return stdlib_next_to_exe;
        }
    }

    // Try CARGO_MANIFEST_DIR for tests (base crate has stdlib in its directory)
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let stdlib_in_manifest = PathBuf::from(&manifest_dir).join(STDLIB_DIR);
        if stdlib_in_manifest.exists() && stdlib_in_manifest.is_dir() {
            return stdlib_in_manifest;
        }
        // For crates that depend on base, try parent directories
        let manifest_path = PathBuf::from(&manifest_dir);
        for ancestor in manifest_path.ancestors().skip(1).take(3) {
            let stdlib_in_ancestor = ancestor.join("base").join(STDLIB_DIR);
            if stdlib_in_ancestor.exists() && stdlib_in_ancestor.is_dir() {
                return stdlib_in_ancestor;
            }
        }
    }

    // Fall back to default
    PathBuf::from(STDLIB_DIR)
}

/// Global cached AnalysisHost with stdlib - built once on first access.
static CACHED_HOST: LazyLock<CachedHost> =
    LazyLock::new(|| CachedHost::load(&discover_stdlib_path()));

/// Cached standard library for fast test setup.
///
/// This module parses the stdlib and builds the index once, caching the entire
/// AnalysisHost in an Arc.
///
/// ## Performance Notes
///
/// - **Arc reference** (`analysis_host_arc()`): Instant (~1µs), for read-only access
/// - **Full clone** (`analysis_host()`): Slow (~15-20s), for tests that modify state
/// - **First access**: Builds cache (~15-20s), subsequent accesses reuse it
///
/// ## Usage Recommendations
///
/// - For read-only stdlib queries, use `analysis_host_arc()`
/// - For tests that add user files, use `analysis_host()` (pays clone cost once)
/// - For tests that don't need stdlib at all, don't use this module
pub struct CachedStdLib;

impl CachedStdLib {
    /// Get a clone of the cached AnalysisHost with stdlib fully indexed.
    ///
    /// **Warning:** This is slow (~15-20s) because it deep-clones all symbols.
    /// Use `analysis_host_arc()` for read-only access.
    pub fn analysis_host() -> AnalysisHost {
        (*CACHED_HOST.host).clone()
    }

    /// Get a reference to the cached AnalysisHost (for read-only operations).
    ///
    /// This is very fast (~1µs) as it just clones an Arc reference.
    /// Use this when you don't need to modify the host.
    pub fn analysis_host_arc() -> Arc<AnalysisHost> {
        CACHED_HOST.host.clone()
    }

    /// Load cached stdlib into an existing AnalysisHost.
    ///
    /// Note: This is less efficient than `analysis_host()` because the index
    /// will need to be rebuilt. Prefer `analysis_host()` for new hosts.
    pub fn load_into(host: &mut AnalysisHost) {
        // Clone all files from cached host
        for (path, file) in CACHED_HOST.host.files() {
            host.set_file(path.clone(), file.clone());
        }
        host.mark_dirty();
    }

    /// Get the number of cached files (for diagnostics).
    pub fn file_count() -> usize {
        CACHED_HOST.host.file_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cached_stdlib_loads_files() {
        let host = CachedStdLib::analysis_host();

        // Should have loaded many files (stdlib has ~90+ files)
        assert!(
            host.file_count() > 50,
            "Expected 50+ stdlib files, got {}",
            host.file_count()
        );
    }

    #[test]
    fn test_cached_stdlib_arc_is_fast() {
        use std::time::Instant;

        // First call may be slow (initializes cache)
        let _arc1 = CachedStdLib::analysis_host_arc();

        // Second call should be instant (just Arc clone)
        let start = Instant::now();
        let _arc2 = CachedStdLib::analysis_host_arc();
        let elapsed = start.elapsed();

        // Should complete in under 1ms
        assert!(
            elapsed.as_micros() < 1000,
            "Arc clone took {:?}, expected <1ms",
            elapsed
        );
    }

    #[test]
    fn test_cached_host_has_built_index() {
        let mut host = CachedStdLib::analysis_host();

        // Index should already be built (not dirty)
        let analysis = host.analysis();
        let symbol_count = analysis.symbol_index().all_symbols().count();

        // Should have many symbols from stdlib
        assert!(
            symbol_count > 100,
            "Expected 100+ symbols, got {}",
            symbol_count
        );
    }

    #[test]
    fn test_cached_stdlib_file_count() {
        // Check the static count
        assert!(CachedStdLib::file_count() > 50, "Expected 50+ files");
    }
}
