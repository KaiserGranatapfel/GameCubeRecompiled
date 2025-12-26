//! Integration tests for the recompilation pipeline

use gcrecomp_core::recompiler::pipeline::RecompilationPipeline;
use gcrecomp_core::recompiler::parser::DolFile;
use std::path::PathBuf;

#[test]
#[ignore] // Requires valid DOL file
fn test_full_pipeline() {
    // This test requires a valid DOL file
    // In a real scenario, you would use a test fixture
    let dol_path = PathBuf::from("tests/fixtures/test.dol");
    
    if !dol_path.exists() {
        // Skip test if fixture doesn't exist
        return;
    }
    
    let dol_file = DolFile::parse(&dol_path).expect("Failed to parse DOL file");
    let output_path = "tests/output/test_output.rs";
    
    // Clean up previous output
    let _ = std::fs::remove_file(output_path);
    
    let result = RecompilationPipeline::recompile(&dol_file, output_path);
    
    // Clean up
    let _ = std::fs::remove_file(output_path);
    
    assert!(result.is_ok(), "Pipeline should complete successfully");
}

#[test]
fn test_pipeline_with_invalid_file() {
    let dol_path = PathBuf::from("nonexistent.dol");
    let dol_file = DolFile::parse(&dol_path);
    
    assert!(dol_file.is_err(), "Should fail on nonexistent file");
}

