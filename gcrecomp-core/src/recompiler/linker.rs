//! Linker Script Parser for GameCube Recompilation
//!
//! This module parses linker scripts to define memory layout, symbol organization,
//! and output file structure rules for recompiled code.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Parsed linker script configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkerScript {
    /// Memory regions defined in the script
    pub memory: HashMap<String, MemoryRegion>,
    /// Section definitions
    pub sections: Vec<SectionDefinition>,
    /// Namespace organization rules
    pub namespaces: HashMap<String, NamespaceRule>,
}

/// Memory region definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRegion {
    /// Origin address
    pub origin: u32,
    /// Length in bytes
    pub length: u32,
}

/// Section definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionDefinition {
    /// Section name (e.g., ".text", ".data")
    pub name: String,
    /// Input section pattern (e.g., "*(.text.*)")
    pub input_pattern: String,
    /// Target memory region
    pub memory_region: Option<String>,
}

/// Namespace organization rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamespaceRule {
    /// Pattern to match symbol names (supports wildcards)
    pub pattern: String,
    /// Module path for matching symbols
    pub module: String,
    /// Optional namespace path
    pub namespace: Option<Vec<String>>,
}

impl LinkerScript {
    /// Parse a linker script from a file
    ///
    /// # Arguments
    /// * `path` - Path to linker script file
    ///
    /// # Returns
    /// `Result<LinkerScript>` - Parsed linker script
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read linker script: {}", path.display()))?;

        Self::from_str(&content)
    }

    /// Parse a linker script from a string.
    ///
    /// This parser supports a GameCube-specific linker script format that extends
    /// standard linker scripts with NAMESPACES blocks for function organization.
    ///
    /// # Supported Blocks
    ///
    /// 1. **MEMORY**: Defines memory regions (TEXT, DATA, BSS)
    ///    ```linker
    ///    MEMORY {
    ///        TEXT : ORIGIN = 0x80003100, LENGTH = 0x1000000
    ///        DATA : ORIGIN = 0x80400000, LENGTH = 0x1000000
    ///    }
    ///    ```
    ///
    /// 2. **SECTIONS**: Maps input sections to memory regions
    ///    ```linker
    ///    SECTIONS {
    ///        .text : { *(.text.*) } > TEXT
    ///        .data : { *(.data.*) } > DATA
    ///    }
    ///    ```
    ///
    /// 3. **NAMESPACES**: Organizes functions into modules based on name patterns
    ///    ```linker
    ///    NAMESPACES {
    ///        "gx" : {
    ///            pattern: "GX*",
    ///            module: "graphics/gx",
    ///            namespace: ["graphics"]
    ///        }
    ///    }
    ///    ```
    ///
    /// # Arguments
    /// * `content` - Linker script content as a string
    ///
    /// # Returns
    /// `Result<LinkerScript>` - Parsed linker script with memory, sections, and namespaces
    ///
    /// # Errors
    /// Returns error if:
    /// - Memory region syntax is invalid
    /// - Section definition is malformed
    /// - Namespace rule is incomplete
    /// - Address parsing fails
    pub fn from_str(content: &str) -> Result<Self> {
        // Initialize data structures for parsed components
        let mut memory = HashMap::new();
        let mut sections = Vec::new();
        let mut namespaces = HashMap::new();

        // Split content into lines for line-by-line parsing
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        // Parse each line, looking for block declarations
        while i < lines.len() {
            let line = lines[i].trim();

            // Skip comments (both # and // style) and empty lines
            if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
                i += 1;
                continue;
            }

            // Parse MEMORY block - defines memory regions
            if line.starts_with("MEMORY") {
                i = Self::parse_memory_block(&lines, i + 1, &mut memory)?;
            }
            // Parse SECTIONS block - maps sections to memory regions
            else if line.starts_with("SECTIONS") {
                i = Self::parse_sections_block(&lines, i + 1, &mut sections)?;
            }
            // Parse NAMESPACES block - defines function organization rules
            else if line.starts_with("NAMESPACES") {
                i = Self::parse_namespaces_block(&lines, i + 1, &mut namespaces)?;
            }

            i += 1;
        }

        // Return parsed linker script
        Ok(Self {
            memory,
            sections,
            namespaces,
        })
    }

    /// Parse MEMORY block
    fn parse_memory_block(
        lines: &[&str],
        start: usize,
        memory: &mut HashMap<String, MemoryRegion>,
    ) -> Result<usize> {
        let mut i = start;

        while i < lines.len() {
            let line = lines[i].trim();

            // End of block
            if line == "}" {
                break;
            }

            // Parse memory region: NAME : ORIGIN = 0x..., LENGTH = 0x...
            if line.contains(':') && line.contains("ORIGIN") {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() == 2 {
                    let name = parts[0].trim().to_string();

                    // Extract ORIGIN and LENGTH
                    let mut origin = 0u32;
                    let mut length = 0u32;

                    for part in parts[1].split(',') {
                        let part = part.trim();
                        if part.starts_with("ORIGIN") {
                            if let Some(addr_str) = part.split('=').nth(1) {
                                origin = Self::parse_address(addr_str.trim())?;
                            }
                        } else if part.starts_with("LENGTH") {
                            if let Some(len_str) = part.split('=').nth(1) {
                                length = Self::parse_address(len_str.trim())?;
                            }
                        }
                    }

                    memory.insert(name, MemoryRegion { origin, length });
                }
            }

            i += 1;
        }

        Ok(i)
    }

    /// Parse SECTIONS block
    fn parse_sections_block(
        lines: &[&str],
        start: usize,
        sections: &mut Vec<SectionDefinition>,
    ) -> Result<usize> {
        let mut i = start;

        while i < lines.len() {
            let line = lines[i].trim();

            // End of block
            if line == "}" {
                break;
            }

            // Parse section: .name : { input_pattern } > memory_region
            if line.starts_with('.') && line.contains(':') {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 2 {
                    let name = parts[0].trim().to_string();

                    // Extract input pattern and memory region
                    let mut input_pattern = String::new();
                    let mut memory_region = None;

                    // Look for input pattern in braces
                    let rest = parts[1..].join(":");
                    if let Some(start_brace) = rest.find('{') {
                        if let Some(end_brace) = rest[start_brace..].find('}') {
                            input_pattern = rest[start_brace + 1..start_brace + end_brace]
                                .trim()
                                .to_string();
                        }
                    }

                    // Look for memory region after >
                    if let Some(gt_pos) = rest.find('>') {
                        memory_region = Some(rest[gt_pos + 1..].trim().to_string());
                    }

                    sections.push(SectionDefinition {
                        name,
                        input_pattern: if input_pattern.is_empty() {
                            format!("*({})", name)
                        } else {
                            input_pattern
                        },
                        memory_region,
                    });
                }
            }

            i += 1;
        }

        Ok(i)
    }

    /// Parse NAMESPACES block
    fn parse_namespaces_block(
        lines: &[&str],
        start: usize,
        namespaces: &mut HashMap<String, NamespaceRule>,
    ) -> Result<usize> {
        let mut i = start;

        while i < lines.len() {
            let line = lines[i].trim();

            // End of block
            if line == "}" {
                break;
            }

            // Parse namespace rule: "name" : { pattern: "...", module: "..." }
            if line.starts_with('"') && line.contains(':') {
                let name_end = line[1..].find('"').map(|pos| pos + 1).unwrap_or(0);
                let name = line[1..name_end].to_string();

                // Extract pattern and module from the rest of the line or following lines
                let mut pattern = String::new();
                let mut module = String::new();
                let mut namespace = None;

                // Look for pattern, module, and namespace in the block
                let mut j = i;
                while j < lines.len() && !lines[j].trim().starts_with('}') {
                    let block_line = lines[j].trim();
                    if block_line.starts_with("pattern:") {
                        if let Some(pat) = block_line.split(':').nth(1) {
                            pattern = pat.trim().trim_matches('"').trim_matches(',').to_string();
                        }
                    } else if block_line.starts_with("module:") {
                        if let Some(mod_str) = block_line.split(':').nth(1) {
                            module = mod_str
                                .trim()
                                .trim_matches('"')
                                .trim_matches(',')
                                .to_string();
                        }
                    } else if block_line.starts_with("namespace:") {
                        if let Some(ns_str) = block_line.split(':').nth(1) {
                            let ns_vec: Vec<String> = ns_str
                                .trim()
                                .trim_matches('[')
                                .trim_matches(']')
                                .split(',')
                                .map(|s| s.trim().trim_matches('"').to_string())
                                .collect();
                            if !ns_vec.is_empty() {
                                namespace = Some(ns_vec);
                            }
                        }
                    }
                    j += 1;
                }

                if !pattern.is_empty() && !module.is_empty() {
                    namespaces.insert(
                        name,
                        NamespaceRule {
                            pattern,
                            module,
                            namespace,
                        },
                    );
                }

                i = j - 1;
            }

            i += 1;
        }

        Ok(i)
    }

    /// Parse an address string (hex or decimal)
    fn parse_address(addr_str: &str) -> Result<u32> {
        let cleaned = addr_str.trim_start_matches("0x").trim_start_matches("0X");
        u32::from_str_radix(cleaned, 16)
            .or_else(|_| cleaned.parse::<u32>())
            .context(format!("Failed to parse address: {}", addr_str))
    }

    /// Get module path for a symbol name based on namespace rules
    ///
    /// # Arguments
    /// * `symbol_name` - Symbol name to match
    ///
    /// # Returns
    /// `Option<String>` - Module path if matched, None otherwise
    pub fn get_module_for_symbol(&self, symbol_name: &str) -> Option<String> {
        for rule in self.namespaces.values() {
            if Self::match_pattern(&rule.pattern, symbol_name) {
                return Some(rule.module.clone());
            }
        }
        None
    }

    /// Get namespace path for a symbol name
    ///
    /// # Arguments
    /// * `symbol_name` - Symbol name to match
    ///
    /// # Returns
    /// `Option<Vec<String>>` - Namespace path if matched, None otherwise
    pub fn get_namespace_for_symbol(&self, symbol_name: &str) -> Option<Vec<String>> {
        for rule in self.namespaces.values() {
            if Self::match_pattern(&rule.pattern, symbol_name) {
                return rule.namespace.clone();
            }
        }
        None
    }

    /// Match a pattern against a symbol name (supports wildcards)
    ///
    /// # Arguments
    /// * `pattern` - Pattern to match (supports * wildcard)
    /// * `symbol_name` - Symbol name to test
    ///
    /// # Returns
    /// `bool` - True if pattern matches
    fn match_pattern(pattern: &str, symbol_name: &str) -> bool {
        // Simple wildcard matching: * matches any sequence
        if pattern.contains('*') {
            let regex_pattern = pattern.replace("*", ".*").replace("?", ".");
            // Simple regex-like matching
            if let Ok(re) = regex::Regex::new(&format!("^{}$", regex_pattern)) {
                return re.is_match(symbol_name);
            }
        }

        // Exact match
        pattern == symbol_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_memory() {
        let script = r#"
MEMORY {
    TEXT : ORIGIN = 0x80003100, LENGTH = 0x1000000
    DATA : ORIGIN = 0x80400000, LENGTH = 0x1000000
}
"#;

        let linker = LinkerScript::from_str(script)
            .map_err(|e| anyhow::anyhow!("Failed to parse linker script: {}", e))?;
        assert_eq!(linker.memory.len(), 2);
        assert_eq!(linker.memory["TEXT"].origin, 0x80003100);
        assert_eq!(linker.memory["TEXT"].length, 0x1000000);
    }

    #[test]
    fn test_parse_namespaces() {
        let script = r#"
NAMESPACES {
    "gx" : {
        pattern: "GX*",
        module: "graphics/gx"
    }
}
"#;

        let linker = LinkerScript::from_str(script)
            .map_err(|e| anyhow::anyhow!("Failed to parse linker script: {}", e))?;
        assert_eq!(linker.namespaces.len(), 1);
        assert_eq!(
            linker.get_module_for_symbol("GXInit"),
            Some("graphics/gx".to_string())
        );
    }
}
