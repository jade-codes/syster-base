use crate::ide::AnalysisHost;
use std::path::PathBuf;

use super::file_loader;

#[cfg(feature = "interchange")]
use crate::interchange::integrate::apply_metadata_to_host;
#[cfg(feature = "interchange")]
use crate::interchange::metadata::ImportMetadata;

/// Loads workspace files on demand
pub struct WorkspaceLoader;

impl WorkspaceLoader {
    pub fn new() -> Self {
        Self
    }

    /// Loads all SysML and KerML files from a directory into an AnalysisHost.
    pub fn load_directory_into_host<P: Into<PathBuf>>(
        &self,
        path: P,
        host: &mut AnalysisHost,
    ) -> Result<(), String> {
        let path = path.into();
        if !path.exists() || !path.is_dir() {
            return Err(format!("Directory not found: {}", path.display()));
        }
        self.load_directory_recursive_into_host(&path, host)
    }

    fn load_directory_recursive_into_host(
        &self,
        dir: &PathBuf,
        host: &mut AnalysisHost,
    ) -> Result<(), String> {
        let paths = file_loader::collect_file_paths(dir)?;
        let mut errors = Vec::new();

        for path in paths {
            match file_loader::load_and_parse(&path) {
                Ok(file) => {
                    host.set_file(path, file);
                }
                Err(e) => {
                    errors.push(format!("{}: {}", path.display(), e));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "Failed to load {} file(s):\n  {}",
                errors.len(),
                errors.join("\n  ")
            ))
        }
    }

    /// Loads a single file into an AnalysisHost.
    pub fn load_file_into_host<P: Into<PathBuf>>(
        &self,
        path: P,
        host: &mut AnalysisHost,
    ) -> Result<(), String> {
        let path = path.into();
        let file = file_loader::load_and_parse(&path)?;
        host.set_file(path, file);
        Ok(())
    }

    /// Scans a directory for metadata files and loads them into the host.
    ///
    /// Looks for files matching these patterns:
    /// - `*.xmi.metadata` - ImportMetadata for individual files
    /// - `*.sysml.metadata` - ImportMetadata for individual files  
    /// - `meta.json` - KPAR package-level metadata
    ///
    /// This should be called after loading SysML files to apply element IDs.
    #[cfg(feature = "interchange")]
    pub fn load_metadata_from_directory<P: Into<PathBuf>>(
        &self,
        path: P,
        host: &mut AnalysisHost,
    ) -> Result<(), String> {
        let path = path.into();
        if !path.exists() || !path.is_dir() {
            return Err(format!("Directory not found: {}", path.display()));
        }

        let mut metadata_files = Vec::new();

        // Recursively find metadata files
        if let Err(e) = self.collect_metadata_files_recursive(&path, &mut metadata_files) {
            return Err(format!("Error scanning for metadata files: {}", e));
        }

        // Load each metadata file
        let mut errors = Vec::new();
        for metadata_path in metadata_files {
            match ImportMetadata::read_from_file(&metadata_path) {
                Ok(metadata) => {
                    apply_metadata_to_host(host, &metadata);
                    tracing::debug!("Loaded metadata from {}", metadata_path.display());
                }
                Err(e) => {
                    errors.push(format!("{}: {}", metadata_path.display(), e));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "Failed to load {} metadata file(s):\n  {}",
                errors.len(),
                errors.join("\n  ")
            ))
        }
    }

    #[cfg(feature = "interchange")]
    fn collect_metadata_files_recursive(
        &self,
        dir: &PathBuf,
        results: &mut Vec<PathBuf>,
    ) -> Result<(), String> {
        let entries = std::fs::read_dir(dir)
            .map_err(|e| format!("Failed to read directory {}: {}", dir.display(), e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();

            if path.is_dir() {
                // Recurse into subdirectories
                self.collect_metadata_files_recursive(&path, results)?;
            } else if path.is_file() {
                // Check if it's a metadata file
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name.ends_with(".metadata")
                        || file_name.ends_with(".metadata.json")
                        || file_name == "meta.json"
                    {
                        results.push(path);
                    }
                }
            }
        }

        Ok(())
    }
}

impl Default for WorkspaceLoader {
    fn default() -> Self {
        Self::new()
    }
}
