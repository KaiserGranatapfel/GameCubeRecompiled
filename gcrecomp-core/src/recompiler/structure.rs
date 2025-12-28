//! File Structure Generator for Recompiled Code
//!
//! This module generates hierarchical file structures from linker script rules,
//! organizing functions into modules and namespaces.

use crate::recompiler::ghidra::{FunctionInfo, SymbolInfo};
use crate::recompiler::linker::LinkerScript;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Organized function structure
#[derive(Debug, Clone)]
pub struct OrganizedFunction {
    /// Function information
    pub function: FunctionInfo,
    /// Module path (e.g., "graphics/gx")
    pub module_path: String,
    /// Namespace path (e.g., ["graphics", "gx"])
    pub namespace_path: Vec<String>,
    /// File name for this function
    pub file_name: String,
}

/// Module structure information
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    /// Module path (e.g., "graphics/gx")
    pub path: String,
    /// Functions in this module
    pub functions: Vec<OrganizedFunction>,
    /// Sub-modules
    pub submodules: Vec<String>,
}

/// Structure generator for organizing recompiled code
pub struct StructureGenerator {
    /// Linker script rules
    linker_script: Option<LinkerScript>,
    /// Base output directory
    base_dir: PathBuf,
}

impl StructureGenerator {
    /// Create a new structure generator
    ///
    /// # Arguments
    /// * `linker_script` - Optional linker script for organization rules
    /// * `base_dir` - Base output directory
    ///
    /// # Returns
    /// `StructureGenerator` - New generator instance
    pub fn new(linker_script: Option<LinkerScript>, base_dir: PathBuf) -> Self {
        Self {
            linker_script,
            base_dir,
        }
    }

    /// Organize functions into hierarchical structure based on linker rules and namespaces.
    ///
    /// This method groups functions into modules and namespaces using the following priority:
    /// 1. Linker script rules (if provided) - pattern matching (e.g., "GX*" -> "graphics/gx")
    /// 2. Function metadata - module_path and namespace_path from Ghidra analysis
    /// 3. Symbol metadata - module_path and namespace_path from symbol information
    /// 4. SDK pattern detection - automatic detection of SDK prefixes (GX*, VI*, OS*, etc.)
    /// 5. Default - functions without matches go to "unknown" module
    ///
    /// # Arguments
    /// * `functions` - Functions to organize (from Ghidra analysis)
    /// * `symbols` - Symbols for additional context (used for cross-referencing)
    ///
    /// # Returns
    /// `Result<HashMap<String, ModuleInfo>>` - Organized modules keyed by module path
    ///
    /// # Example
    /// ```
    /// // Functions matching "GX*" pattern will be organized into "graphics/gx" module
    /// // Functions matching "VI*" pattern will be organized into "graphics/vi" module
    /// // Unknown functions go to "unknown" module
    /// ```
    pub fn organize_functions(
        &self,
        functions: &[FunctionInfo],
        symbols: &[SymbolInfo],
    ) -> Result<HashMap<String, ModuleInfo>> {
        // Create a lookup map for fast symbol access by address
        // This allows us to quickly find symbol information for a function address
        let mut modules: HashMap<String, ModuleInfo> = HashMap::new();
        let symbol_map: HashMap<u32, &SymbolInfo> =
            symbols.iter().map(|s| (s.address, s)).collect();

        // Process each function and organize it into the appropriate module
        for func in functions {
            // Determine module path, namespace path, and file name for this function
            let organized = self.organize_function(func, &symbol_map)?;

            // Get or create the module entry
            let module_path = &organized.module_path;
            let entry = modules.entry(module_path.clone()).or_insert_with(|| {
                // Create new module if it doesn't exist
                ModuleInfo {
                    path: module_path.clone(),
                    functions: Vec::new(),
                    submodules: Vec::new(),
                }
            });

            // Add function to the module
            entry.functions.push(organized);
        }

        // Build submodule hierarchy
        // This creates parent modules for nested paths (e.g., "graphics" for "graphics/gx")
        self.build_submodule_hierarchy(&mut modules)?;

        Ok(modules)
    }

    /// Organize a single function
    fn organize_function(
        &self,
        func: &FunctionInfo,
        symbol_map: &HashMap<u32, &SymbolInfo>,
    ) -> Result<OrganizedFunction> {
        // Determine module path
        let module_path = if let Some(module) = &func.module_path {
            module.clone()
        } else if let Some(symbol) = symbol_map.get(&func.address) {
            if let Some(module) = &symbol.module_path {
                module.clone()
            } else {
                self.determine_module_from_name(&func.name)?
            }
        } else {
            self.determine_module_from_name(&func.name)?
        };

        // Determine namespace path
        let namespace_path = if !func.namespace_path.is_empty() {
            func.namespace_path.clone()
        } else if let Some(symbol) = symbol_map.get(&func.address) {
            if !symbol.namespace_path.is_empty() {
                symbol.namespace_path.clone()
            } else {
                self.determine_namespace_from_module(&module_path)
            }
        } else {
            self.determine_namespace_from_module(&module_path)
        };

        // Generate file name
        let file_name = self.generate_file_name(func);

        Ok(OrganizedFunction {
            function: func.clone(),
            module_path,
            namespace_path,
            file_name,
        })
    }

    /// Determine module path from function name using linker script rules
    fn determine_module_from_name(&self, name: &str) -> Result<String> {
        if let Some(linker) = &self.linker_script {
            if let Some(module) = linker.get_module_for_symbol(name) {
                return Ok(module);
            }
        }

        // Default: use "unknown" module
        Ok("unknown".to_string())
    }

    /// Determine namespace path from module path
    fn determine_namespace_from_module(&self, module_path: &str) -> Vec<String> {
        // Split module path by '/' to get namespace components
        module_path.split('/').map(|s| s.to_string()).collect()
    }

    /// Generate file name for a function
    fn generate_file_name(&self, func: &FunctionInfo) -> String {
        // Use function name if it's not a default name, otherwise use address
        if func.name.is_empty() || func.name.starts_with("sub_") || func.name.starts_with("FUN_") {
            format!("func_0x{:08X}.rs", func.address)
        } else {
            // Sanitize function name for file system
            let sanitized = func
                .name
                .replace("::", "_")
                .replace(" ", "_")
                .replace("-", "_")
                .replace(".", "_");
            format!("{}_{:08X}.rs", sanitized, func.address)
        }
    }

    /// Build submodule hierarchy
    fn build_submodule_hierarchy(&self, modules: &mut HashMap<String, ModuleInfo>) -> Result<()> {
        let module_paths: Vec<String> = modules.keys().cloned().collect();

        for path in module_paths {
            let parts: Vec<&str> = path.split('/').collect();

            // Build parent modules
            for i in 1..parts.len() {
                let parent_path = parts[0..i].join("/");
                let child_name = parts[i].to_string();

                if let Some(parent) = modules.get_mut(&parent_path) {
                    if !parent.submodules.contains(&child_name) {
                        parent.submodules.push(child_name);
                    }
                } else {
                    // Create parent module
                    modules.insert(
                        parent_path.clone(),
                        ModuleInfo {
                            path: parent_path,
                            functions: Vec::new(),
                            submodules: vec![child_name],
                        },
                    );
                }
            }
        }

        Ok(())
    }

    /// Generate module tree structure by creating directories and mod.rs files.
    ///
    /// This method creates the physical file system structure for the hierarchical module
    /// organization. For each module, it:
    /// 1. Creates the module directory (e.g., "graphics/gx")
    /// 2. Generates a mod.rs file that declares submodules and re-exports functions
    ///
    /// The resulting structure allows Rust's module system to properly resolve imports
    /// and provides a clean API for accessing recompiled functions.
    ///
    /// # Arguments
    /// * `modules` - Organized modules from `organize_functions()`
    ///
    /// # Returns
    /// `Result<()>` - Success or error if directory creation or file writing fails
    ///
    /// # File Structure Created
    /// ```
    /// base_dir/
    /// ├── graphics/
    /// │   ├── mod.rs          (declares gx, vi submodules)
    /// │   ├── gx/
    /// │   │   ├── mod.rs      (re-exports GX functions)
    /// │   │   └── *.rs        (individual function files)
    /// │   └── vi/
    /// │       ├── mod.rs
    /// │       └── *.rs
    /// └── system/
    ///     └── ...
    /// ```
    pub fn generate_module_tree(&self, modules: &HashMap<String, ModuleInfo>) -> Result<()> {
        // Create base directory if it doesn't exist
        std::fs::create_dir_all(&self.base_dir).context("Failed to create base directory")?;

        // Generate mod.rs files for each module in the hierarchy
        for (path, module_info) in modules {
            // Create module directory (e.g., "graphics/gx")
            let module_dir = self.base_dir.join(path);
            std::fs::create_dir_all(&module_dir).with_context(|| {
                format!(
                    "Failed to create module directory: {}",
                    module_dir.display()
                )
            })?;

            // Generate mod.rs file for this module
            // This file declares submodules and re-exports functions
            let mod_rs_path = module_dir.join("mod.rs");
            let mod_rs_content = self.generate_mod_rs(module_info)?;
            std::fs::write(&mod_rs_path, mod_rs_content)
                .with_context(|| format!("Failed to write mod.rs: {}", mod_rs_path.display()))?;
        }

        Ok(())
    }

    /// Generate mod.rs content for a module
    fn generate_mod_rs(&self, module_info: &ModuleInfo) -> Result<String> {
        let mut content = String::new();

        content.push_str("//! Auto-generated module\n");
        content.push_str("//! Generated by GCRecomp\n\n");

        // Add submodule declarations
        for submodule in &module_info.submodules {
            content.push_str(&format!("pub mod {};\n", submodule));
        }

        // Add function re-exports
        if !module_info.functions.is_empty() {
            content.push_str("\n// Function exports\n");
            for func in &module_info.functions {
                let func_name = self.get_function_identifier(&func.function);
                content.push_str(&format!(
                    "pub use super::{}::{};\n",
                    func.file_name.trim_end_matches(".rs"),
                    func_name
                ));
            }
        }

        Ok(content)
    }

    /// Get function identifier for use in code
    fn get_function_identifier(&self, func: &FunctionInfo) -> String {
        if func.name.is_empty() || func.name.starts_with("sub_") || func.name.starts_with("FUN_") {
            format!("func_0x{:08X}", func.address)
        } else {
            func.name
                .replace("::", "_")
                .replace(" ", "_")
                .replace("-", "_")
                .replace(".", "_")
        }
    }

    /// Write function files
    ///
    /// # Arguments
    /// * `modules` - Organized modules
    /// * `function_code_map` - Map of function addresses to generated code
    ///
    /// # Returns
    /// `Result<()>` - Success or error
    pub fn write_function_files(
        &self,
        modules: &HashMap<String, ModuleInfo>,
        function_code_map: &HashMap<u32, String>,
    ) -> Result<()> {
        for module_info in modules.values() {
            let module_dir = self.base_dir.join(&module_info.path);

            for func in &module_info.functions {
                let file_path = module_dir.join(&func.file_name);

                if let Some(code) = function_code_map.get(&func.function.address) {
                    std::fs::write(&file_path, code).with_context(|| {
                        format!("Failed to write function file: {}", file_path.display())
                    })?;
                }
            }
        }

        Ok(())
    }

    /// Get full path for a function file
    ///
    /// # Arguments
    /// * `organized_func` - Organized function
    ///
    /// # Returns
    /// `PathBuf` - Full path to function file
    pub fn get_function_path(&self, organized_func: &OrganizedFunction) -> PathBuf {
        self.base_dir
            .join(&organized_func.module_path)
            .join(&organized_func.file_name)
    }
}
