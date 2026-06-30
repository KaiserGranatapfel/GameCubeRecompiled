// CLI command handlers
use anyhow::{Context, Result};
use gcrecomp_core::recompiler::{parser::DolFile, pipeline::RecompilationPipeline};
use std::fs;
use std::path::{Path, PathBuf};

pub fn analyze_dol(dol_file: &Path, _use_reoxide: bool) -> Result<()> {
    println!("Reading DOL file: {}", dol_file.display());

    let data = fs::read(dol_file)
        .with_context(|| format!("Failed to read DOL file: {}", dol_file.display()))?;
    let dol = DolFile::parse(&data, dol_file.to_str().unwrap_or("unknown.dol"))
        .context("Failed to parse DOL file")?;

    println!("DOL file parsed successfully");
    println!("  Text sections: {}", dol.text_sections.len());
    println!("  Data sections: {}", dol.data_sections.len());
    println!("  Entry point: 0x{:08X}", dol.entry_point);
    println!(
        "  BSS address: 0x{:08X}, size: 0x{:08X}",
        dol.bss_address, dol.bss_size
    );

    // Decode + discover + enrich (no Ghidra / external tool required).
    let (facts, report) = RecompilationPipeline::analyze(&dol).context("Analysis failed")?;

    println!("\nAnalysis complete (naive discovery + enrichment):");
    println!("  Functions:            {}", report.functions);
    println!("  Leaf functions:       {}", report.leaf_functions);
    println!("  Functions with loops: {}", report.functions_with_loops);
    println!("  Instructions:         {}", report.total_instructions);
    println!(
        "  Instruction coverage: {:.1}% ({}/{} translated)",
        report.instruction_coverage() * 100.0,
        report.translated_instructions,
        report.total_instructions
    );

    // Show the 10 largest functions as a sample.
    let mut by_size: Vec<_> = facts.iter().collect();
    by_size.sort_by_key(|f| std::cmp::Reverse(f.instruction_count));
    println!("\n  Largest functions:");
    for f in by_size.iter().take(10) {
        println!(
            "    {} @ 0x{:08X}  {} instrs, {} calls{}{}",
            f.name,
            f.address,
            f.instruction_count,
            f.call_targets.len(),
            if f.is_leaf { ", leaf" } else { "" },
            if f.has_loop { ", loop" } else { "" },
        );
    }

    Ok(())
}

pub fn recompile_dol(dol_file: &Path, output_dir: Option<&Path>, _use_reoxide: bool) -> Result<()> {
    println!("Recompiling DOL file: {}", dol_file.display());

    let data = fs::read(dol_file)
        .with_context(|| format!("Failed to read DOL file: {}", dol_file.display()))?;
    let dol = DolFile::parse(&data, dol_file.to_str().unwrap_or("unknown.dol"))
        .context("Failed to parse DOL file")?;

    // Output: the `recompiled` library crate's lib.rs by default (so the whole
    // game becomes a compilable crate the `game` binary links). With --output-dir,
    // write <dir>/recompiled.rs instead.
    let output_file = match output_dir {
        Some(dir) => {
            fs::create_dir_all(dir).context("Failed to create output directory")?;
            dir.join("recompiled.rs")
        }
        None => PathBuf::from("recompiled/src/lib.rs"),
    };

    // Run the real decode -> analyze -> codegen pipeline (no Ghidra required).
    RecompilationPipeline::recompile(&dol, output_file.to_str().context("Invalid output path")?)
        .context("Recompilation pipeline failed")?;

    println!("Generated Rust code written to: {}", output_file.display());

    Ok(())
}

pub fn build_dol(dol_file: &Path, output_dir: Option<&Path>, use_reoxide: bool) -> Result<()> {
    println!("Building recompiled game from: {}", dol_file.display());

    // Step 1: Recompile DOL -> Rust (decode + codegen, no Ghidra required).
    println!("Step 1/2: Recompiling to Rust...");
    recompile_dol(dol_file, output_dir, use_reoxide)?;

    // Step 2: Build the `game` crate into a native executable.
    println!("\nStep 2/2: Building the game crate...");
    let status = std::process::Command::new("cargo")
        .args(["build", "-p", "game"])
        .status()
        .context("Failed to run cargo build")?;

    if !status.success() {
        anyhow::bail!("Cargo build of `game` failed");
    }

    println!("Build complete! Executable: target/debug/game");

    Ok(())
}
