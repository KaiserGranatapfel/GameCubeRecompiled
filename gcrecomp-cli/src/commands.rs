// CLI command handlers
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use gcrecomp_core::recompiler::{
    parser::DolFile,
    ghidra::{GhidraAnalysis, GhidraBackend},
    codegen::CodeGenerator,
};
use std::fs;

pub fn analyze_dol(dol_file: &Path, use_reoxide: bool) -> Result<()> {
    println!("Reading DOL file: {}", dol_file.display());
    
    let data = fs::read(dol_file)
        .with_context(|| format!("Failed to read DOL file: {}", dol_file.display()))?;
    
    let dol = DolFile::parse(&data)
        .context("Failed to parse DOL file")?;
    
    println!("DOL file parsed successfully");
    println!("  Text sections: {}", dol.text_sections.len());
    println!("  Data sections: {}", dol.data_sections.len());
    println!("  Entry point: 0x{:08X}", dol.entry_point);
    println!("  BSS address: 0x{:08X}, size: 0x{:08X}", dol.bss_address, dol.bss_size);
    
    println!("\nRunning Ghidra analysis...");
    let backend = if use_reoxide {
        GhidraBackend::ReOxide
    } else {
        GhidraBackend::HeadlessCli
    };
    
    let analysis = GhidraAnalysis::analyze(
        dol_file.to_str().context("Invalid DOL file path")?,
        backend,
    )?;
    
    println!("Analysis complete");
    println!("  Functions found: {}", analysis.functions.len());
    println!("  Symbols found: {}", analysis.symbols.len());
    
    for func in &analysis.functions {
        println!("    Function: {} @ 0x{:08X} (size: {})", 
                 func.name, func.address, func.size);
    }
    
    Ok(())
}

pub fn recompile_dol(
    dol_file: &Path,
    output_dir: Option<&Path>,
    use_reoxide: bool,
    linker_script: Option<&Path>,
    fidb_path: Option<&Path>,
    enable_bsim: bool,
    hierarchical: bool,
) -> Result<()> {
    println!("Recompiling DOL file: {}", dol_file.display());
    
    // Parse DOL file
    let data = fs::read(dol_file)
        .with_context(|| format!("Failed to read DOL file: {}", dol_file.display()))?;
    
    let dol = DolFile::parse(&data)
        .context("Failed to parse DOL file")?;
    
    // Determine output directory
    let output_dir = output_dir
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("game/src/recompiled"));
    
    fs::create_dir_all(&output_dir)
        .context("Failed to create output directory")?;
    
    if hierarchical {
        // Use hierarchical recompilation with linker script support
        println!("Using hierarchical recompilation with enhanced Ghidra analysis...");
        gcrecomp_core::recompiler::pipeline::RecompilationPipeline::recompile_hierarchical(
            &dol,
            &output_dir,
            linker_script,
            fidb_path,
            enable_bsim,
        )?;
        println!("Hierarchical recompilation complete!");
        println!("Output directory: {}", output_dir.display());
    } else {
        // Use legacy flat recompilation
        println!("Using flat recompilation...");
        let output_file = output_dir.join("recompiled.rs");
        gcrecomp_core::recompiler::pipeline::RecompilationPipeline::recompile(
            &dol,
            output_file.to_str().context("Invalid output path")?,
        )?;
        println!("Generated Rust code written to: {}", output_file.display());
    }
    
    Ok(())
}

pub fn build_dol(
    dol_file: &Path,
    output_dir: Option<&Path>,
    use_reoxide: bool,
    linker_script: Option<&Path>,
    fidb_path: Option<&Path>,
    enable_bsim: bool,
    hierarchical: bool,
) -> Result<()> {
    println!("Building recompiled game from: {}", dol_file.display());
    
    // Step 1: Analyze
    println!("Step 1/3: Analyzing DOL file...");
    analyze_dol(dol_file, use_reoxide)?;
    
    // Step 2: Recompile
    println!("\nStep 2/3: Recompiling to Rust...");
    recompile_dol(
        dol_file,
        output_dir,
        use_reoxide,
        linker_script,
        fidb_path,
        enable_bsim,
        hierarchical,
    )?;
    
    // Step 3: Build
    println!("\nStep 3/3: Building Rust project...");
    let output = std::process::Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("--manifest-path")
        .arg("game/Cargo.toml")
        .output()
        .context("Failed to run cargo build")?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Cargo build failed: {}", stderr);
    }
    
    println!("Build complete! Executable should be in game/target/release/");
    
    Ok(())
}

