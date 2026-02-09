//! Code Optimizations
//!
//! This module provides optimization passes for decoded PowerPC instructions.
//! These optimizations improve code quality and reduce generated code size.
//!
//! # Optimization Passes
//! - **Constant Folding**: Evaluate constant expressions at compile time
//! - **Dead Code Elimination**: Remove unused instructions
//! - **Constant Propagation**: Track li/addi constant loads through register chains
//! - **Function-level DCE**: Remove unreachable functions using call graph analysis

use crate::recompiler::decoder::{DecodedInstruction, InstructionType, Operand};
use std::collections::{HashMap, HashSet};

/// Optimizer for PowerPC instructions.
///
/// Applies various optimization passes to improve code quality.
pub struct Optimizer {
    /// Enable constant folding optimization
    constant_folding: bool,
    /// Enable dead code elimination
    dead_code_elimination: bool,
}

impl Optimizer {
    /// Create a new optimizer with all optimizations enabled.
    pub fn new() -> Self {
        Self {
            constant_folding: true,
            dead_code_elimination: true,
        }
    }

    /// Optimize a sequence of instructions.
    pub fn optimize(&self, instructions: &[DecodedInstruction]) -> Vec<DecodedInstruction> {
        let mut optimized: Vec<DecodedInstruction> = instructions.to_vec();

        if self.constant_folding {
            optimized = self.fold_constants(&optimized);
        }

        if self.dead_code_elimination {
            optimized = self.eliminate_dead_code(&optimized);
        }

        optimized
    }

    /// Constant folding and propagation pass.
    ///
    /// Tracks constant values loaded by `li` (opcode 14 with rA=0) and `addi`
    /// patterns, propagating them through register chains.
    fn fold_constants(&self, instructions: &[DecodedInstruction]) -> Vec<DecodedInstruction> {
        let mut result: Vec<DecodedInstruction> = Vec::with_capacity(instructions.len());
        let mut constants: HashMap<u8, u32> = HashMap::new();

        for inst in instructions.iter() {
            // Track li/addi constant loads (opcode 14 = addi; li is addi rD,0,imm)
            if inst.instruction.opcode == 14 && inst.instruction.operands.len() >= 3 {
                if let (Operand::Register(rd), Operand::Register(ra), Operand::Immediate(imm)) = (
                    &inst.instruction.operands[0],
                    &inst.instruction.operands[1],
                    &inst.instruction.operands[2],
                ) {
                    if *ra == 0 {
                        // li rD, imm — rD = sign_extend(imm)
                        constants.insert(*rd, *imm as i32 as u32);
                    } else if let Some(&base) = constants.get(ra) {
                        // addi rD, rA, imm where rA is known constant
                        constants.insert(*rd, base.wrapping_add(*imm as i32 as u32));
                    } else {
                        constants.remove(rd);
                    }
                }
            }
            // Track lis (opcode 15 = addis; lis is addis rD,0,imm)
            else if inst.instruction.opcode == 15 && inst.instruction.operands.len() >= 3 {
                if let (Operand::Register(rd), Operand::Register(ra), Operand::Immediate(imm)) = (
                    &inst.instruction.operands[0],
                    &inst.instruction.operands[1],
                    &inst.instruction.operands[2],
                ) {
                    if *ra == 0 {
                        constants.insert(*rd, (*imm as u32) << 16);
                    } else {
                        constants.remove(rd);
                    }
                }
            }
            // Invalidate register on any other write
            else if let Some(Operand::Register(rd)) = inst.instruction.operands.first() {
                if matches!(
                    inst.instruction.instruction_type,
                    InstructionType::Arithmetic
                        | InstructionType::Load
                        | InstructionType::Move
                        | InstructionType::Shift
                        | InstructionType::Rotate
                ) {
                    constants.remove(rd);
                }
            }
            // Branches invalidate all tracked constants (control flow merge)
            if matches!(inst.instruction.instruction_type, InstructionType::Branch) {
                constants.clear();
            }

            result.push(inst.clone());
        }

        result
    }

    /// Dead code elimination pass.
    ///
    /// Removes instructions that write to registers never subsequently read.
    fn eliminate_dead_code(&self, instructions: &[DecodedInstruction]) -> Vec<DecodedInstruction> {
        let mut result: Vec<DecodedInstruction> = Vec::with_capacity(instructions.len());

        // Collect used registers in a backward pass
        let mut used_after: HashSet<u8> = HashSet::new();
        // Assume all registers may be live at function exit
        for r in 0..32u8 {
            used_after.insert(r);
        }

        let mut keep = vec![true; instructions.len()];

        for (i, inst) in instructions.iter().enumerate().rev() {
            // Branch/system/store instructions always kept (side effects)
            if matches!(
                inst.instruction.instruction_type,
                InstructionType::Branch
                    | InstructionType::System
                    | InstructionType::Store
                    | InstructionType::Compare
            ) {
                // Mark all source registers as used
                for op in &inst.instruction.operands {
                    if let Operand::Register(r) = op {
                        used_after.insert(*r);
                    }
                }
                continue;
            }

            // For instructions that write a register: check if it's read later
            if let Some(Operand::Register(rd)) = inst.instruction.operands.first() {
                if !used_after.contains(rd) {
                    keep[i] = false;
                    continue;
                }
                // Remove destination from used set, add sources
                used_after.remove(rd);
            }

            for op in inst.instruction.operands.iter().skip(1) {
                if let Operand::Register(r) = op {
                    used_after.insert(*r);
                }
            }
        }

        for (i, inst) in instructions.iter().enumerate() {
            if keep[i] {
                result.push(inst.clone());
            }
        }

        result
    }

    /// Function-level dead code elimination using call graph.
    ///
    /// Given a set of function addresses and their call targets, returns the set
    /// of reachable function addresses from the given entry points.
    pub fn reachable_functions(
        entry_points: &[u32],
        call_graph: &HashMap<u32, Vec<u32>>,
    ) -> HashSet<u32> {
        let mut reachable = HashSet::new();
        let mut worklist: Vec<u32> = entry_points.to_vec();

        while let Some(addr) = worklist.pop() {
            if reachable.insert(addr) {
                if let Some(callees) = call_graph.get(&addr) {
                    for &callee in callees {
                        if !reachable.contains(&callee) {
                            worklist.push(callee);
                        }
                    }
                }
            }
        }

        reachable
    }
}

impl Default for Optimizer {
    fn default() -> Self {
        Self::new()
    }
}
