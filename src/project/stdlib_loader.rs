mod loader;

use crate::base::constants::STDLIB_DIR;
use crate::ide::AnalysisHost;
use std::path::PathBuf;

/// Loads the standard library from /sysml.lib/ at startup
pub struct StdLibLoader {
    stdlib_path: PathBuf,
    /// Track if stdlib has been loaded (for lazy loading)
    loaded: bool,
}

impl StdLibLoader {
    /// Creates a new stdlib loader with automatic path discovery.
    ///
    /// Searches for the stdlib in these locations (in order):
    /// 1. Next to the current executable (for installed binaries)
    /// 2. Current working directory (for development)
    pub fn new() -> Self {
        Self {
            stdlib_path: Self::discover_path(),
            loaded: false,
        }
    }

    /// Creates a new stdlib loader with a specific path.
    pub fn with_path(path: PathBuf) -> Self {
        Self {
            stdlib_path: path,
            loaded: false,
        }
    }

    /// Discover the stdlib path by searching common locations.
    ///
    /// Returns the first existing path, or falls back to the default.
    fn discover_path() -> PathBuf {
        // Try next to the executable first (for installed binaries)
        if let Some(exe_dir) = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
        {
            let stdlib_next_to_exe = exe_dir.join(STDLIB_DIR);
            if stdlib_next_to_exe.exists() && stdlib_next_to_exe.is_dir() {
                return stdlib_next_to_exe;
            }
        }

        // Fall back to current directory / default path
        PathBuf::from(STDLIB_DIR)
    }

    /// Returns true if stdlib has been loaded by this loader
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    /// Ensures stdlib is loaded into an AnalysisHost - loads only if not already loaded.
    ///
    /// Returns `Ok(true)` if stdlib was loaded, `Ok(false)` if already loaded.
    pub fn ensure_loaded_into_host(&mut self, host: &mut AnalysisHost) -> Result<bool, String> {
        if self.loaded {
            return Ok(false);
        }

        self.load_into_host(host)?;
        self.loaded = true;
        Ok(true)
    }

    /// Loads the SysML standard library into an AnalysisHost.
    pub fn load_into_host(&self, host: &mut AnalysisHost) -> Result<(), String> {
        loader::load_into_host(&self.stdlib_path, host)
    }
}

impl Default for StdLibLoader {
    fn default() -> Self {
        Self::new()
    }
}
