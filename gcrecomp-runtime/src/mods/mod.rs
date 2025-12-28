//! Modding System Infrastructure
//!
//! This module provides the plugin system for GCRecomp, allowing external
//! mods to extend and modify game behavior through a well-defined API.
//!
//! # Overview
//!
//! GCRecomp supports a plugin-based modding system that allows external mods to:
//! - Intercept and modify function calls
//! - Access CPU context and memory
//! - Extend game functionality
//! - Interact with the runtime system
//!
//! # Mod Structure
//!
//! A mod consists of:
//! 1. **Dynamic Library** (`.so`, `.dylib`, or `.dll`) - The compiled mod code
//! 2. **Metadata File** (`.json` or `.toml`) - Mod information and dependencies
//! 3. **Configuration File** (`.config.json`, optional) - Mod-specific settings
//!
//! ## Example Mod Structure
//!
//! ```
//! my_mod/
//! ├── my_mod.so          # Compiled mod library
//! ├── my_mod.json        # Metadata
//! └── my_mod.config.json # Configuration (optional)
//! ```
//!
//! # Creating a Mod
//!
//! ## 1. Define Mod Metadata
//!
//! Create a `my_mod.json` file:
//!
//! ```json
//! {
//!   "name": "my_mod",
//!   "version": "1.0.0",
//!   "author": "Your Name",
//!   "description": "A sample mod",
//!   "dependencies": []
//! }
//! ```
//!
//! Or in TOML format (`my_mod.toml`):
//!
//! ```toml
//! name = "my_mod"
//! version = "1.0.0"
//! author = "Your Name"
//! description = "A sample mod"
//! dependencies = []
//! ```
//!
//! ## 2. Implement the Mod Trait
//!
//! Your mod must implement the `Mod` trait. See the `Mod` trait documentation for details.
//!
//! ## 3. Export Required Symbols
//!
//! Your mod library must export two functions:
//! - `mod_metadata()` - Returns `ModMetadata`
//! - `mod_init()` - Returns `*mut dyn Mod`
//!
//! # Mod Dependencies
//!
//! Mods can depend on other mods. Specify dependencies in metadata:
//!
//! ```json
//! {
//!   "name": "my_mod",
//!   "version": "1.0.0",
//!   "dependencies": [
//!     {
//!       "name": "base_mod",
//!       "version": ">=1.0.0"
//!     }
//!   ]
//! }
//! ```
//!
//! Dependencies are automatically resolved and loaded in the correct order.
//!
//! # Configuration
//!
//! Mods can have configuration files (`my_mod.config.json`):
//!
//! ```json
//! {
//!   "enabled": true,
//!   "some_setting": 42,
//!   "another_setting": "value"
//! }
//! ```
//!
//! Access configuration in the `initialize` method via the `config` parameter.
//!
//! # Building a Mod
//!
//! ## Cargo.toml
//!
//! ```toml
//! [package]
//! name = "my_mod"
//! version = "1.0.0"
//! edition = "2021"
//!
//! [lib]
//! crate-type = ["cdylib"]  # Important: must be cdylib
//!
//! [dependencies]
//! gcrecomp-runtime = { path = "../../gcrecomp-runtime" }
//! anyhow = "1.0"
//! serde = { version = "1.0", features = ["derive"] }
//! serde_json = "1.0"
//! log = "0.4"
//! ```
//!
//! ## Build Command
//!
//! ```bash
//! cargo build --release
//! ```
//!
//! The compiled library will be in `target/release/libmy_mod.so` (or `.dylib`/`.dll` on other platforms).
//!
//! # Loading Mods
//!
//! Mods are loaded from a directory specified at runtime:
//!
//! ```rust
//! use gcrecomp_runtime::Runtime;
//!
//! let mut runtime = Runtime::new()?;
//! runtime.load_mods(std::path::Path::new("./mods"))?;
//! ```
//!
//! # Best Practices
//!
//! 1. **Error Handling**: Always return proper `Result` types and handle errors gracefully
//! 2. **Logging**: Use the `log` crate for debug output
//! 3. **Resource Cleanup**: Clean up resources in `shutdown()`
//! 4. **Thread Safety**: Mods must be `Send + Sync`
//! 5. **Versioning**: Use semantic versioning for mod versions
//! 6. **Documentation**: Document your mod's functionality and API
//!
//! # Limitations
//!
//! - Mods cannot modify the recompiled code itself (only intercept calls)
//! - Hook execution adds overhead (minimal but measurable)
//! - Mods must be compiled for the same target platform as the runtime
//!
//! # Troubleshooting
//!
//! ## Mod Not Loading
//!
//! - Check that the library file exists and is the correct format
//! - Verify metadata file is valid JSON/TOML
//! - Check dependencies are satisfied
//! - Review logs for error messages
//!
//! ## Hooks Not Working
//!
//! - Ensure hooks are registered before functions are called
//! - Check that the hook manager is properly initialized
//! - Verify function addresses are correct
//!
//! ## Build Errors
//!
//! - Ensure `crate-type = ["cdylib"]` in Cargo.toml
//! - Check that all dependencies are available
//! - Verify Rust version compatibility

pub mod api;
pub mod hook_integration;
pub mod hooks;
pub mod loader;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Metadata for a mod plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModMetadata {
    /// Mod name (unique identifier)
    pub name: String,
    /// Mod version (semver format)
    pub version: String,
    /// Mod author
    pub author: String,
    /// Brief description
    pub description: String,
    /// Dependencies (mod names and version requirements)
    pub dependencies: Vec<Dependency>,
}

/// Dependency specification for a mod.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// Name of the required mod
    pub name: String,
    /// Version requirement (semver range)
    pub version: String,
}

/// Trait that all mod plugins must implement.
pub trait Mod: Send + Sync {
    /// Initialize the mod. Called when the mod is loaded.
    ///
    /// # Arguments
    /// * `metadata` - The mod's metadata
    /// * `config` - Mod-specific configuration (from config file)
    ///
    /// # Returns
    /// `Result<()>` - Success or error during initialization
    fn initialize(&mut self, metadata: &ModMetadata, config: &serde_json::Value) -> Result<()>;

    /// Called when the mod is being unloaded.
    ///
    /// Allows the mod to clean up resources.
    fn shutdown(&mut self) -> Result<()>;

    /// Get the mod's metadata.
    fn metadata(&self) -> &ModMetadata;
}

/// Registry for managing loaded mods.
pub struct ModRegistry {
    /// Map of mod name to loaded mod instance
    mods: HashMap<String, Box<dyn Mod>>,
    /// Map of mod name to metadata
    metadata: HashMap<String, ModMetadata>,
    /// Map of mod name to its library path
    mod_paths: HashMap<String, PathBuf>,
}

impl ModRegistry {
    /// Create a new mod registry.
    pub fn new() -> Self {
        Self {
            mods: HashMap::new(),
            metadata: HashMap::new(),
            mod_paths: HashMap::new(),
        }
    }

    /// Discover mods in a directory.
    ///
    /// Scans the directory for mod files (.so, .dylib, .dll) and their
    /// associated metadata files.
    ///
    /// # Arguments
    /// * `mod_dir` - Directory to scan for mods
    ///
    /// # Returns
    /// `Vec<ModMetadata>` - List of discovered mod metadata
    pub fn discover_mods(&self, mod_dir: &Path) -> Result<Vec<ModMetadata>> {
        let mut discovered = Vec::new();

        if !mod_dir.exists() {
            log::warn!("Mod directory does not exist: {:?}", mod_dir);
            return Ok(discovered);
        }

        for entry in std::fs::read_dir(mod_dir)? {
            let entry = entry?;
            let path = entry.path();

            // Check if it's a mod library file
            if Self::is_mod_file(&path) {
                // Look for metadata file (mod_name.json or mod_name.toml)
                let metadata_path = Self::find_metadata_file(&path)?;
                if let Some(meta_path) = metadata_path {
                    if let Ok(metadata) = Self::load_metadata(&meta_path) {
                        discovered.push(metadata);
                    }
                } else {
                    log::warn!("No metadata file found for mod: {:?}", path);
                }
            }
        }

        Ok(discovered)
    }

    /// Check if a file is a mod library file.
    pub fn is_mod_file(path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            matches!(ext_str.as_str(), "so" | "dylib" | "dll")
        } else {
            false
        }
    }

    /// Find metadata file for a mod library.
    pub fn find_metadata_file(lib_path: &Path) -> Result<Option<PathBuf>> {
        let base_name = lib_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid mod file path"))?;

        let dir = lib_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Mod file has no parent directory"))?;

        // Try JSON first, then TOML
        let json_path = dir.join(format!("{}.json", base_name));
        if json_path.exists() {
            return Ok(Some(json_path));
        }

        let toml_path = dir.join(format!("{}.toml", base_name));
        if toml_path.exists() {
            return Ok(Some(toml_path));
        }

        Ok(None)
    }

    /// Load metadata from a file.
    pub fn load_metadata(path: &Path) -> Result<ModMetadata> {
        let content = std::fs::read_to_string(path)?;

        if path.extension().and_then(|e| e.to_str()) == Some("toml") {
            // TOML format
            let metadata: ModMetadata = toml::from_str(&content)?;
            Ok(metadata)
        } else {
            // JSON format (default)
            let metadata: ModMetadata = serde_json::from_str(&content)?;
            Ok(metadata)
        }
    }

    /// Register a mod in the registry.
    ///
    /// # Arguments
    /// * `name` - Mod name
    /// * `mod_instance` - The mod instance
    /// * `metadata` - Mod metadata
    /// * `path` - Path to the mod library
    pub fn register_mod(
        &mut self,
        name: String,
        mod_instance: Box<dyn Mod>,
        metadata: ModMetadata,
        path: PathBuf,
    ) {
        self.metadata.insert(name.clone(), metadata);
        self.mod_paths.insert(name.clone(), path);
        self.mods.insert(name, mod_instance);
    }

    /// Get a mod by name.
    pub fn get_mod(&self, name: &str) -> Option<&dyn Mod> {
        self.mods.get(name).map(|m| m.as_ref())
    }

    /// Get a mod mutably by name.
    pub fn get_mod_mut(&mut self, name: &str) -> Option<&mut dyn Mod> {
        self.mods.get_mut(name).map(|m| m.as_mut())
    }

    /// Get all loaded mod names.
    pub fn mod_names(&self) -> Vec<String> {
        self.mods.keys().cloned().collect()
    }

    /// Check if a mod is loaded.
    pub fn is_loaded(&self, name: &str) -> bool {
        self.mods.contains_key(name)
    }

    /// Unload a mod.
    pub fn unload_mod(&mut self, name: &str) -> Result<()> {
        if let Some(mut mod_instance) = self.mods.remove(name) {
            mod_instance.shutdown()?;
        }
        self.metadata.remove(name);
        self.mod_paths.remove(name);
        Ok(())
    }

    /// Unload all mods.
    pub fn unload_all(&mut self) -> Result<()> {
        let names: Vec<String> = self.mods.keys().cloned().collect();
        for name in names {
            self.unload_mod(&name)?;
        }
        Ok(())
    }
}

impl Default for ModRegistry {
    fn default() -> Self {
        Self::new()
    }
}
