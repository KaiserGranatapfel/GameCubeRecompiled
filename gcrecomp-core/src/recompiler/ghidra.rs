//! ReOxide Integration and Ghidra Headless Analysis
//!
//! This module provides integration with Ghidra for reverse engineering analysis.
//! It supports two backends:
//! - **ReOxide**: Python-based tool that enhances Ghidra's decompilation capabilities
//! - **HeadlessCli**: Direct Ghidra headless CLI integration
//!
//! # Auto-Installation
//! The system automatically installs ReOxide via pipx/pip if not present,
//! ensuring seamless integration without manual setup.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub struct GhidraAnalysis {
    pub functions: Vec<FunctionInfo>,
    pub symbols: Vec<SymbolInfo>,
    pub decompiled_code: HashMap<u32, DecompiledFunction>,
    pub instructions: HashMap<u32, Vec<InstructionData>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub address: u32,
    pub name: String,
    pub size: u32,
    pub calling_convention: String,
    pub parameters: Vec<ParameterInfo>,
    pub return_type: Option<String>,
    pub local_variables: Vec<LocalVariableInfo>,
    pub basic_blocks: Vec<BasicBlockInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub param_type: String,
    pub offset: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalVariableInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub var_type: String,
    pub offset: i32,
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicBlockInfo {
    pub address: String,
    pub size: u32,
    pub instructions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub address: u32,
    pub name: String,
    pub symbol_type: SymbolType,
    pub namespace: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymbolType {
    Function,
    Data,
    Label,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompiledFunction {
    pub c_code: String,
    pub high_function: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionData {
    pub address: u32,
    pub mnemonic: String,
    pub operands: Vec<String>,
    pub raw_bytes: Vec<u8>,
}

pub enum GhidraBackend {
    ReOxide,
    HeadlessCli,
}

impl GhidraAnalysis {
    /// Analyze a DOL file using Ghidra.
    ///
    /// # Backend Selection
    /// - **ReOxide**: Automatically installs and uses ReOxide if available
    /// - **HeadlessCli**: Falls back to direct Ghidra headless CLI
    ///
    /// # Arguments
    /// * `dol_path` - Path to DOL file
    /// * `backend` - Backend to use (ReOxide will auto-install if needed)
    ///
    /// # Returns
    /// `Result<GhidraAnalysis>` - Analysis results
    #[inline] // May be called frequently
    pub fn analyze(dol_path: &str, backend: GhidraBackend) -> Result<Self> {
        match backend {
            GhidraBackend::ReOxide => {
                // Try ReOxide first, fallback to HeadlessCli if it fails
                Self::analyze_reoxide(dol_path)
                    .or_else(|e| {
                        log::warn!("ReOxide analysis failed: {}. Falling back to HeadlessCli.", e);
                        Self::analyze_headless(dol_path)
                    })
            }
            GhidraBackend::HeadlessCli => Self::analyze_headless(dol_path),
        }
    }

    /// Analyze using ReOxide (Python tool for enhanced Ghidra integration).
    ///
    /// # Algorithm
    /// 1. Check if ReOxide is installed, install if missing
    /// 2. Initialize ReOxide configuration if needed
    /// 3. Install Ghidra scripts if needed
    /// 4. Use ReOxide to enhance Ghidra analysis
    /// 5. Parse enhanced analysis results
    ///
    /// # Arguments
    /// * `dol_path` - Path to DOL file
    ///
    /// # Returns
    /// `Result<GhidraAnalysis>` - Enhanced analysis results
    #[inline(never)] // Large function - don't inline
    fn analyze_reoxide(dol_path: &str) -> Result<Self> {
        log::info!("Using ReOxide backend for enhanced Ghidra analysis...");
        
        // Step 1: Ensure ReOxide is installed
        Self::ensure_reoxide_installed()?;
        
        // Step 2: Ensure ReOxide is configured
        Self::ensure_reoxide_configured()?;
        
        // Step 3: Ensure Ghidra scripts are installed
        Self::ensure_ghidra_scripts_installed()?;
        
        // Step 4: Use ReOxide-enhanced Ghidra analysis
        // ReOxide works with Ghidra, so we still use analyzeHeadless but with ReOxide scripts
        let dol_path = Path::new(dol_path);
        let project_name = dol_path
            .file_stem()
            .and_then(|s| s.to_str())
            .context("Invalid DOL path")?;

        // Create a temporary project directory
        let project_dir = std::env::temp_dir().join(format!("gcrecomp_reoxide_{}", project_name));
        std::fs::create_dir_all(&project_dir)?;

        // Export directory for Ghidra script output
        let export_dir = project_dir.join("export");
        std::fs::create_dir_all(&export_dir)?;
        std::env::set_var("GHIDRA_EXPORT_DIR", &export_dir);

        // Find Ghidra installation
        let ghidra_path = find_ghidra()?;
        let analyze_headless = ghidra_path.join("support").join("analyzeHeadless");
        
        // Use ReOxide-enhanced export script
        let script_path = find_or_create_reoxide_export_script(&ghidra_path)?;

        // Step 1: Import and analyze with ReOxide enhancements
        log::info!("Importing DOL file into Ghidra with ReOxide...");
        let import_output = Command::new(&analyze_headless)
            .arg(&project_dir)
            .arg(project_name)
            .arg("-import")
            .arg(dol_path)
            .arg("-processor")
            .arg("PowerPC:BE:32:default")
            .arg("-analysis")
            .output()
            .context("Failed to run Ghidra import with ReOxide")?;

        if !import_output.status.success() {
            let stderr = String::from_utf8_lossy(&import_output.stderr);
            log::warn!("Ghidra import warnings: {}", stderr);
        }

        // Step 2: Run ReOxide-enhanced export script
        log::info!("Running ReOxide-enhanced export script...");
        let script_dir = script_path.parent()
            .context("Script path has no parent directory")?;
        let script_name = script_path.file_name()
            .and_then(|n| n.to_str())
            .context("Invalid script filename")?;
        
        let script_output = Command::new(&analyze_headless)
            .arg(&project_dir)
            .arg(project_name)
            .arg("-process")
            .arg("-scriptPath")
            .arg(script_dir)
            .arg("-script")
            .arg(script_name)
            .arg("-deleteProject")
            .output()
            .context("Failed to run ReOxide export script")?;

        if !script_output.status.success() {
            let stderr = String::from_utf8_lossy(&script_output.stderr);
            log::warn!("ReOxide script warnings: {}", stderr);
        }

        // Step 3: Parse exported data (same as headless)
        log::info!("Parsing ReOxide-enhanced exported data...");
        let functions = parse_functions_json(&export_dir)?;
        let symbols = parse_symbols_json(&export_dir)?;
        let decompiled_code = parse_decompiled_json(&export_dir)?;
        let instructions = extract_instructions(&project_dir, project_name)?;

        Ok(Self {
            functions,
            symbols,
            decompiled_code,
            instructions,
        })
    }

    /// Ensure ReOxide is installed, installing it if necessary.
    ///
    /// # Algorithm
    /// 1. Check if `reoxide` command is available
    /// 2. If not, try to install via pipx (preferred) or pip
    /// 3. Verify installation succeeded
    ///
    /// # Returns
    /// `Result<()>` - Success if ReOxide is available
    #[inline] // May be called frequently
    fn ensure_reoxide_installed() -> Result<()> {
        // Check if reoxide is already available
        if Command::new("reoxide")
            .arg("--version")
            .output()
            .is_ok() {
            log::info!("ReOxide is already installed");
            return Ok(());
        }

        log::info!("ReOxide not found. Installing ReOxide...");
        
        // Try pipx first (preferred for CLI tools)
        let install_result = if Command::new("pipx")
            .arg("--version")
            .output()
            .is_ok() {
            log::info!("Installing ReOxide via pipx...");
            Command::new("pipx")
                .arg("install")
                .arg("reoxide")
                .output()
        } else {
            // Fallback to pip
            log::info!("Installing ReOxide via pip...");
            Command::new("pip")
                .arg("install")
                .arg("--user")
                .arg("reoxide")
                .output()
        };

        match install_result {
            Ok(output) if output.status.success() => {
                log::info!("ReOxide installed successfully");
                Ok(())
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("Failed to install ReOxide: {}", stderr);
            }
            Err(e) => {
                anyhow::bail!("Failed to run pip/pipx to install ReOxide: {}", e);
            }
        }
    }

    /// Ensure ReOxide is configured with Ghidra.
    ///
    /// # Algorithm
    /// Runs `reoxide init-config` if configuration doesn't exist.
    ///
    /// # Returns
    /// `Result<()>` - Success if ReOxide is configured
    #[inline] // May be called frequently
    fn ensure_reoxide_configured() -> Result<()> {
        // Check if ReOxide config exists (it creates a config file)
        // For now, we'll just try to run init-config and ignore if it already exists
        let config_result = Command::new("reoxide")
            .arg("init-config")
            .output();

        match config_result {
            Ok(output) if output.status.success() => {
                log::info!("ReOxide configuration initialized");
                Ok(())
            }
            Ok(_) => {
                // Config might already exist, which is fine
                log::debug!("ReOxide configuration already exists or init skipped");
                Ok(())
            }
            Err(e) => {
                log::warn!("Could not initialize ReOxide config: {}. Continuing anyway.", e);
                Ok(()) // Non-fatal, continue
            }
        }
    }

    /// Ensure ReOxide Ghidra scripts are installed.
    ///
    /// # Algorithm
    /// Runs `reoxide install-ghidra-scripts` to install scripts into Ghidra.
    ///
    /// # Returns
    /// `Result<()>` - Success if scripts are installed
    #[inline] // May be called frequently
    fn ensure_ghidra_scripts_installed() -> Result<()> {
        log::info!("Installing ReOxide Ghidra scripts...");
        
        let script_result = Command::new("reoxide")
            .arg("install-ghidra-scripts")
            .output()
            .context("Failed to run reoxide install-ghidra-scripts")?;

        if script_result.status.success() {
            log::info!("ReOxide Ghidra scripts installed successfully");
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&script_result.stderr);
            log::warn!("ReOxide script installation had warnings: {}", stderr);
            // Non-fatal, continue anyway
            Ok(())
        }
    }

    fn analyze_headless(dol_path: &str) -> Result<Self> {
        let dol_path = Path::new(dol_path);
        let project_name = dol_path
            .file_stem()
            .and_then(|s| s.to_str())
            .context("Invalid DOL path")?;

        // Create a temporary project directory
        let project_dir = std::env::temp_dir().join(format!("gcrecomp_{}", project_name));
        std::fs::create_dir_all(&project_dir)?;

        // Export directory for Ghidra script output
        let export_dir = project_dir.join("export");
        std::fs::create_dir_all(&export_dir)?;
        std::env::set_var("GHIDRA_EXPORT_DIR", &export_dir);

        // Find Ghidra installation
        let ghidra_path = find_ghidra()?;
        let analyze_headless = ghidra_path.join("support").join("analyzeHeadless");
        let script_path = find_or_create_export_script(&ghidra_path)?;

        // Step 1: Import and analyze
        log::info!("Importing DOL file into Ghidra...");
        let import_output = Command::new(&analyze_headless)
            .arg(&project_dir)
            .arg(project_name)
            .arg("-import")
            .arg(dol_path)
            .arg("-processor")
            .arg("PowerPC:BE:32:default")
            .arg("-analysis")
            .output()
            .context("Failed to run Ghidra import")?;

        if !import_output.status.success() {
            let stderr = String::from_utf8_lossy(&import_output.stderr);
            log::warn!("Ghidra import warnings: {}", stderr);
        }

        // Step 2: Run export script
        log::info!("Running Ghidra export script...");
        let script_dir = script_path.parent()
            .context("Script path has no parent directory")?;
        let script_name = script_path.file_name()
            .and_then(|n| n.to_str())
            .context("Invalid script filename")?;
        
        let script_output = Command::new(&analyze_headless)
            .arg(&project_dir)
            .arg(project_name)
            .arg("-process")
            .arg("-scriptPath")
            .arg(script_dir)
            .arg("-script")
            .arg(script_name)
            .arg("-deleteProject")
            .output()
            .context("Failed to run Ghidra export script")?;

        if !script_output.status.success() {
            let stderr = String::from_utf8_lossy(&script_output.stderr);
            log::warn!("Ghidra script warnings: {}", stderr);
        }

        // Step 3: Parse exported data
        log::info!("Parsing exported data...");
        let functions = parse_functions_json(&export_dir)?;
        let symbols = parse_symbols_json(&export_dir)?;
        let decompiled_code = parse_decompiled_json(&export_dir)?;
        let instructions = extract_instructions(&project_dir, project_name)?;

        // Note: Cleanup is handled by -deleteProject flag in script execution

        Ok(Self {
            functions,
            symbols,
            decompiled_code,
            instructions,
        })
    }

    pub fn get_function_at_address(&self, address: u32) -> Option<&FunctionInfo> {
        self.functions
            .iter()
            .find(|f| f.address <= address && address < f.address + f.size)
    }
}

fn find_ghidra() -> Result<std::path::PathBuf> {
    // Check common Ghidra installation locations
    let common_paths: Vec<std::path::PathBuf> = vec![
        "/usr/local/ghidra".into(),
        "/opt/ghidra".into(),
        "/Applications/ghidra".into(),
    ];

    // Also check environment variable
    let env_path = std::env::var("GHIDRA_INSTALL_DIR").ok().map(std::path::PathBuf::from);

    let all_paths = common_paths.into_iter().chain(env_path);

    for path in all_paths {
        let ghidra_path = Path::new(&path);
        if ghidra_path.join("support").join("analyzeHeadless").exists() {
            return Ok(ghidra_path.to_path_buf());
        }
    }

    anyhow::bail!(
        "Ghidra not found. Please set GHIDRA_INSTALL_DIR environment variable or install Ghidra in a standard location."
    );
}

fn find_or_create_export_script(ghidra_path: &Path) -> Result<PathBuf> {
    // Check if script exists in scripts directory
    let script_path = PathBuf::from("scripts/ghidra_export.py");
    if script_path.exists() {
        return Ok(script_path);
    }

    // Try to find it in Ghidra scripts directory
    let ghidra_scripts = ghidra_path.join("Ghidra").join("Features").join("Python").join("ghidra_scripts");
    if ghidra_scripts.exists() {
        let script = ghidra_scripts.join("ghidra_export.py");
        if script.exists() {
            return Ok(script);
        }
    }

    // Create the script if it doesn't exist
    let script_content = include_str!("../../scripts/ghidra_export.py");
    std::fs::write(&script_path, script_content)
        .context("Failed to create Ghidra export script")?;
    
    Ok(script_path)
}

/// Find or create ReOxide-enhanced export script.
///
/// # Algorithm
/// 1. First, try to find ReOxide scripts in the user's ghidra_scripts directory
/// 2. Fallback to standard export script if ReOxide scripts not found
///
/// # Returns
/// `Result<PathBuf>` - Path to ReOxide export script or fallback to standard script
#[inline] // May be called frequently
fn find_or_create_reoxide_export_script(ghidra_path: &Path) -> Result<PathBuf> {
    // First, try to find ReOxide scripts in the user's ghidra_scripts directory
    let home_dir = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok();
    
    if let Some(home) = home_dir {
        let reoxide_script = PathBuf::from(&home)
            .join("ghidra_scripts")
            .join("reoxide_export.py");
        if reoxide_script.exists() {
            log::info!("Found ReOxide export script at: {}", reoxide_script.display());
            return Ok(reoxide_script);
        }
    }

    // Fallback to our standard export script
    log::debug!("ReOxide export script not found, using standard export script");
    find_or_create_export_script(ghidra_path)
}

fn parse_functions_json(export_dir: &Path) -> Result<Vec<FunctionInfo>> {
    let json_path = export_dir.join("functions.json");
    if !json_path.exists() {
        log::warn!("functions.json not found, returning empty vector");
        return Ok(vec![]);
    }

    let content = std::fs::read_to_string(&json_path)
        .context("Failed to read functions.json")?;
    
    let raw_functions: Vec<serde_json::Value> = serde_json::from_str(&content)
        .context("Failed to parse functions.json")?;

    let mut functions = Vec::new();
    for func in raw_functions {
        let address_str = func["address"].as_str()
            .context("Missing address in function")?;
        let address = parse_address(address_str)?;

        let parameters: Vec<ParameterInfo> = func["parameters"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|p| ParameterInfo {
                name: p["name"].as_str().unwrap_or("").to_string(),
                param_type: p["type"].as_str().unwrap_or("u32").to_string(),
                offset: p["offset"].as_i64().map(|o| o as i32),
            })
            .collect();

        let local_vars: Vec<LocalVariableInfo> = func["local_variables"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|v| LocalVariableInfo {
                name: v["name"].as_str().unwrap_or("").to_string(),
                var_type: v["type"].as_str().unwrap_or("u32").to_string(),
                offset: v["offset"].as_i64().unwrap_or(0) as i32,
                address: v["address"].as_str().unwrap_or("").to_string(),
            })
            .collect();

        let basic_blocks: Vec<BasicBlockInfo> = func["basic_blocks"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|b| BasicBlockInfo {
                address: b["address"].as_str().unwrap_or("").to_string(),
                size: b["size"].as_u64().unwrap_or(0) as u32,
                instructions: b["instructions"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|i| i.as_str().unwrap_or("").to_string())
                    .collect(),
            })
            .collect();

        functions.push(FunctionInfo {
            address,
            name: func["name"].as_str().unwrap_or("unknown").to_string(),
            size: func["size"].as_u64().unwrap_or(0) as u32,
            calling_convention: func["calling_convention"].as_str().unwrap_or("default").to_string(),
            parameters,
            return_type: func["return_type"].as_str().map(|s| s.to_string()),
            local_variables: local_vars,
            basic_blocks,
        });
    }

    Ok(functions)
}

fn parse_symbols_json(export_dir: &Path) -> Result<Vec<SymbolInfo>> {
    let json_path = export_dir.join("symbols.json");
    if !json_path.exists() {
        log::warn!("symbols.json not found, returning empty vector");
        return Ok(vec![]);
    }

    let content = std::fs::read_to_string(&json_path)
        .context("Failed to read symbols.json")?;
    
    let raw_symbols: Vec<serde_json::Value> = serde_json::from_str(&content)
        .context("Failed to parse symbols.json")?;

    let mut symbols = Vec::new();
    for sym in raw_symbols {
        let address_str = sym["address"].as_str()
            .context("Missing address in symbol")?;
        let address = parse_address(address_str)?;

        let symbol_type = match sym["type"].as_str().unwrap_or("Unknown") {
            "Function" => SymbolType::Function,
            "Data" => SymbolType::Data,
            "Label" => SymbolType::Label,
            _ => SymbolType::Unknown,
        };

        symbols.push(SymbolInfo {
            address,
            name: sym["name"].as_str().unwrap_or("unknown").to_string(),
            symbol_type,
            namespace: sym["namespace"].as_str().map(|s| s.to_string()),
        });
    }

    Ok(symbols)
}

fn parse_decompiled_json(export_dir: &Path) -> Result<HashMap<u32, DecompiledFunction>> {
    let json_path = export_dir.join("decompiled.json");
    if !json_path.exists() {
        log::warn!("decompiled.json not found, returning empty map");
        return Ok(HashMap::new());
    }

    let content = std::fs::read_to_string(&json_path)
        .context("Failed to read decompiled.json")?;
    
    let raw_decompiled: HashMap<String, serde_json::Value> = serde_json::from_str(&content)
        .context("Failed to parse decompiled.json")?;

    let mut decompiled = HashMap::new();
    for (addr_str, func_data) in raw_decompiled {
        let address = parse_address(&addr_str)?;
        decompiled.insert(address, DecompiledFunction {
            c_code: func_data["c_code"].as_str().unwrap_or("").to_string(),
            high_function: func_data["high_function"].as_str().unwrap_or("").to_string(),
        });
    }

    Ok(decompiled)
}

fn extract_instructions(_project_dir: &Path, _project_name: &str) -> Result<HashMap<u32, Vec<InstructionData>>> {
    // TODO: Extract instruction-level data from Ghidra
    // This would require parsing the listing or using a script
    Ok(HashMap::new())
}

fn parse_address(addr_str: &str) -> Result<u32> {
    // Handle formats like "0x80000000" or "80000000"
    let cleaned = addr_str.trim_start_matches("0x").trim_start_matches("0X");
    u32::from_str_radix(cleaned, 16)
        .or_else(|_| cleaned.parse::<u32>())
        .context(format!("Failed to parse address: {}", addr_str))
}

