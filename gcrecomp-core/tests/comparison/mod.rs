//! Binary Comparison Tests
//!
//! This module provides utilities for comparing original and recompiled code execution.

use crate::tests::utils::*;
use gcrecomp_core::runtime::context::CpuContext;
use gcrecomp_core::runtime::memory::MemoryManager;

/// Result of a binary comparison test.
#[derive(Debug, Clone)]
pub struct ComparisonResult {
    /// Whether the comparison passed
    pub passed: bool,
    /// Differences found
    pub differences: Vec<String>,
}

/// Compare register states between original and recompiled execution.
pub fn compare_register_state(
    original: &CpuContext,
    recompiled: &CpuContext,
) -> ComparisonResult {
    let mut differences = Vec::new();

    // Compare GPRs
    for i in 0..32 {
        let orig_val = original.get_register(i);
        let recomp_val = recompiled.get_register(i);
        if orig_val != recomp_val {
            differences.push(format!(
                "Register r{}: original=0x{:08X}, recompiled=0x{:08X}",
                i, orig_val, recomp_val
            ));
        }
    }

    // Compare special registers
    if original.pc != recompiled.pc {
        differences.push(format!(
            "PC: original=0x{:08X}, recompiled=0x{:08X}",
            original.pc, recompiled.pc
        ));
    }
    if original.lr != recompiled.lr {
        differences.push(format!(
            "LR: original=0x{:08X}, recompiled=0x{:08X}",
            original.lr, recompiled.lr
        ));
    }
    if original.ctr != recompiled.ctr {
        differences.push(format!(
            "CTR: original=0x{:08X}, recompiled=0x{:08X}",
            original.ctr, recompiled.ctr
        ));
    }
    if original.cr != recompiled.cr {
        differences.push(format!(
            "CR: original=0x{:08X}, recompiled=0x{:08X}",
            original.cr, recompiled.cr
        ));
    }

    ComparisonResult {
        passed: differences.is_empty(),
        differences,
    }
}

/// Compare memory state between original and recompiled execution.
pub fn compare_memory_state(
    original: &MemoryManager,
    recompiled: &MemoryManager,
    address: u32,
    size: usize,
) -> ComparisonResult {
    let mut differences = Vec::new();

    let orig_data = original.read_bytes(address, size).unwrap();
    let recomp_data = recompiled.read_bytes(address, size).unwrap();

    if orig_data != recomp_data {
        // Find specific differences
        for (i, (orig_byte, recomp_byte)) in orig_data.iter().zip(recomp_data.iter()).enumerate() {
            if orig_byte != recomp_byte {
                differences.push(format!(
                    "Memory at 0x{:08X}: original=0x{:02X}, recompiled=0x{:02X}",
                    address + i as u32, orig_byte, recomp_byte
                ));
            }
        }
    }

    ComparisonResult {
        passed: differences.is_empty(),
        differences,
    }
}

/// Compare floating-point values with tolerance.
pub fn compare_float_with_tolerance(original: f64, recompiled: f64, tolerance: f64) -> bool {
    (original - recompiled).abs() <= tolerance
}

/// Compare execution results with fuzzy matching for floats.
pub fn compare_execution_results(
    original_ctx: &CpuContext,
    recompiled_ctx: &CpuContext,
    original_memory: &MemoryManager,
    recompiled_memory: &MemoryManager,
    memory_region: Option<(u32, usize)>,
    float_tolerance: f64,
) -> ComparisonResult {
    let mut differences = Vec::new();

    // Compare registers
    let reg_result = compare_register_state(original_ctx, recompiled_ctx);
    if !reg_result.passed {
        differences.extend(reg_result.differences);
    }

    // Compare floating-point registers with tolerance
    for i in 0..32 {
        let orig_fpr = original_ctx.get_fpr(i);
        let recomp_fpr = recompiled_ctx.get_fpr(i);
        if !compare_float_with_tolerance(orig_fpr, recomp_fpr, float_tolerance) {
            differences.push(format!(
                "FPR f{}: original={}, recompiled={} (diff > {})",
                i, orig_fpr, recomp_fpr, float_tolerance
            ));
        }
    }

    // Compare memory if region specified
    if let Some((address, size)) = memory_region {
        let mem_result = compare_memory_state(original_memory, recompiled_memory, address, size);
        if !mem_result.passed {
            differences.extend(mem_result.differences);
        }
    }

    ComparisonResult {
        passed: differences.is_empty(),
        differences,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_comparison() {
        let mut orig = mock_cpu_context();
        let mut recomp = mock_cpu_context();

        orig.set_register(3, 0x12345678);
        recomp.set_register(3, 0x12345678);

        let result = compare_register_state(&orig, &recomp);
        assert!(result.passed);
    }

    #[test]
    fn test_register_comparison_difference() {
        let mut orig = mock_cpu_context();
        let mut recomp = mock_cpu_context();

        orig.set_register(3, 0x12345678);
        recomp.set_register(3, 0x87654321);

        let result = compare_register_state(&orig, &recomp);
        assert!(!result.passed);
        assert!(!result.differences.is_empty());
    }
}

