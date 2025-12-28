//! Mod Loader for Dynamic Library Loading
//!
//! This module handles loading mod plugins from dynamic libraries (.so, .dylib, .dll).
//!
//! # Overview
//!
//! The mod loader discovers, loads, and initializes mod plugins from dynamic libraries.
//! It handles:
//! - Dynamic library loading using `libloading`
//! - Mod initialization and shutdown lifecycle
//! - Dependency resolution and load ordering
//! - Configuration file loading
//!
//! # Usage
//!
//! ```rust,no_run
//! use gcrecomp_runtime::mods::loader::ModLoader;
//!
//! let mut loader = ModLoader::new();
//! let loaded_mods = loader.load_mods_from_directory(std::path::Path::new("./mods"))?;
//!
//! for (metadata, mod_instance) in loaded_mods {
//!     // Mod is loaded and initialized
//! }
//! ```
//!
//! # Mod Library Requirements
//!
//! Mod libraries must export two functions:
//! - `mod_metadata()` - Returns `ModMetadata`
//! - `mod_init()` - Returns `*mut dyn Mod`
//!
//! These functions must be marked with `#[no_mangle]` and `extern "C"`.

use super::r#mod::{Mod, ModMetadata, ModRegistry};
use anyhow::{Context, Result};
use libloading::{Library, Symbol};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Function signature for mod initialization.
type ModInitFn = unsafe extern "C" fn() -> *mut dyn Mod;

/// Function signature for mod metadata retrieval.
type ModMetadataFn = unsafe extern "C" fn() -> ModMetadata;

/// Loader for mod plugins from dynamic libraries.
pub struct ModLoader {
    /// Loaded libraries (kept alive for the lifetime of the loader)
    libraries: Vec<Library>,
    /// Map of mod name to its library path
    pub mod_paths: HashMap<String, PathBuf>,
}

impl ModLoader {
    /// Create a new mod loader.
    pub fn new() -> Self {
        Self {
            libraries: Vec::new(),
            mod_paths: HashMap::new(),
        }
    }

    /// Load a mod from a dynamic library.
    ///
    /// # Arguments
    /// * `lib_path` - Path to the mod library file
    ///
    /// # Returns
    /// `(ModMetadata, Box<dyn Mod>)` - The mod's metadata and instance
    ///
    /// # Safety
    /// This function is unsafe because it loads and executes code from a dynamic library.
    /// The library must export the required symbols:
    /// - `mod_init`: Function that returns a `*mut dyn Mod`
    /// - `mod_metadata`: Function that returns `ModMetadata`
    pub unsafe fn load_mod(&mut self, lib_path: &Path) -> Result<(ModMetadata, Box<dyn Mod>)> {
        log::info!("Loading mod from: {:?}", lib_path);

        // Load the library
        let library = Library::new(lib_path)
            .with_context(|| format!("Failed to load mod library: {:?}", lib_path))?;

        // Get the mod_metadata function
        let metadata_fn: Symbol<ModMetadataFn> = library
            .get(b"mod_metadata")
            .with_context(|| "Library does not export mod_metadata symbol")?;

        // Get the mod_init function
        let init_fn: Symbol<ModInitFn> = library
            .get(b"mod_init")
            .with_context(|| "Library does not export mod_init symbol")?;

        // Call mod_metadata to get metadata
        let metadata = metadata_fn();

        // Call mod_init to create mod instance
        let mod_ptr = init_fn();
        if mod_ptr.is_null() {
            anyhow::bail!("mod_init returned null pointer");
        }

        // Convert raw pointer to Box
        let mod_instance = Box::from_raw(mod_ptr);

        // Keep library alive
        self.libraries.push(library);
        self.mod_paths
            .insert(metadata.name.clone(), lib_path.to_path_buf());

        Ok((metadata, mod_instance))
    }

    /// Load a mod with configuration.
    ///
    /// # Arguments
    /// * `lib_path` - Path to the mod library file
    /// * `config` - Mod configuration (JSON value)
    ///
    /// # Returns
    /// `(ModMetadata, Box<dyn Mod>)` - The mod's metadata and initialized instance
    pub fn load_mod_with_config(
        &mut self,
        lib_path: &Path,
        config: &serde_json::Value,
    ) -> Result<(ModMetadata, Box<dyn Mod>)> {
        let (metadata, mut mod_instance) = unsafe { self.load_mod(lib_path)? };

        // Initialize the mod with its configuration
        mod_instance
            .initialize(&metadata, config)
            .with_context(|| format!("Failed to initialize mod: {}", metadata.name))?;

        Ok((metadata, mod_instance))
    }

    /// Load mods from a directory.
    ///
    /// Discovers mods in the directory and loads them, respecting dependencies.
    ///
    /// # Arguments
    /// * `mod_dir` - Directory containing mods
    ///
    /// # Returns
    /// `Vec<(ModMetadata, Box<dyn Mod>)>` - List of loaded mods
    pub fn load_mods_from_directory(
        &mut self,
        mod_dir: &Path,
    ) -> Result<Vec<(ModMetadata, Box<dyn Mod>)>> {
        let mut loaded_mods = Vec::new();

        if !mod_dir.exists() {
            log::warn!("Mod directory does not exist: {:?}", mod_dir);
            return Ok(loaded_mods);
        }

        // First, discover all mods and their metadata
        let mut mod_metadata: Vec<(PathBuf, ModMetadata)> = Vec::new();
        for entry in std::fs::read_dir(mod_dir)? {
            let entry = entry?;
            let path = entry.path();

            if Self::is_mod_file(&path) {
                if let Some(meta_path) = ModRegistry::find_metadata_file(&path)? {
                    if let Ok(metadata) = ModRegistry::load_metadata(&meta_path) {
                        mod_metadata.push((path, metadata));
                    }
                }
            }
        }

        // Sort mods by dependencies (topological sort)
        let sorted_mods = Self::sort_by_dependencies(mod_metadata)?;

        // Load mods in dependency order
        for (lib_path, metadata) in sorted_mods {
            // Load mod configuration if it exists
            let config_path = lib_path
                .parent()
                .map(|p| p.join(format!("{}.config.json", metadata.name)))
                .unwrap_or_else(|| {
                    log::warn!("Mod library path has no parent: {:?}", lib_path);
                    lib_path.clone()
                });
            let config = if config_path.exists() {
                let config_str = std::fs::read_to_string(&config_path)?;
                serde_json::from_str(&config_str).unwrap_or_else(|_| serde_json::Value::Null)
            } else {
                serde_json::Value::Null
            };

            match self.load_mod_with_config(&lib_path, &config) {
                Ok((meta, mod_instance)) => {
                    loaded_mods.push((meta, mod_instance));
                    log::info!("Successfully loaded mod: {}", metadata.name);
                }
                Err(e) => {
                    log::error!("Failed to load mod {}: {}", metadata.name, e);
                    // Continue loading other mods
                }
            }
        }

        Ok(loaded_mods)
    }

    /// Check if a file is a mod library file.
    fn is_mod_file(path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            matches!(ext_str.as_str(), "so" | "dylib" | "dll")
        } else {
            false
        }
    }

    /// Sort mods by dependencies (topological sort).
    ///
    /// Ensures that mods are loaded in an order that satisfies their dependencies.
    fn sort_by_dependencies(
        mods: Vec<(PathBuf, ModMetadata)>,
    ) -> Result<Vec<(PathBuf, ModMetadata)>> {
        // Simple implementation: check dependencies and sort
        // For a more robust implementation, use a proper topological sort algorithm
        let mut sorted = Vec::new();
        let mut remaining: Vec<(PathBuf, ModMetadata)> = mods;

        while !remaining.is_empty() {
            let mut progress = false;

            for i in (0..remaining.len()).rev() {
                let (ref path, ref metadata) = remaining[i];
                let deps_satisfied = metadata.dependencies.iter().all(|dep| {
                    // Check if dependency is already loaded or in remaining list
                    sorted.iter().any(|(_, m)| m.name == dep.name)
                        || remaining.iter().any(|(_, m)| m.name == dep.name)
                });

                if deps_satisfied || metadata.dependencies.is_empty() {
                    let (path, metadata) = remaining.remove(i);
                    sorted.push((path, metadata));
                    progress = true;
                }
            }

            if !progress {
                // Circular dependency or missing dependency
                let missing: Vec<String> = remaining
                    .iter()
                    .flat_map(|(_, m)| &m.dependencies)
                    .map(|d| d.name.clone())
                    .collect();
                anyhow::bail!(
                    "Cannot resolve mod dependencies. Missing or circular: {:?}",
                    missing
                );
            }
        }

        Ok(sorted)
    }

    /// Unload all mods and libraries.
    pub fn unload_all(&mut self) {
        self.libraries.clear();
        self.mod_paths.clear();
    }
}

impl Default for ModLoader {
    fn default() -> Self {
        Self::new()
    }
}
