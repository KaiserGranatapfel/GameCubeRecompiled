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

use crate::recompiler::analysis::control_flow::ControlFlowAnalyzer;
use crate::recompiler::analysis::data_flow::DataFlowAnalyzer;
use crate::recompiler::codegen::CodeGenerator;
use crate::recompiler::decoder::DecodedInstruction;
use crate::recompiler::ghidra::GhidraAnalysis;
use crate::recompiler::parser::DolFile;
use crate::recompiler::validator::CodeValidator;
use anyhow::Result;

/// Recompilation pipeline orchestrator.
///
/// Coordinates all stages of the recompilation process from DOL file parsing
/// to Rust code generation.
pub struct RecompilationPipeline;

/// Mutable context that carries state through pipeline stages.
#[derive(Default)]
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
        Self::default()
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

        // Step 1: Decode instructions
        log::info!("Step 1: Decoding instructions...");
        let instructions: Vec<DecodedInstruction> = Self::decode_all_instructions(dol_file)?;

        // Step 2: Discover functions. Use Ghidra only if GHIDRA_INSTALL_DIR is set;
        // otherwise fall back to a naive scan of the decoded instructions so the
        // pipeline runs end-to-end with no external tool. ponytail: naive linear
        // sweep (split on `blr`), bounded; swap in Ghidra reachability for accuracy.
        let ghidra_analysis: GhidraAnalysis = if std::env::var("GHIDRA_INSTALL_DIR").is_ok() {
            log::info!("Step 2: Running Ghidra analysis (GHIDRA_INSTALL_DIR set)...");
            GhidraAnalysis::analyze(
                &dol_file.path,
                crate::recompiler::ghidra::GhidraBackend::HeadlessCli,
            )
            .unwrap_or_else(|e| {
                log::warn!("Ghidra analysis failed ({e}); falling back to naive discovery");
                Self::naive_function_discovery(dol_file.entry_point, &instructions)
            })
        } else {
            log::info!("Step 2: No GHIDRA_INSTALL_DIR; using naive function discovery...");
            Self::naive_function_discovery(dol_file.entry_point, &instructions)
        };

        // Step 2b: Enrich functions with derived facts and report coverage.
        let facts =
            crate::recompiler::enrich::enrich_functions(&ghidra_analysis.functions, &instructions);
        let report = crate::recompiler::enrich::CoverageReport::from_facts(&facts);
        log::info!(
            "Enrichment: {} functions ({} leaf, {} with loops); instruction coverage {:.1}% ({}/{} translated)",
            report.functions,
            report.leaf_functions,
            report.functions_with_loops,
            report.instruction_coverage() * 100.0,
            report.translated_instructions,
            report.total_instructions,
        );

        // Step 3: Control flow analysis
        log::info!("Step 3: Building control flow graph...");
        let cfg = ControlFlowAnalyzer::build_cfg(&instructions, 0u32)?;

        // Step 4: Data flow analysis
        log::info!("Step 4: Performing data flow analysis...");
        let _def_use_chains = DataFlowAnalyzer::build_def_use_chains(&instructions);
        let _live_analysis = DataFlowAnalyzer::live_variable_analysis(&cfg);

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
        rust_code.push_str("//! Generated by GCRecomp\n");
        rust_code
            .push_str("#![allow(warnings, clippy::all, clippy::pedantic, clippy::nursery)]\n\n");
        rust_code.push_str("use gcrecomp_core::runtime::context::CpuContext;\n");
        rust_code.push_str("use gcrecomp_core::runtime::memory::MemoryManager;\n");
        rust_code.push_str("use anyhow::Result;\n\n");
        rust_code.push_str(&format!(
            "/// Original DOL entry-point address (call via `call_function_by_address`).\npub const ENTRY_POINT: u32 = 0x{:08X};\n\n",
            dol_file.entry_point
        ));

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

            // Generate function code
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
                        register: None,
                        stack_offset: p.offset.unwrap_or(0),
                    })
                    .collect(),
                return_type: None,
                local_variables: func
                    .local_variables
                    .iter()
                    .map(|v| crate::recompiler::analysis::VariableInfo {
                        name: v.name.clone(),
                        type_info: crate::recompiler::analysis::TypeInfo::Unknown,
                        stack_offset: v.offset,
                        scope_start: 0u32,
                        scope_end: 0u32,
                    })
                    .collect(),
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

        // Add function dispatcher at the end
        rust_code.push_str("\n/// Function dispatcher - calls recompiled functions by address\n");
        rust_code
            .push_str("/// This is generated automatically to handle indirect function calls\n");
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

        // Memory image loader: the DOL's text+data sections are serialized to a
        // sidecar `game_image.bin` and embedded, so the game can load real data
        // into RAM before running recompiled code (otherwise every read is 0).
        rust_code.push_str("/// Embedded initial memory image (DOL sections).\n");
        rust_code
            .push_str("pub static GAME_IMAGE: &[u8] = include_bytes!(\"game_image.bin\");\n\n");
        rust_code.push_str("/// Load the DOL's sections into RAM at their virtual addresses.\n");
        rust_code.push_str("pub fn load_image(memory: &mut MemoryManager) {\n");
        rust_code.push_str("    let img = GAME_IMAGE;\n");
        rust_code.push_str("    let mut p = 0usize;\n");
        rust_code.push_str("    while p + 8 <= img.len() {\n");
        rust_code.push_str(
            "        let addr = u32::from_le_bytes([img[p], img[p + 1], img[p + 2], img[p + 3]]);\n",
        );
        rust_code.push_str("        let len = u32::from_le_bytes([img[p + 4], img[p + 5], img[p + 6], img[p + 7]]) as usize;\n");
        rust_code.push_str("        p += 8;\n");
        rust_code.push_str("        if p + len > img.len() { break; }\n");
        rust_code.push_str("        let _ = memory.load_section(addr, &img[p..p + len]);\n");
        rust_code.push_str("        p += len;\n");
        rust_code.push_str("    }\n");
        rust_code.push_str("}\n");

        // Step 7: Validation
        log::info!("Step 7: Validating generated code...");
        CodeValidator::validate_rust_code(&rust_code)?;

        // Step 8: Write output + the embedded memory image next to it.
        log::info!("Step 8: Writing output to {}...", output_path);
        std::fs::write(output_path, rust_code)?;

        let mut image: Vec<u8> = Vec::new();
        for sec in dol_file
            .text_sections
            .iter()
            .chain(dol_file.data_sections.iter())
        {
            if sec.data.is_empty() {
                continue;
            }
            image.extend_from_slice(&sec.address.to_le_bytes());
            image.extend_from_slice(&(sec.data.len() as u32).to_le_bytes());
            image.extend_from_slice(&sec.data);
        }
        let img_path = std::path::Path::new(output_path).with_file_name("game_image.bin");
        std::fs::write(&img_path, &image)?;
        log::info!(
            "Wrote memory image: {} ({} bytes)",
            img_path.display(),
            image.len()
        );

        log::info!("Recompilation complete!");
        Ok(())
    }

    /// Analyze a DOL without generating code or needing any external tool:
    /// decode, discover functions (naive sweep), and enrich them. Returns the
    /// per-function facts and a whole-program coverage rollup. Used by the
    /// `analyze` CLI command.
    pub fn analyze(
        dol_file: &DolFile,
    ) -> Result<(
        Vec<crate::recompiler::enrich::FunctionFacts>,
        crate::recompiler::enrich::CoverageReport,
    )> {
        let instructions = Self::decode_all_instructions(dol_file)?;
        let analysis = Self::naive_function_discovery(dol_file.entry_point, &instructions);
        let facts = crate::recompiler::enrich::enrich_functions(&analysis.functions, &instructions);
        let report = crate::recompiler::enrich::CoverageReport::from_facts(&facts);
        Ok((facts, report))
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
        let dol = ctx
            .dol_file
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No DOL file loaded"))?;
        let analysis =
            GhidraAnalysis::analyze(&dol.path, crate::recompiler::ghidra::GhidraBackend::ReOxide)?;
        ctx.ghidra_analysis = Some(analysis);
        Ok(())
    }

    /// Stage: Decode PowerPC instructions from the DOL.
    pub fn stage_decode(ctx: &mut PipelineContext) -> Result<()> {
        log::info!("Stage: Decoding instructions...");
        let dol = ctx
            .dol_file
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No DOL file loaded"))?;
        let instructions = Self::decode_all_instructions(dol)?;
        ctx.stats.total_instructions = instructions.len();
        ctx.instructions = Some(instructions);
        Ok(())
    }

    /// Stage: Build control flow graph.
    pub fn stage_build_cfg(ctx: &mut PipelineContext) -> Result<()> {
        log::info!("Stage: Building control flow graph...");
        let instructions = ctx
            .instructions
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No instructions decoded"))?;
        let cfg = ControlFlowAnalyzer::build_cfg(instructions, 0u32)?;
        ctx.cfg = Some(cfg);
        Ok(())
    }

    /// Stage: Perform data flow analysis.
    pub fn stage_analyze_data_flow(ctx: &mut PipelineContext) -> Result<()> {
        log::info!("Stage: Performing data flow analysis...");
        let instructions = ctx
            .instructions
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No instructions decoded"))?;
        let cfg = ctx
            .cfg
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No CFG built"))?;
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
        let ghidra_analysis = ctx
            .ghidra_analysis
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No Ghidra analysis"))?;
        let instructions = ctx
            .instructions
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No instructions decoded"))?;
        let mut codegen = CodeGenerator::new();

        let estimated_capacity = ghidra_analysis.functions.len() * 1000;
        let mut rust_code = String::with_capacity(estimated_capacity);

        rust_code.push_str("//! Recompiled GameCube game functions\n");
        rust_code.push_str("//! Generated by GCRecomp\n\n");
        rust_code.push_str("use gcrecomp_core::runtime::context::CpuContext;\n");
        rust_code.push_str("use gcrecomp_core::runtime::memory::MemoryManager;\n");
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
                parameters: func
                    .parameters
                    .iter()
                    .map(|p| crate::recompiler::analysis::ParameterInfo {
                        name: p.name.clone(),
                        type_info: crate::recompiler::analysis::TypeInfo::Unknown,
                        register: None,
                        stack_offset: p.offset.unwrap_or(0),
                    })
                    .collect(),
                return_type: None,
                local_variables: func
                    .local_variables
                    .iter()
                    .map(|v| crate::recompiler::analysis::VariableInfo {
                        name: v.name.clone(),
                        type_info: crate::recompiler::analysis::TypeInfo::Unknown,
                        stack_offset: v.offset,
                        scope_start: 0,
                        scope_end: 0,
                    })
                    .collect(),
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
        let code = ctx
            .rust_code
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No code generated"))?;
        CodeValidator::validate_rust_code(code)?;
        Ok(())
    }

    /// Stage: Write output to file.
    pub fn stage_write_output(ctx: &mut PipelineContext, output_path: &str) -> Result<()> {
        log::info!("Stage: Writing output to {}...", output_path);
        let code = ctx
            .rust_code
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No code generated"))?;
        std::fs::write(output_path, code)?;
        Ok(())
    }

    /// Stage: Generate `game/src/assets.rs` to embed the GCFS archive.
    ///
    /// If `game/assets.bin` exists, generates an `include_bytes!` reference.
    /// Otherwise, generates an empty stub so the game crate always compiles.
    pub fn stage_embed_assets() -> Result<()> {
        let assets_bin = std::path::Path::new("game/assets.bin");
        let assets_rs = std::path::Path::new("game/src/assets.rs");

        let content = if assets_bin.exists() {
            let meta = std::fs::metadata(assets_bin)?;
            log::info!(
                "Embedding assets archive ({} bytes) via include_bytes!",
                meta.len()
            );
            "/// Compressed GCFS archive of disc filesystem assets.\n\
             pub static ARCHIVE: &[u8] = include_bytes!(\"../assets.bin\");\n"
                .to_string()
        } else {
            log::info!("No assets.bin found; generating empty asset stub.");
            "/// No disc assets available (DOL-only upload).\n\
             pub static ARCHIVE: &[u8] = &[];\n"
                .to_string()
        };

        std::fs::write(assets_rs, content)?;
        log::info!("Generated {}", assets_rs.display());
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
                let instruction_address: u32 =
                    section_address.wrapping_add((chunk_index * 4usize) as u32);

                if let Ok(decoded) =
                    crate::recompiler::decoder::Instruction::decode(word, instruction_address)
                {
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

    /// Discover functions without Ghidra: a linear sweep over decoded instructions
    /// starting at the DOL entry point, splitting a new function after every `blr`
    /// (return) instruction. Bounded by `GCRECOMP_MAX_FUNCS` (default 64) and
    /// `GCRECOMP_MAX_INSTRS` (default 8192) so a full game (~800k instrs) doesn't
    /// generate Rust that never finishes compiling.
    ///
    /// ponytail: naive heuristic; a real recompiler would use Ghidra/CFG reachability
    /// to find true function boundaries. Raise the env caps for a fuller recompile.
    fn naive_function_discovery(entry: u32, instructions: &[DecodedInstruction]) -> GhidraAnalysis {
        use crate::recompiler::ghidra::FunctionInfo;
        const BLR: u32 = 0x4E80_0020; // canonical `blr` (branch to link register)

        let env_usize = |key: &str, default: usize| {
            std::env::var(key)
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(default)
        };
        // Default to the whole game; the full DOL (~14k functions / ~840k
        // instructions) compiles in ~90s. Lower these env vars to bound a quick run.
        let max_funcs = env_usize("GCRECOMP_MAX_FUNCS", 2_000_000);
        let max_instrs = env_usize("GCRECOMP_MAX_INSTRS", 20_000_000);

        let mk = |start: u32, size: u32| FunctionInfo {
            address: start,
            name: format!("sub_{:08x}", start),
            size,
            calling_convention: "default".to_string(),
            parameters: vec![],
            return_type: None,
            local_variables: vec![],
            basic_blocks: vec![],
        };

        let mut functions: Vec<FunctionInfo> = Vec::new();
        let mut cur_start: Option<u32> = None;
        let mut cur_bytes: u32 = 0;
        let mut seen = 0usize;

        for inst in instructions.iter().filter(|i| i.address >= entry) {
            if functions.len() >= max_funcs || seen >= max_instrs {
                break;
            }
            if cur_start.is_none() {
                cur_start = Some(inst.address);
                cur_bytes = 0;
            }
            cur_bytes += 4;
            seen += 1;
            if inst.raw == BLR {
                functions.push(mk(cur_start.take().unwrap(), cur_bytes));
            }
        }
        // Close any trailing open function (no terminating blr within the budget).
        if let Some(start) = cur_start {
            if cur_bytes > 0 {
                functions.push(mk(start, cur_bytes));
            }
        }

        log::info!(
            "Naive discovery: {} functions from entry 0x{:08X} ({} instructions scanned)",
            functions.len(),
            entry,
            seen
        );

        GhidraAnalysis {
            functions,
            symbols: vec![],
            decompiled_code: std::collections::HashMap::new(),
            instructions: std::collections::HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recompiler::decoder::Instruction;

    // Decode real instruction words so the test exercises the same `raw`/`address`
    // fields naive discovery keys on. 0x4E800020 = blr; 0x38000000 = a non-return.
    fn instrs(words: &[u32], base: u32) -> Vec<DecodedInstruction> {
        words
            .iter()
            .enumerate()
            .map(|(i, &w)| Instruction::decode(w, base + (i as u32) * 4).unwrap())
            .collect()
    }

    #[test]
    fn naive_discovery_splits_on_blr_from_entry() {
        const NOP: u32 = 0x3800_0000;
        const BLR: u32 = 0x4E80_0020;
        // 0x100..: [pre-entry junk] | f1: nop,nop,blr | f2: nop,blr
        let words = [NOP, NOP, /* entry@0x108 */ NOP, NOP, BLR, NOP, BLR];
        let is = instrs(&words, 0x100);
        let entry = 0x108;

        let a = RecompilationPipeline::naive_function_discovery(entry, &is);

        assert_eq!(a.functions.len(), 2, "two blr-delimited functions");
        assert_eq!(
            a.functions[0].address, 0x108,
            "first function starts at entry"
        );
        assert_eq!(a.functions[0].size, 12, "f1 = 3 instrs (nop,nop,blr)");
        assert_eq!(a.functions[1].address, 0x114);
        assert_eq!(a.functions[1].size, 8, "f2 = 2 instrs (nop,blr)");
    }

    #[test]
    fn naive_discovery_closes_trailing_function_without_blr() {
        const NOP: u32 = 0x3800_0000;
        let is = instrs(&[NOP, NOP, NOP], 0x200);
        let a = RecompilationPipeline::naive_function_discovery(0x200, &is);
        assert_eq!(a.functions.len(), 1);
        assert_eq!(
            a.functions[0].size, 12,
            "no blr -> one function spanning all 3"
        );
    }
}
