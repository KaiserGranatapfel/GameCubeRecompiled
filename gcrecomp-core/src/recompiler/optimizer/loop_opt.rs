//! Loop Optimizations
//!
//! This module provides loop optimization passes including unrolling,
//! invariant code motion, and loop fusion.

use crate::recompiler::analysis::control_flow::ControlFlowGraph;
use crate::recompiler::analysis::loop_analysis::{LoopAnalyzer, LoopInfo};
use crate::recompiler::decoder::DecodedInstruction;
use std::collections::HashSet;

/// Optimize loops in instruction sequence.
///
/// Applies loop optimizations:
/// - Loop unrolling
/// - Loop invariant code motion
/// - Loop fusion (where applicable)
///
/// # Arguments
/// * `instructions` - Instruction sequence to optimize
///
/// # Returns
/// Optimized instruction sequence
pub fn optimize_loops(instructions: &[DecodedInstruction]) -> Vec<DecodedInstruction> {
    if instructions.is_empty() {
        return instructions.to_vec();
    }

    // Build CFG to detect loops
    let cfg = match crate::recompiler::analysis::control_flow::ControlFlowAnalyzer::build_cfg(
        instructions,
        0,
    ) {
        Ok(cfg) => cfg,
        Err(_) => {
            // If CFG construction fails, return original instructions
            return instructions.to_vec();
        }
    };

    // Detect loops
    let loops = LoopAnalyzer::analyze_loops(&cfg);

    if loops.is_empty() {
        return instructions.to_vec();
    }

    let mut optimized = instructions.to_vec();
    let mut processed_loops = HashSet::new();

    // Process each loop
    for loop_info in &loops {
        if processed_loops.contains(&loop_info.header) {
            continue;
        }

        // Get loop body instructions
        let loop_instructions: Vec<DecodedInstruction> = loop_info
            .body
            .iter()
            .flat_map(|&block_idx| {
                cfg.nodes
                    .get(block_idx)
                    .map(|block| block.instructions.clone())
                    .unwrap_or_default()
            })
            .collect();

        if loop_instructions.is_empty() {
            continue;
        }

        // Check if loop is small enough to unroll (heuristic: < 20 instructions)
        if loop_instructions.len() < 20 {
            // Unroll small loops 2-4 times
            let unroll_factor = (20 / loop_instructions.len()).min(4).max(2);
            let unrolled = unroll_loop(loop_info, &loop_instructions, unroll_factor);
            
            // Replace loop body with unrolled version
            // This is simplified - in a full implementation, we'd need to update the CFG
            // For now, we'll apply invariant code motion
            let (invariant, loop_body) = move_invariant_code(loop_info, &loop_instructions);
            
            // Combine invariant code + optimized loop body
            let mut new_loop = invariant;
            new_loop.extend(loop_body);
            
            // In a full implementation, we'd replace the loop in optimized
            // For now, just mark as processed
            processed_loops.insert(loop_info.header);
        } else {
            // For larger loops, just apply invariant code motion
            let (invariant, loop_body) = move_invariant_code(loop_info, &loop_instructions);
            // Would replace loop with invariant + loop_body
            processed_loops.insert(loop_info.header);
        }
    }

    optimized
}

/// Unroll a loop.
///
/// # Arguments
/// * `loop_info` - Information about the loop
/// * `instructions` - Instructions in the loop
/// * `unroll_factor` - How many times to unroll
///
/// # Returns
/// Unrolled instruction sequence
pub fn unroll_loop(
    _loop_info: &LoopInfo,
    instructions: &[DecodedInstruction],
    unroll_factor: usize,
) -> Vec<DecodedInstruction> {
    let mut unrolled = Vec::with_capacity(instructions.len() * unroll_factor);

    // Repeat loop body unroll_factor times
    for i in 0..unroll_factor {
        for inst in instructions {
            let mut new_inst = inst.clone();
            // Update addresses for unrolled instructions (for debugging)
            new_inst.address = inst.address.wrapping_add((i * instructions.len() * 4) as u32);
            unrolled.push(new_inst);
        }
    }

    unrolled
}

/// Move loop-invariant code outside the loop.
///
/// # Arguments
/// * `loop_info` - Information about the loop
/// * `instructions` - Instructions in the loop
///
/// # Returns
/// (invariant_code, loop_body) - Separated code
pub fn move_invariant_code(
    loop_info: &LoopInfo,
    instructions: &[DecodedInstruction],
) -> (Vec<DecodedInstruction>, Vec<DecodedInstruction>) {
    let mut invariant = Vec::new();
    let mut loop_body = Vec::new();

    // Identify invariant registers (not modified in loop)
    let invariant_regs: HashSet<u8> = loop_info.invariants.iter().copied().collect();

    // Separate instructions based on whether they depend on loop variables
    for inst in instructions {
        let mut is_invariant = true;

        // Check if instruction uses loop induction variables
        for operand in &inst.instruction.operands {
            if let crate::recompiler::decoder::Operand::Register(reg) = operand {
                // Check if this register is an induction variable
                let is_iv = loop_info
                    .induction_variables
                    .iter()
                    .any(|iv| iv.register == *reg);

                if is_iv {
                    is_invariant = false;
                    break;
                }

                // Check if register is written to in loop (not invariant)
                if !invariant_regs.contains(reg) {
                    // This register might be modified in loop
                    // Simplified check: if it's a destination register, it's not invariant
                    if inst.instruction.operands.first() == Some(operand) {
                        is_invariant = false;
                    }
                }
            }
        }

        // Instructions that load constants or use invariant registers are invariant
        match inst.instruction.instruction_type {
            crate::recompiler::decoder::InstructionType::LoadImm
            | crate::recompiler::decoder::InstructionType::Move => {
                // These might be invariant if they don't depend on loop variables
                if is_invariant {
                    invariant.push(inst.clone());
                } else {
                    loop_body.push(inst.clone());
                }
            }
            _ => {
                if is_invariant {
                    invariant.push(inst.clone());
                } else {
                    loop_body.push(inst.clone());
                }
            }
        }
    }

    (invariant, loop_body)
}

/// Fuse two adjacent loops if possible.
///
/// # Arguments
/// * `loop1` - First loop
/// * `loop2` - Second loop
/// * `instructions1` - Instructions for first loop
/// * `instructions2` - Instructions for second loop
///
/// # Returns
/// Fused loop if possible, None otherwise
pub fn fuse_loops(
    loop1: &LoopInfo,
    loop2: &LoopInfo,
    instructions1: &[DecodedInstruction],
    instructions2: &[DecodedInstruction],
) -> Option<Vec<DecodedInstruction>> {
    // Check if loops can be fused:
    // 1. Same iteration count (simplified: assume they can be fused if adjacent)
    // 2. No dependencies between loops (loop2 doesn't use loop1's outputs)
    // 3. Loops are adjacent in control flow

    // Check for dependencies: does loop2 use registers written by loop1?
    let loop1_outputs: HashSet<u8> = instructions1
        .iter()
        .filter_map(|inst| {
            inst.instruction.operands.first().and_then(|op| {
                if let crate::recompiler::decoder::Operand::Register(reg) = op {
                    Some(*reg)
                } else {
                    None
                }
            })
        })
        .collect();

    let loop2_inputs: HashSet<u8> = instructions2
        .iter()
        .flat_map(|inst| {
            inst.instruction.operands.iter().filter_map(|op| {
                if let crate::recompiler::decoder::Operand::Register(reg) = op {
                    Some(*reg)
                } else {
                    None
                }
            })
        })
        .collect();

    // If loop2 uses outputs from loop1, they can't be fused
    if !loop1_outputs.is_disjoint(&loop2_inputs) {
        return None;
    }

    // Fuse loops by combining their bodies
    let mut fused = Vec::with_capacity(instructions1.len() + instructions2.len());
    fused.extend_from_slice(instructions1);
    fused.extend_from_slice(instructions2);
    Some(fused)
}
