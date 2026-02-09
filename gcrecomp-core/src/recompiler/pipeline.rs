//! Complete Recompilation Pipeline
//!
//! This module orchestrates the complete recompilation process from DOL file to Rust code.
//! It coordinates all analysis and code generation stages in the correct order.
//!
//! # Pipeline Stages
//! 1. **Ghidra Analysis**: Extract function metadata, symbols, and type information
//! 2. **Instruction Decoding**: Decode PowerPC instructions from binary
//! 3. **Control Flow Analysis**: Build control flow graph (CFG)
//! 4. **Data Flow Analysis**: Build def-use chains and perform live variable analysis
//! 5. **Type Inference**: Recover type information for registers and variables
//! 6. **Code Generation**: Generate Rust code from analyzed instructions
//! 7. **Validation**: Validate generated Rust code
//! 8. **Output**: Write generated code to file
//!
//! # Memory Optimizations
//! - Pre-allocate vectors with known capacity where possible
//! - Use `SmallVec` for temporary instruction lists
//! - Avoid unnecessary clones (use references where possible)
//! - Reuse buffers for string concatenation

use crate::recompiler::parser::DolFile;
use crate::recompiler::decoder::DecodedInstruction;
use crate::recompiler::ghidra::GhidraAnalysis;
use crate::recompiler::analysis::control_flow::ControlFlowAnalyzer;
use crate::recompiler::analysis::data_flow::DataFlowAnalyzer;
use crate::recompiler::analysis::type_inference::TypeInferenceEngine;
use crate::recompiler::codegen::CodeGenerator;
use crate::recompiler::validator::CodeValidator;
use anyhow::Result;
use smallvec::SmallVec;

/// Recompilation pipeline orchestrator.
///
/// Coordinates all stages of the recompilation process from DOL file parsing
/// to Rust code generation.
pub struct RecompilationPipeline;

/// Mutable context that carries state through pipeline stages.
pub struct PipelineContext {
    pub dol_file: Option<DolFile>,
    pub ghidra_analysis: Option<GhidraAnalysis>,
    pub instructions: Option<Vec<DecodedInstruction>>,
    pub cfg: Option<crate::recompiler::analysis::control_flow::ControlFlowGraph>,
    pub rust_code: Option<String>,
    pub stats: PipelineStats,
}

/// Statistics collected during pipeline execution.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PipelineStats {
    pub total_functions: usize,
    pub successful_functions: usize,
    pub failed_functions: usize,
    pub total_instructions: usize,
}

impl PipelineContext {
    pub fn new() -> Self {
        Self {
            dol_file: None,
            ghidra_analysis: None,
            instructions: None,
            cfg: None,
            rust_code: None,
            stats: PipelineStats::default(),
        }
    }
}

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
        
        // Step 4: Data flow analysis
        log::info!("Step 4: Performing data flow analysis...");
        let def_use_chains = DataFlowAnalyzer::build_def_use_chains(&instructions);
        let live_analysis = DataFlowAnalyzer::live_variable_analysis(&cfg);
        
        // Step 5: Type inference
        log::info!("Step 5: Inferring types...");
        // Would use function metadata from Ghidra
        
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
        
        let total_functions: usize = ghidra_analysis.functions.len();
        let mut successful_functions: usize = 0usize;
        let mut failed_functions: usize = 0usize;
        
        for (idx, func) in ghidra_analysis.functions.iter().enumerate() {
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
            let func_instructions: Vec<DecodedInstruction> = Self::map_instructions_to_function(func, &instructions);
            
            if func_instructions.is_empty() {
                log::warn!(
                    "Function {} at 0x{:08X} has no instructions, skipping",
                    func.name,
                    func.address
                );
                failed_functions += 1;
                continue;
            }
            
            // Generate function code
            let func_metadata = crate::recompiler::analysis::FunctionMetadata {
                address: func.address,
                name: func.name.clone(),
                size: func.size,
                calling_convention: func.calling_convention.clone(),
                parameters: func.parameters.iter().map(|p| {
                    crate::recompiler::analysis::ParameterInfo {
                        name: p.name.clone(),
                        type_info: crate::recompiler::analysis::TypeInfo::Unknown,
                        register: None,
                        stack_offset: p.offset.unwrap_or(0),
                    }
                }).collect(),
                return_type: None,
                local_variables: func.local_variables.iter().map(|v| {
                    crate::recompiler::analysis::VariableInfo {
                        name: v.name.clone(),
                        type_info: crate::recompiler::analysis::TypeInfo::Unknown,
                        stack_offset: v.offset,
                        scope_start: 0u32,
                        scope_end: 0u32,
                    }
                }).collect(),
                basic_blocks: vec![],
            };
            
            match codegen.generate_function(&func_metadata, &func_instructions) {
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
                        func.name,
                        func.address,
                        e
                    ));
                    rust_code.push_str(&format!(
                        "pub fn {}_0x{:08X}(_ctx: &mut CpuContext, _memory: &mut MemoryManager) -> Result<Option<u32>> {{\n",
                        codegen.sanitize_identifier(&func.name),
                        func.address
                    ));
                    rust_code.push_str("    log::warn!(\"Function stub called - not implemented\");\n");
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
        
        // Add function dispatcher at the end
        rust_code.push_str("\n/// Function dispatcher - calls recompiled functions by address\n");
        rust_code.push_str("/// This is generated automatically to handle indirect function calls\n");
        rust_code.push_str("pub fn call_function_by_address(\n");
        rust_code.push_str("    address: u32,\n");
        rust_code.push_str("    ctx: &mut CpuContext,\n");
        rust_code.push_str("    memory: &mut MemoryManager,\n");
        rust_code.push_str(") -> Result<Option<u32>> {\n");
        rust_code.push_str("    // Static function address mapping\n");
        rust_code.push_str("    match address {\n");
        
        // Add function address mappings
        for func in ghidra_analysis.functions.iter() {
            let func_name = if func.name.is_empty() || func.name.starts_with("sub_") {
                format!("func_0x{:08X}", func.address)
            } else {
                format!("{}_{:08X}", codegen.sanitize_identifier(&func.name), func.address)
            };
            rust_code.push_str(&format!(
                "        0x{:08X}u32 => {}(ctx, memory),\n",
                func.address,
                func_name
            ));
        }
        
        rust_code.push_str("        _ => {\n");
        rust_code.push_str("            log::warn!(\"Unknown function address: 0x{:08X}\", address);\n");
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
    
    // --- Discrete stage methods for Lua orchestration ---

    /// Stage: Load a DOL file into the pipeline context.
    pub fn stage_load_dol(ctx: &mut PipelineContext, path: &str) -> Result<()> {
        log::info!("Stage: Loading DOL file: {}", path);
        let data = std::fs::read(path)?;
        let dol = crate::recompiler::parser::DolFile::parse(&data, path)?;
        ctx.dol_file = Some(dol);
        Ok(())
    }

    /// Stage: Run Ghidra analysis on the loaded DOL.
    pub fn stage_analyze(ctx: &mut PipelineContext) -> Result<()> {
        log::info!("Stage: Running Ghidra analysis...");
        let dol = ctx.dol_file.as_ref().ok_or_else(|| anyhow::anyhow!("No DOL file loaded"))?;
        let analysis = GhidraAnalysis::analyze(
            &dol.path,
            crate::recompiler::ghidra::GhidraBackend::ReOxide,
        )?;
        ctx.ghidra_analysis = Some(analysis);
        Ok(())
    }

    /// Stage: Decode PowerPC instructions from the DOL.
    pub fn stage_decode(ctx: &mut PipelineContext) -> Result<()> {
        log::info!("Stage: Decoding instructions...");
        let dol = ctx.dol_file.as_ref().ok_or_else(|| anyhow::anyhow!("No DOL file loaded"))?;
        let instructions = Self::decode_all_instructions(dol)?;
        ctx.stats.total_instructions = instructions.len();
        ctx.instructions = Some(instructions);
        Ok(())
    }

    /// Stage: Build control flow graph.
    pub fn stage_build_cfg(ctx: &mut PipelineContext) -> Result<()> {
        log::info!("Stage: Building control flow graph...");
        let instructions = ctx.instructions.as_ref().ok_or_else(|| anyhow::anyhow!("No instructions decoded"))?;
        let cfg = ControlFlowAnalyzer::build_cfg(instructions, 0u32)?;
        ctx.cfg = Some(cfg);
        Ok(())
    }

    /// Stage: Perform data flow analysis.
    pub fn stage_analyze_data_flow(ctx: &mut PipelineContext) -> Result<()> {
        log::info!("Stage: Performing data flow analysis...");
        let instructions = ctx.instructions.as_ref().ok_or_else(|| anyhow::anyhow!("No instructions decoded"))?;
        let cfg = ctx.cfg.as_ref().ok_or_else(|| anyhow::anyhow!("No CFG built"))?;
        let _def_use_chains = DataFlowAnalyzer::build_def_use_chains(instructions);
        let _live_analysis = DataFlowAnalyzer::live_variable_analysis(cfg);
        Ok(())
    }

    /// Stage: Infer types (placeholder).
    pub fn stage_infer_types(_ctx: &mut PipelineContext) -> Result<()> {
        log::info!("Stage: Inferring types...");
        Ok(())
    }

    /// Stage: Generate Rust code from analyzed instructions.
    pub fn stage_generate_code(ctx: &mut PipelineContext) -> Result<()> {
        log::info!("Stage: Generating code...");
        let ghidra_analysis = ctx.ghidra_analysis.as_ref().ok_or_else(|| anyhow::anyhow!("No Ghidra analysis"))?;
        let instructions = ctx.instructions.as_ref().ok_or_else(|| anyhow::anyhow!("No instructions decoded"))?;
        let mut codegen = CodeGenerator::new();

        let estimated_capacity = ghidra_analysis.functions.len() * 1000;
        let mut rust_code = String::with_capacity(estimated_capacity);

        rust_code.push_str("//! Recompiled GameCube game functions\n");
        rust_code.push_str("//! Generated by GCRecomp\n\n");
        rust_code.push_str("use crate::runtime::context::CpuContext;\n");
        rust_code.push_str("use crate::runtime::memory::MemoryManager;\n");
        rust_code.push_str("use anyhow::Result;\n\n");

        let total_functions = ghidra_analysis.functions.len();
        let mut successful = 0usize;
        let mut failed = 0usize;

        for func in ghidra_analysis.functions.iter() {
            let func_instructions = Self::map_instructions_to_function(func, instructions);

            if func_instructions.is_empty() {
                failed += 1;
                continue;
            }

            let func_metadata = crate::recompiler::analysis::FunctionMetadata {
                address: func.address,
                name: func.name.clone(),
                size: func.size,
                calling_convention: func.calling_convention.clone(),
                parameters: func.parameters.iter().map(|p| {
                    crate::recompiler::analysis::ParameterInfo {
                        name: p.name.clone(),
                        type_info: crate::recompiler::analysis::TypeInfo::Unknown,
                        register: None,
                        stack_offset: p.offset.unwrap_or(0),
                    }
                }).collect(),
                return_type: None,
                local_variables: func.local_variables.iter().map(|v| {
                    crate::recompiler::analysis::VariableInfo {
                        name: v.name.clone(),
                        type_info: crate::recompiler::analysis::TypeInfo::Unknown,
                        stack_offset: v.offset,
                        scope_start: 0,
                        scope_end: 0,
                    }
                }).collect(),
                basic_blocks: vec![],
            };

            match codegen.generate_function(&func_metadata, &func_instructions) {
                Ok(func_code) => {
                    rust_code.push_str(&func_code);
                    rust_code.push('\n');
                    successful += 1;
                }
                Err(e) => {
                    failed += 1;
                    rust_code.push_str(&format!(
                        "// Stub for {} at 0x{:08X} (generation failed: {})\n",
                        func.name, func.address, e
                    ));
                    rust_code.push_str(&format!(
                        "pub fn {}_0x{:08X}(_ctx: &mut CpuContext, _memory: &mut MemoryManager) -> Result<Option<u32>> {{\n",
                        codegen.sanitize_identifier(&func.name), func.address
                    ));
                    rust_code.push_str("    Ok(None)\n}\n\n");
                }
            }
        }

        // Function dispatcher
        rust_code.push_str("\npub fn call_function_by_address(\n    address: u32,\n    ctx: &mut CpuContext,\n    memory: &mut MemoryManager,\n) -> Result<Option<u32>> {\n    match address {\n");
        for func in ghidra_analysis.functions.iter() {
            let func_name = if func.name.is_empty() || func.name.starts_with("sub_") {
                format!("func_0x{:08X}", func.address)
            } else {
                format!("{}_{:08X}", codegen.sanitize_identifier(&func.name), func.address)
            };
            rust_code.push_str(&format!("        0x{:08X}u32 => {}(ctx, memory),\n", func.address, func_name));
        }
        rust_code.push_str("        _ => Ok(None),\n    }\n}\n");

        ctx.stats.total_functions = total_functions;
        ctx.stats.successful_functions = successful;
        ctx.stats.failed_functions = failed;
        ctx.rust_code = Some(rust_code);
        Ok(())
    }

    /// Stage: Validate generated code.
    pub fn stage_validate(ctx: &mut PipelineContext) -> Result<()> {
        log::info!("Stage: Validating generated code...");
        let code = ctx.rust_code.as_ref().ok_or_else(|| anyhow::anyhow!("No code generated"))?;
        CodeValidator::validate_rust_code(code)?;
        Ok(())
    }

    /// Stage: Write output to file.
    pub fn stage_write_output(ctx: &mut PipelineContext, output_path: &str) -> Result<()> {
        log::info!("Stage: Writing output to {}...", output_path);
        let code = ctx.rust_code.as_ref().ok_or_else(|| anyhow::anyhow!("No code generated"))?;
        std::fs::write(output_path, code)?;
        Ok(())
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
        // Only text sections are executable
        let mut estimated_count: usize = 0usize;
        for section in dol_file.text_sections.iter() {
            estimated_count = estimated_count.wrapping_add(section.data.len() / 4usize);
        }

        // Pre-allocate vector with estimated capacity
        let mut instructions: Vec<DecodedInstruction> = Vec::with_capacity(estimated_count);

        // Decode instructions from text sections (executable sections)
        for section in dol_file.text_sections.iter() {
            let data: &[u8] = &section.data;
            let section_address: u32 = section.address;

            // Decode each 4-byte instruction chunk
            for (chunk_index, chunk) in data.chunks_exact(4usize).enumerate() {
                let word: u32 = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                // Calculate instruction address: section base + offset
                let instruction_address: u32 = section_address.wrapping_add((chunk_index * 4usize) as u32);

                if let Ok(decoded) = crate::recompiler::decoder::Instruction::decode(word, instruction_address) {
                    instructions.push(decoded);
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
