//! Complete Recompilation Pipeline
//!
//! This module orchestrates the complete recompilation process from DOL file to Rust code.
//! It coordinates all analysis and code generation stages in the correct order.
//!
//! # System Architecture
//!
//! GCRecomp is a static recompiler that translates GameCube PowerPC binaries (DOL files)
//! into optimized Rust code. The system consists of several stages:
//!
//! ```
//! DOL File → Parser → Ghidra Analysis → Decoder → Analysis → Code Generator → Validator → Rust Code
//! ```
//!
//! # Pipeline Stages
//!
//! 1. **Ghidra Analysis**: Extract function metadata, symbols, and type information
//! 2. **Instruction Decoding**: Decode PowerPC instructions from binary
//! 3. **Control Flow Analysis**: Build control flow graph (CFG)
//! 4. **Data Flow Analysis**: Build def-use chains and perform live variable analysis
//! 5. **Type Inference**: Recover type information for registers and variables
//! 6. **Code Generation**: Generate Rust code from analyzed instructions
//! 7. **Validation**: Validate generated Rust code
//! 8. **Output**: Write generated code to file
//!
//! # Core Components
//!
//! ## Parser Module
//! - Parses DOL file format and extracts executable sections
//! - Reads DOL file header and section metadata
//! - Loads section data into memory
//!
//! ## Decoder Module
//! - Decodes PowerPC instructions from binary format
//! - Identifies instruction types (arithmetic, branch, load/store, etc.)
//! - Extracts operands (registers, immediates, addresses)
//!
//! ## Analysis Module
//! - **Control Flow Analysis**: Builds CFG, identifies basic blocks, detects loops
//! - **Data Flow Analysis**: Builds def-use chains, performs live variable analysis
//! - **Type Inference**: Recovers type information for registers and variables
//! - **Pointer Analysis**: Tracks points-to sets and aliasing
//! - **Struct Detection**: Infers struct layouts from memory access patterns
//!
//! ## Code Generator
//! - Converts PowerPC instructions to Rust code
//! - Generates function signatures with proper types
//! - Integrates with runtime system
//! - Supports SIMD code generation
//!
//! ## Optimizer
//! - Constant folding
//! - Dead code elimination
//! - Loop optimizations (unrolling, invariant code motion, fusion)
//! - Function inlining
//!
//! # Memory Optimizations
//! - Pre-allocate vectors with known capacity where possible
//! - Use `SmallVec` for temporary instruction lists
//! - Avoid unnecessary clones (use references where possible)
//! - Reuse buffers for string concatenation

use crate::recompiler::analysis::control_flow::ControlFlowAnalyzer;
use crate::recompiler::analysis::data_flow::DataFlowAnalyzer;
use crate::recompiler::analysis::inter_procedural::InterProceduralAnalyzer;
use crate::recompiler::analysis::type_inference::TypeInferenceEngine;
use crate::recompiler::codegen::CodeGenerator;
use crate::recompiler::optimizer::inlining::analyze_inlining_candidates;
use crate::recompiler::decoder::DecodedInstruction;
use crate::recompiler::ghidra::GhidraAnalysis;
use crate::recompiler::linker::LinkerScript;
use crate::recompiler::parser::DolFile;
use crate::recompiler::structure::StructureGenerator;
use crate::recompiler::validator::CodeValidator;
use anyhow::Result;
use smallvec::SmallVec;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Recompilation pipeline orchestrator.
///
/// Coordinates all stages of the recompilation process from DOL file parsing
/// to Rust code generation.
pub struct RecompilationPipeline;

impl RecompilationPipeline {
    /// Recompile a DOL file to Rust code.
    ///
    /// # Algorithm
    /// Executes the complete recompilation pipeline:
    /// 1. Analyze with Ghidra to extract metadata
    /// 2. Decode all PowerPC instructions
    /// 3. Build control flow graph
    /// 4. Perform data flow analysis
    /// 5. Infer types
    /// 6. Generate Rust code
    /// 7. Validate generated code
    /// 8. Write output to file
    ///
    /// # Arguments
    /// * `dol_file` - Parsed DOL file structure
    /// * `output_path` - Path to output Rust file
    ///
    /// # Returns
    /// `Result<()>` - Success, or error if any stage fails
    ///
    /// # Errors
    /// Returns error if any pipeline stage fails (Ghidra analysis, decoding, codegen, etc.)
    ///
    /// # Examples
    /// ```rust
    /// let dol_file = DolFile::parse("game.dol")?;
    /// RecompilationPipeline::recompile(&dol_file, "output.rs")?;
    /// ```
    #[inline(never)] // Large function - don't inline
    pub fn recompile(dol_file: &DolFile, output_path: &str) -> Result<()> {
        log::info!("Starting recompilation pipeline...");

        // Step 1: Analyze with Ghidra (try ReOxide first, fallback to HeadlessCli)
        log::info!("Step 1: Running Ghidra analysis (trying ReOxide first)...");
        let ghidra_analysis: GhidraAnalysis = GhidraAnalysis::analyze(
            &dol_file.path,
            crate::recompiler::ghidra::GhidraBackend::ReOxide, // Auto-installs if needed, falls back to HeadlessCli
        )?;

        // Step 2: Decode instructions
        log::info!("Step 2: Decoding instructions...");
        let instructions: Vec<DecodedInstruction> = Self::decode_all_instructions(dol_file)?;

        // Step 3: Control flow analysis
        log::info!("Step 3: Building control flow graph...");
        let cfg = ControlFlowAnalyzer::build_cfg(&instructions, 0u32)?;

        // Step 3.5: Loop optimizations
        log::info!("Step 3.5: Optimizing loops...");
        let mut optimized_instructions = crate::recompiler::optimizer::loop_opt::optimize_loops(&instructions);
        log::info!("Loop optimization complete: {} -> {} instructions", instructions.len(), optimized_instructions.len());

        // Step 4: Data flow analysis (on optimized instructions)
        log::info!("Step 4: Performing data flow analysis...");
        let def_use_chains = DataFlowAnalyzer::build_def_use_chains(&optimized_instructions);
        let live_analysis = DataFlowAnalyzer::live_variable_analysis(&cfg);

        // Step 4.5: Inter-procedural analysis
        log::info!("Step 4.5: Building call graph...");
        let call_graph = InterProceduralAnalyzer::build_call_graph(&ghidra_analysis.functions);
        let unreachable = InterProceduralAnalyzer::find_unreachable_functions(&call_graph);
        log::info!("Found {} unreachable functions", unreachable.len());
        
        // Build function size map for inlining analysis
        let function_sizes: HashMap<u32, usize> = ghidra_analysis
            .functions
            .iter()
            .map(|f| (f.address, f.size as usize))
            .collect();
        
        // Find inlining candidates
        let inlining_candidates = analyze_inlining_candidates(&call_graph, &function_sizes);
        log::info!("Found {} inlining candidates", inlining_candidates.len());

        // Step 5: Type inference
        log::info!("Step 5: Inferring types...");
        let type_inference_engine = TypeInferenceEngine;

        // Step 6: Code generation
        log::info!("Step 6: Generating Rust code...");
        let mut codegen: CodeGenerator = CodeGenerator::new();

        // Pre-allocate string buffer with estimated capacity
        // Estimate: ~1000 bytes per function on average
        let estimated_capacity: usize = ghidra_analysis.functions.len() * 1000usize;
        let mut rust_code: String = String::with_capacity(estimated_capacity);

        // Add module header
        rust_code.push_str("//! Recompiled GameCube game functions\n");
        rust_code.push_str("//! Generated by GCRecomp\n\n");
        rust_code.push_str("use crate::runtime::context::CpuContext;\n");
        rust_code.push_str("use crate::runtime::memory::MemoryManager;\n");
        rust_code.push_str("use anyhow::Result;\n\n");

        // Filter out unreachable functions (dead code elimination)
        let reachable_functions: Vec<_> = ghidra_analysis
            .functions
            .iter()
            .enumerate()
            .filter(|(idx, _)| !unreachable.contains(idx))
            .map(|(_, func)| func)
            .collect();
        
        let total_functions: usize = reachable_functions.len();
        let mut successful_functions: usize = 0usize;
        let mut failed_functions: usize = 0usize;

        for (idx, func) in reachable_functions.iter().enumerate() {
            // Progress reporting
            if idx % 10 == 0 || idx == total_functions - 1 {
                log::info!(
                    "Generating code for function {}/{} ({}%) - {}",
                    idx + 1,
                    total_functions,
                    ((idx + 1) * 100) / total_functions.max(1),
                    func.name
                );
            }

            // Get instructions for this function using address-based mapping
            let func_instructions: Vec<DecodedInstruction> =
                Self::map_instructions_to_function(func, &instructions);

            if func_instructions.is_empty() {
                log::warn!(
                    "Function {} at 0x{:08X} has no instructions, skipping",
                    func.name,
                    func.address
                );
                failed_functions += 1;
                continue;
            }

            // Convert Ghidra types to TypeInfo
            let convert_ghidra_type = |type_str: &str| -> crate::recompiler::analysis::TypeInfo {
                match type_str {
                    "int" | "i32" => crate::recompiler::analysis::TypeInfo::Integer { signed: true, size: 32 },
                    "uint" | "u32" => crate::recompiler::analysis::TypeInfo::Integer { signed: false, size: 32 },
                    "float" | "f32" => crate::recompiler::analysis::TypeInfo::Float { size: 32 },
                    "double" | "f64" => crate::recompiler::analysis::TypeInfo::Float { size: 64 },
                    "void" => crate::recompiler::analysis::TypeInfo::Void,
                    _ if type_str.contains('*') => {
                        // Pointer type
                        let pointee = type_str.trim_end_matches('*').trim();
                        crate::recompiler::analysis::TypeInfo::Pointer {
                            pointee: Box::new(convert_ghidra_type(pointee)),
                        }
                    }
                    _ => crate::recompiler::analysis::TypeInfo::Unknown,
                }
            };
            
            // Run type inference for this function
            let temp_metadata = crate::recompiler::analysis::FunctionMetadata {
                address: func.address,
                name: func.name.clone(),
                size: func.size,
                calling_convention: func.calling_convention.clone(),
                parameters: func
                    .parameters
                    .iter()
                    .map(|p| crate::recompiler::analysis::ParameterInfo {
                        name: p.name.clone(),
                        type_info: convert_ghidra_type(&p.param_type),
                        register: None,
                        stack_offset: p.offset,
                    })
                    .collect(),
                return_type: func.return_type.as_ref().map(|t| convert_ghidra_type(t)),
                local_variables: func
                    .local_variables
                    .iter()
                    .map(|v| crate::recompiler::analysis::VariableInfo {
                        name: v.name.clone(),
                        type_info: convert_ghidra_type(&v.var_type),
                        stack_offset: v.offset,
                        scope_start: 0u32,
                        scope_end: 0u32,
                    })
                    .collect(),
                basic_blocks: vec![],
            };
            
            let register_types = type_inference_engine.infer_types(&func_instructions, &temp_metadata);
            
            // Helper to convert InferredType to TypeInfo
            let inferred_to_type_info = |inf_type: &crate::recompiler::analysis::InferredType| -> crate::recompiler::analysis::TypeInfo {
                match inf_type {
                    crate::recompiler::analysis::InferredType::Integer { signed, size } => {
                        crate::recompiler::analysis::TypeInfo::Integer { signed: *signed, size: *size }
                    }
                    crate::recompiler::analysis::InferredType::Float { size } => {
                        crate::recompiler::analysis::TypeInfo::Float { size: *size }
                    }
                    crate::recompiler::analysis::InferredType::Pointer { pointee } => {
                        crate::recompiler::analysis::TypeInfo::Pointer {
                            pointee: Box::new(inferred_to_type_info(pointee)),
                        }
                    }
                    crate::recompiler::analysis::InferredType::Unknown => {
                        crate::recompiler::analysis::TypeInfo::Unknown
                    }
                }
            };
            
            // Generate function code with type information
            let func_metadata = crate::recompiler::analysis::FunctionMetadata {
                address: func.address,
                name: func.name.clone(),
                size: func.size,
                calling_convention: func.calling_convention.clone(),
                parameters: func
                    .parameters
                    .iter()
                    .enumerate()
                    .map(|(idx, p)| {
                        // Use inferred type if available, otherwise use metadata type
                        let inferred_type = if idx < 32 {
                            register_types.get(&(idx as u8))
                        } else {
                            None
                        };
                        crate::recompiler::analysis::ParameterInfo {
                            name: p.name.clone(),
                            type_info: if let Some(inf_type) = inferred_type {
                                inferred_to_type_info(inf_type)
                            } else {
                                convert_ghidra_type(&p.param_type)
                            },
                            register: Some(idx as u8),
                            stack_offset: p.offset,
                        }
                    })
                    .collect(),
                return_type: func.return_type.as_ref().map(|t| convert_ghidra_type(t)),
                local_variables: func
                    .local_variables
                    .iter()
                    .map(|v| crate::recompiler::analysis::VariableInfo {
                        name: v.name.clone(),
                        type_info: convert_ghidra_type(&v.var_type),
                        stack_offset: v.offset,
                        scope_start: 0u32,
                        scope_end: 0u32,
                    })
                    .collect(),
                basic_blocks: vec![],
            };

            // Use optimized instructions for code generation
            match codegen.generate_function(&func_metadata, &func_instructions_optimized) {
                Ok(func_code) => {
                    rust_code.push_str(&func_code);
                    rust_code.push('\n');
                    successful_functions += 1;
                }
                Err(e) => {
                    log::warn!(
                        "Failed to generate code for function {} at 0x{:08X}: {}",
                        func.name,
                        func.address,
                        e
                    );
                    failed_functions += 1;
                    // Generate a stub function instead
                    rust_code.push_str(&format!(
                        "// Stub for function {} at 0x{:08X} (generation failed: {})\n",
                        func.name, func.address, e
                    ));
                    rust_code.push_str(&format!(
                        "pub fn {}_0x{:08X}(_ctx: &mut CpuContext, _memory: &mut MemoryManager) -> Result<Option<u32>> {{\n",
                        codegen.sanitize_identifier(&func.name),
                        func.address
                    ));
                    rust_code
                        .push_str("    log::warn!(\"Function stub called - not implemented\");\n");
                    rust_code.push_str("    Ok(None)\n");
                    rust_code.push_str("}\n\n");
                }
            }
        }

        log::info!(
            "Code generation complete: {} successful, {} failed out of {} total functions",
            successful_functions,
            failed_functions,
            total_functions
        );

        // Use optimized HashMap-based dispatcher instead of match
        // This provides O(1) lookup instead of O(n) match statement
        let dispatcher_content = Self::generate_function_dispatcher(&ghidra_analysis.functions)?;
        rust_code.push_str(&dispatcher_content);

        // Add function address mappings (only for reachable functions)
        for func in reachable_functions.iter() {
            let func_name = if func.name.is_empty() || func.name.starts_with("sub_") {
                format!("func_0x{:08X}", func.address)
            } else {
                format!(
                    "{}_{:08X}",
                    codegen.sanitize_identifier(&func.name),
                    func.address
                )
            };
            rust_code.push_str(&format!(
                "        0x{:08X}u32 => {}(ctx, memory),\n",
                func.address, func_name
            ));
        }

        rust_code.push_str("        _ => {\n");
        rust_code
            .push_str("            log::warn!(\"Unknown function address: 0x{:08X}\", address);\n");
        rust_code.push_str("            Ok(None)\n");
        rust_code.push_str("        }\n");
        rust_code.push_str("    }\n");
        rust_code.push_str("}\n\n");

        // Step 7: Validation
        log::info!("Step 7: Validating generated code...");
        CodeValidator::validate_rust_code(&rust_code)?;

        // Step 8: Write output
        log::info!("Step 8: Writing output to {}...", output_path);
        std::fs::write(output_path, rust_code)?;

        log::info!("Recompilation complete!");
        Ok(())
    }

    /// Recompile a DOL file to hierarchical Rust code structure.
    ///
    /// This is the enhanced recompilation pipeline that leverages Ghidra's full capabilities
    /// for automatic symbol extraction and organizes output into a hierarchical file structure
    /// similar to N64 decompilation projects.
    ///
    /// # Pipeline Stages
    ///
    /// 1. **Enhanced Ghidra Analysis**:
    ///    - Runs auto-analyzers (DecompilerParameterID, Reference, etc.)
    ///    - Applies Function ID databases to identify known SDK functions
    ///    - Optionally runs BSim for fuzzy function matching
    ///    - Extracts function metadata, symbols, and type information
    ///
    /// 2. **Symbol Resolution & Namespace Detection**:
    ///    - Detects namespace hierarchy from symbols
    ///    - Assigns confidence scores to symbol matches
    ///    - Identifies SDK patterns (GX*, VI*, OS*, etc.)
    ///
    /// 3. **Linker Script Processing**:
    ///    - Parses linker script to understand memory layout
    ///    - Applies namespace organization rules
    ///    - Maps functions to modules based on patterns
    ///
    /// 4. **Function Organization**:
    ///    - Groups functions by namespace/module using linker rules
    ///    - Builds hierarchical module structure
    ///    - Generates file names for each function
    ///
    /// 5. **Per-Function Recompilation**:
    ///    - Decodes PowerPC instructions for each function
    ///    - Generates individual Rust function files
    ///    - Includes proper module imports and dependencies
    ///
    /// 6. **Module Structure Generation**:
    ///    - Creates `mod.rs` files for each module
    ///    - Generates function dispatcher for indirect calls
    ///    - Writes all files to disk in hierarchical structure
    ///
    /// # Arguments
    /// * `dol_file` - Parsed DOL file structure containing binary data
    /// * `output_dir` - Base output directory for hierarchical structure (e.g., "game/src/recompiled")
    /// * `linker_script_path` - Optional path to linker script defining organization rules
    /// * `fidb_path` - Optional path to Function ID database for SDK function identification
    /// * `enable_bsim` - Whether to enable BSim fuzzy matching (requires external database)
    ///
    /// # Returns
    /// `Result<()>` - Success, or error if any stage fails
    ///
    /// # Example Output Structure
    /// ```
    /// game/src/recompiled/
    /// ├── mod.rs
    /// ├── dispatcher.rs
    /// ├── graphics/
    /// │   ├── mod.rs
    /// │   ├── gx/
    /// │   │   ├── mod.rs
    /// │   │   ├── GXInit_0x80012345.rs
    /// │   │   └── GXDraw_0x80012389.rs
    /// │   └── vi/
    /// │       ├── mod.rs
    /// │       └── VIInit_0x80023456.rs
    /// └── system/
    ///     └── ...
    /// ```
    ///
    /// # Errors
    /// Returns error if:
    /// - Ghidra analysis fails
    /// - Linker script parsing fails
    /// - Instruction decoding fails
    /// - Code generation fails for critical functions
    /// - File system operations fail
    #[inline(never)] // Large function - don't inline
    pub fn recompile_hierarchical(
        dol_file: &DolFile,
        output_dir: &Path,
        linker_script_path: Option<&Path>,
        _fidb_path: Option<&Path>,
        _enable_bsim: bool,
    ) -> Result<()> {
        log::info!("Starting hierarchical recompilation pipeline...");

        // ============================================================================
        // STEP 1: Enhanced Ghidra Analysis
        // ============================================================================
        // Run Ghidra with ReOxide backend (auto-installs if needed, falls back to HeadlessCli)
        // This extracts functions, symbols, decompiled code, and instruction data
        log::info!("Step 1: Running enhanced Ghidra analysis...");
        let mut ghidra_analysis = GhidraAnalysis::analyze(
            &dol_file.path,
            crate::recompiler::ghidra::GhidraBackend::ReOxide,
        )?;

        // Apply Function ID database if provided
        // Function ID uses hash-based matching to identify known SDK functions
        // This is crucial for stripped binaries where symbols are missing
        if let Some(fidb_path) = _fidb_path {
            log::info!("Applying Function ID database...");
            ghidra_analysis.apply_function_id(fidb_path)?;
        }

        // Run BSim analysis if enabled
        // BSim performs fuzzy matching to find equivalent functions across binaries
        // Useful for handling compiler variations and optimizations
        if _enable_bsim {
            log::info!("Running BSim analysis...");
            if let Some(bsim_path) = _fidb_path {
                ghidra_analysis.run_bsim_analysis(bsim_path)?;
            }
        }

        // Detect namespace hierarchy from symbols
        // Extracts namespace information to organize functions into logical groups
        log::info!("Detecting namespaces...");
        let namespaces = ghidra_analysis.detect_namespaces();
        log::info!("Found {} namespaces", namespaces.len());

        // ============================================================================
        // STEP 2: Linker Script Processing
        // ============================================================================
        // Parse linker script to understand memory layout and organization rules
        // Linker script defines:
        // - Memory regions (TEXT, DATA, BSS)
        // - Section mappings
        // - Namespace patterns (e.g., "GX*" -> "graphics/gx")
        let linker_script = if let Some(linker_path) = linker_script_path {
            log::info!("Step 2: Parsing linker script...");
            Some(LinkerScript::from_file(linker_path)?)
        } else {
            log::info!("Step 2: No linker script provided, using defaults");
            // Without linker script, functions are organized by detected namespaces
            // or fall back to "unknown" module
            None
        };

        // ============================================================================
        // STEP 3: Instruction Decoding
        // ============================================================================
        // Decode all PowerPC instructions from the DOL file
        // Instructions are decoded with their addresses for mapping to functions
        log::info!("Step 3: Decoding instructions...");
        let instructions = Self::decode_all_instructions(dol_file)?;
        log::info!("Decoded {} instructions", instructions.len());

        // ============================================================================
        // STEP 4: Function Organization
        // ============================================================================
        // Organize functions into hierarchical structure based on:
        // 1. Linker script rules (if provided)
        // 2. Detected namespaces from Ghidra
        // 3. SDK pattern matching (GX*, VI*, OS*, etc.)
        log::info!("Step 4: Organizing functions into hierarchical structure...");
        let structure_gen = StructureGenerator::new(linker_script, output_dir.to_path_buf());
        let modules = structure_gen
            .organize_functions(&ghidra_analysis.functions, &ghidra_analysis.symbols)?;
        log::info!("Organized into {} modules", modules.len());

        // ============================================================================
        // STEP 5: Per-Function Code Generation
        // ============================================================================
        // Generate individual Rust function files for each function
        // Each function gets its own file with:
        // - Proper module imports
        // - Function signature matching PowerPC calling convention
        // - Translated PowerPC instructions to Rust
        // - Error handling and logging
        log::info!("Step 5: Generating Rust code for each function...");
        let mut codegen = CodeGenerator::new();
        let mut function_code_map: HashMap<u32, String> = HashMap::new();
        let total_functions = ghidra_analysis.functions.len();
        let mut successful = 0;
        let mut failed = 0;

        // Process each function individually
        for (idx, func) in ghidra_analysis.functions.iter().enumerate() {
            // Progress reporting every 10 functions or on last function
            if idx % 10 == 0 || idx == total_functions - 1 {
                log::info!(
                    "Generating code for function {}/{} ({}%) - {}",
                    idx + 1,
                    total_functions,
                    ((idx + 1) * 100) / total_functions.max(1),
                    func.name
                );
            }

            // Map instructions to this function by address range
            // Function contains instructions from func.address to func.address + func.size
            let func_instructions = Self::map_instructions_to_function(func, &instructions);
            if func_instructions.is_empty() {
                log::warn!(
                    "Function {} at 0x{:08X} has no instructions, skipping",
                    func.name,
                    func.address
                );
                failed += 1;
                continue;
            }

            // Get module path for this function (used for import generation)
            // Module path determines where the function file will be written
            let module_path = func.module_path.as_deref();

            // Create function metadata from Ghidra analysis
            // This includes function name, address, size, calling convention, parameters, etc.
            let func_metadata = crate::recompiler::analysis::FunctionMetadata {
                address: func.address,
                name: func.name.clone(),
                size: func.size,
                calling_convention: func.calling_convention.clone(),
                parameters: func
                    .parameters
                    .iter()
                    .map(|p| crate::recompiler::analysis::ParameterInfo {
                        name: p.name.clone(),
                        type_info: crate::recompiler::analysis::TypeInfo::Unknown,
                    })
                    .collect(),
                return_type: func
                    .return_type
                    .clone()
                    .map(|_| crate::recompiler::analysis::TypeInfo::Unknown),
            };

            // Generate complete function file with imports and code
            // The code generator translates PowerPC instructions to Rust code
            match codegen.generate_function_file(&func_metadata, &func_instructions, module_path) {
                Ok(code) => {
                    // Store generated code keyed by function address
                    // This allows the structure generator to write files in the correct locations
                    function_code_map.insert(func.address, code);
                    successful += 1;
                }
                Err(e) => {
                    // Log warning but continue processing other functions
                    // Non-critical failures don't stop the entire recompilation
                    log::warn!(
                        "Failed to generate code for function {} at 0x{:08X}: {}",
                        func.name,
                        func.address,
                        e
                    );
                    failed += 1;
                }
            }
        }

        log::info!(
            "Code generation complete: {} successful, {} failed",
            successful,
            failed
        );

        // ============================================================================
        // STEP 6: Module Structure Generation
        // ============================================================================
        // Generate mod.rs files for each module in the hierarchy
        // These files declare submodules and re-export functions
        log::info!("Step 6: Generating module structure...");
        structure_gen.generate_module_tree(&modules)?;

        // ============================================================================
        // STEP 7: Write Function Files
        // ============================================================================
        // Write all generated function files to disk in their respective module directories
        // Each function file is written to: <output_dir>/<module_path>/<function_file_name>
        log::info!("Step 7: Writing function files...");
        structure_gen.write_function_files(&modules, &function_code_map)?;

        // ============================================================================
        // STEP 8: Generate Root Module
        // ============================================================================
        // Create root mod.rs that declares all top-level modules
        // This is the entry point for the recompiled code
        log::info!("Step 8: Generating root mod.rs...");
        let root_mod_rs = output_dir.join("mod.rs");
        let root_mod_content = Self::generate_root_mod_rs(&modules)?;
        std::fs::write(&root_mod_rs, root_mod_content).context("Failed to write root mod.rs")?;

        // ============================================================================
        // STEP 9: Generate Function Dispatcher
        // ============================================================================
        // Create function dispatcher for indirect function calls
        // This allows the runtime to call functions by address (e.g., function pointers)
        // The dispatcher uses a match statement to route calls to the correct function
        log::info!("Step 9: Generating function dispatcher...");
        let dispatcher_path = output_dir.join("dispatcher.rs");
        let dispatcher_content = Self::generate_function_dispatcher(&ghidra_analysis.functions)?;
        std::fs::write(&dispatcher_path, dispatcher_content)
            .context("Failed to write dispatcher.rs")?;

        log::info!("Hierarchical recompilation complete!");
        log::info!("Output directory: {}", output_dir.display());
        log::info!(
            "Total functions: {} successful, {} failed",
            successful,
            failed
        );
        Ok(())
    }

    /// Generate root mod.rs file
    fn generate_root_mod_rs(
        modules: &HashMap<String, crate::recompiler::structure::ModuleInfo>,
    ) -> Result<String> {
        let mut content = String::new();
        content.push_str("//! Auto-generated recompiled module\n");
        content.push_str("//! Generated by GCRecomp\n\n");

        // Add module declarations
        for (path, _) in modules {
            let module_name = path.split('/').last().unwrap_or(path);
            content.push_str(&format!("pub mod {};\n", module_name));
        }

        // Add dispatcher
        content.push_str("\npub mod dispatcher;\n");
        content.push_str("pub use dispatcher::call_function_by_address;\n");

        Ok(content)
    }

    /// Generate function dispatcher (optimized with HashMap)
    fn generate_function_dispatcher(
        functions: &[crate::recompiler::ghidra::FunctionInfo],
    ) -> Result<String> {
        let mut content = String::new();
        content.push_str("//! Function dispatcher for indirect calls\n");
        content.push_str("//! Generated by GCRecomp\n");
        content.push_str("//! Uses HashMap for O(1) lookup performance\n\n");
        content.push_str("use crate::runtime::context::CpuContext;\n");
        content.push_str("use crate::runtime::memory::MemoryManager;\n");
        content.push_str("use anyhow::Result;\n");
        content.push_str("use std::collections::HashMap;\n\n");
        content.push_str("/// Function pointer type\n");
        content.push_str("type FunctionPtr = fn(&mut CpuContext, &mut MemoryManager) -> Result<Option<u32>>;\n\n");
        content.push_str("/// Static function table (lazy initialized)\n");
        content.push_str("static mut FUNCTION_TABLE: Option<HashMap<u32, FunctionPtr>> = None;\n");
        content.push_str("static INIT_ONCE: std::sync::Once = std::sync::Once::new();\n\n");
        content.push_str("/// Initialize function table\n");
        content.push_str("fn init_function_table() -> &'static HashMap<u32, FunctionPtr> {\n");
        content.push_str("    unsafe {\n");
        content.push_str("        INIT_ONCE.call_once(|| {\n");
        content.push_str("            let mut table = HashMap::new();\n");
        
        // Add all functions to the table
        for func in functions {
            let func_name = if func.name.is_empty() || func.name.starts_with("sub_") {
                format!("func_0x{:08X}", func.address)
            } else {
                format!(
                    "{}_{:08X}",
                    func.name.replace("::", "_").replace(" ", "_"),
                    func.address
                )
            };
            content.push_str(&format!(
                "            table.insert(0x{:08X}, {} as FunctionPtr);\n",
                func.address, func_name
            ));
        }
        
        content.push_str("            FUNCTION_TABLE = Some(table);\n");
        content.push_str("        });\n");
        content.push_str("        FUNCTION_TABLE.as_ref().expect(\"Function table not initialized\")\n");
        content.push_str("    }\n");
        content.push_str("}\n\n");
        content.push_str("/// Dispatch function calls by address\n");
        content.push_str("pub fn call_function_by_address(\n");
        content.push_str("    address: u32,\n");
        content.push_str("    ctx: &mut CpuContext,\n");
        content.push_str("    memory: &mut MemoryManager,\n");
        content.push_str(") -> Result<Option<u32>> {\n");
        content.push_str("    let table = init_function_table();\n");
        content.push_str("    match table.get(&address) {\n");
        content.push_str("        Some(func_ptr) => func_ptr(ctx, memory),\n");
        content.push_str("        None => {\n");
        content.push_str("            log::warn!(\"Unknown function address: 0x{:08X}\", address);\n");
        content.push_str("            Ok(None)\n");
        content.push_str("        }\n");
        content.push_str("    }\n");
        content.push_str("}\n");

        for func in functions {
            let func_name = if func.name.is_empty() || func.name.starts_with("sub_") {
                format!("func_0x{:08X}", func.address)
            } else {
                format!(
                    "{}_{:08X}",
                    func.name.replace("::", "_").replace(" ", "_"),
                    func.address
                )
            };
            content.push_str(&format!(
                "        0x{:08X}u32 => {}(ctx, memory),\n",
                func.address, func_name
            ));
        }

        content.push_str("        _ => {\n");
        content
            .push_str("            log::warn!(\"Unknown function address: 0x{:08X}\", address);\n");
        content.push_str("            Ok(None)\n");
        content.push_str("        }\n");
        content.push_str("    }\n");
        content.push_str("}\n");

        Ok(content)
    }

    /// Decode all instructions from a DOL file.
    ///
    /// # Algorithm
    /// Iterates through all executable sections in the DOL file and decodes
    /// PowerPC instructions (4 bytes each, big-endian).
    ///
    /// # Arguments
    /// * `dol_file` - Parsed DOL file structure
    ///
    /// # Returns
    /// `Result<Vec<DecodedInstruction>>` - Vector of decoded instructions
    ///
    /// # Errors
    /// Returns error if DOL file is malformed or decoding fails
    ///
    /// # Memory Optimization
    /// Pre-allocates vector with estimated capacity based on section sizes.
    #[inline] // May be called frequently
    fn decode_all_instructions(dol_file: &DolFile) -> Result<Vec<DecodedInstruction>> {
        // Estimate total instruction count for pre-allocation
        let mut estimated_count: usize = 0usize;
        for section in dol_file.sections.iter() {
            if section.executable {
                estimated_count = estimated_count.wrapping_add(section.data.len() / 4usize);
            }
        }

        // Pre-allocate vector with estimated capacity
        let mut instructions: Vec<DecodedInstruction> = Vec::with_capacity(estimated_count);

        // Decode instructions from all sections with address tracking
        for section in dol_file.sections.iter() {
            if section.executable {
                let data: &[u8] = &section.data;
                let section_address: u32 = section.address;

                // Decode each 4-byte instruction chunk
                for (chunk_index, chunk) in data.chunks_exact(4usize).enumerate() {
                    let word: u32 = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                    // Calculate instruction address: section base + offset
                    let instruction_address: u32 =
                        section_address.wrapping_add((chunk_index * 4usize) as u32);

                    if let Ok(decoded) =
                        crate::recompiler::decoder::Instruction::decode(word, instruction_address)
                    {
                        instructions.push(decoded);
                    }
                }
            }
        }

        Ok(instructions)
    }

    /// Map instructions to a function based on address ranges.
    ///
    /// # Algorithm
    /// Filters instructions that fall within the function's address range:
    /// `func.address <= inst.address < func.address + func.size`
    ///
    /// # Edge Cases
    /// - Functions with overlapping addresses: uses first match (functions sorted by address)
    /// - Instructions outside any function: will be skipped (not included in any function)
    /// - Functions with zero size: minimum size of 4 bytes (one instruction)
    ///
    /// # Arguments
    /// * `func` - Function information from Ghidra analysis
    /// * `instructions` - All decoded instructions from the DOL file
    ///
    /// # Returns
    /// `Vec<DecodedInstruction>` - Instructions belonging to this function
    #[inline] // May be called frequently
    fn map_instructions_to_function(
        func: &crate::recompiler::ghidra::FunctionInfo,
        instructions: &[DecodedInstruction],
    ) -> Vec<DecodedInstruction> {
        // Ensure minimum function size (at least one instruction = 4 bytes)
        let func_size: u32 = if func.size == 0u32 { 4u32 } else { func.size };
        let func_end: u32 = func.address.wrapping_add(func_size);

        instructions
            .iter()
            .filter(|inst| {
                // Check if instruction address falls within function range
                func.address <= inst.address && inst.address < func_end
            })
            .cloned()
            .collect()
    }
}
