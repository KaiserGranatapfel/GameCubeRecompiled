//! Pointer Analysis
//!
//! This module provides points-to analysis and pointer aliasing detection.

use crate::recompiler::decoder::DecodedInstruction;
use std::collections::{HashMap, HashSet};

/// Points-to set for a pointer.
pub type PointsToSet = HashSet<u32>;

/// Pointer analysis results.
pub struct PointerAnalysis {
    /// Map from pointer register to points-to set
    points_to: HashMap<u8, PointsToSet>,
    /// Map from register to aliases
    aliases: HashMap<u8, HashSet<u8>>,
}

impl PointerAnalysis {
    /// Create new pointer analysis.
    pub fn new() -> Self {
        Self {
            points_to: HashMap::new(),
            aliases: HashMap::new(),
        }
    }

    /// Analyze instructions for pointer operations.
    pub fn analyze(instructions: &[DecodedInstruction]) -> Self {
        let mut analysis = Self::new();

        for inst in instructions {
            // Detect pointer operations:
            // - Load address (li, lis, addi with base register)
            // - Pointer arithmetic (add, sub)
            // - Load/store operations (indicate pointer usage)

            if is_pointer_operation(inst) {
                analyze_pointer_operation(&mut analysis, inst);
            }
        }

        analysis
    }

    /// Get points-to set for a register.
    pub fn get_points_to(&self, reg: u8) -> Option<&PointsToSet> {
        self.points_to.get(&reg)
    }

    /// Check if two registers may alias.
    pub fn may_alias(&self, reg1: u8, reg2: u8) -> bool {
        if let (Some(set1), Some(set2)) = (self.points_to.get(&reg1), self.points_to.get(&reg2)) {
            !set1.is_disjoint(set2)
        } else {
            false
        }
    }
}

impl Default for PointerAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if an instruction is a pointer operation.
fn is_pointer_operation(inst: &DecodedInstruction) -> bool {
    use crate::recompiler::decoder::InstructionType;
    // Check for load/store, address computation, pointer arithmetic
    matches!(
        inst.instruction.instruction_type,
        InstructionType::Load
            | InstructionType::Store
            | InstructionType::LoadImm // li, lis (load immediate address)
            | InstructionType::Arithmetic // addi, addis (pointer arithmetic)
    )
}

/// Analyze a pointer operation.
///
/// Extracts pointer information from instruction and updates points-to sets and aliases.
fn analyze_pointer_operation(analysis: &mut PointerAnalysis, inst: &DecodedInstruction) {
    use crate::recompiler::decoder::{InstructionType, Operand};

    match inst.instruction.instruction_type {
        InstructionType::Load | InstructionType::Store => {
            // Load/store operations indicate pointer usage
            if let Some(Operand::Register(base_reg)) = inst.instruction.operands.get(1) {
                // Base register is likely a pointer
                let points_to = analysis.points_to.entry(*base_reg).or_insert_with(HashSet::new);
                
                // Try to extract target address from instruction
                // For load/store with immediate offset, we can infer the base address
                if let Some(Operand::Immediate(offset)) = inst.instruction.operands.get(2) {
                    // This is a load/store with base + offset
                    // The base register points to addresses around the offset
                    // Simplified: just mark it as a pointer
                    points_to.insert(inst.address.wrapping_add(*offset as u32));
                } else {
                    // Unknown target, mark as pointer
                    points_to.insert(0x80000000); // Default to RAM region
                }
            }
        }
        InstructionType::Arithmetic => {
            // Pointer arithmetic (addi, addis, add, sub)
            if inst.opcode == 14 || inst.opcode == 15 || inst.opcode == 31 {
                // addi, addis, or add (opcode 31 with sub-opcode)
                if let (Some(Operand::Register(dest)), Some(Operand::Register(src)), Some(Operand::Immediate(imm))) = (
                    inst.instruction.operands.get(0),
                    inst.instruction.operands.get(1),
                    inst.instruction.operands.get(2),
                ) {
                    // dest = src + imm (pointer arithmetic)
                    if let Some(src_points_to) = analysis.points_to.get(src) {
                        let dest_points_to = analysis.points_to.entry(*dest).or_insert_with(HashSet::new);
                        // Propagate points-to set with offset
                        for &addr in src_points_to {
                            dest_points_to.insert(addr.wrapping_add(*imm as u32));
                        }
                        
                        // Also create alias relationship
                        let aliases = analysis.aliases.entry(*dest).or_insert_with(HashSet::new);
                        aliases.insert(*src);
                    } else if *imm > 0 && *imm < 0x10000 {
                        // If src is not a known pointer but we're adding a small offset,
                        // it might be pointer arithmetic - mark dest as potential pointer
                        let dest_points_to = analysis.points_to.entry(*dest).or_insert_with(HashSet::new);
                        dest_points_to.insert(0x80000000); // Default to RAM region
                    }
                } else if let (Some(Operand::Register(dest)), Some(Operand::Register(src)), Some(Operand::Register(src2))) = (
                    inst.instruction.operands.get(0),
                    inst.instruction.operands.get(1),
                    inst.instruction.operands.get(2),
                ) {
                    // dest = src + src2 (register-based pointer arithmetic)
                    if let Some(src_points_to) = analysis.points_to.get(src) {
                        let dest_points_to = analysis.points_to.entry(*dest).or_insert_with(HashSet::new);
                        // Copy points-to set (conservative - don't know the exact offset)
                        dest_points_to.extend(src_points_to.iter());
                        
                        // Create alias relationship
                        let aliases = analysis.aliases.entry(*dest).or_insert_with(HashSet::new);
                        aliases.insert(*src);
                        aliases.insert(*src2);
                    }
                }
            }
        }
        InstructionType::LoadImm => {
            // Loading immediate address (li, lis) creates a pointer
            if let (Some(Operand::Register(dest)), Some(Operand::Immediate(imm))) = (
                inst.instruction.operands.get(0),
                inst.instruction.operands.get(1),
            ) {
                let points_to = analysis.points_to.entry(*dest).or_insert_with(HashSet::new);
                let addr = *imm as u32;
                // Check if address is in valid memory regions
                if addr >= 0x80000000 && addr < 0x81800000 {
                    points_to.insert(addr); // RAM region
                } else if addr >= 0xCC000000 && addr < 0xCC200000 {
                    points_to.insert(addr); // VRAM region
                } else if addr >= 0xC0000000 && addr < 0xD0000000 {
                    points_to.insert(addr); // ARAM region
                } else {
                    points_to.insert(addr); // Still track it
                }
            } else if let (Some(Operand::Register(dest)), Some(Operand::Immediate32(imm))) = (
                inst.instruction.operands.get(0),
                inst.instruction.operands.get(1),
            ) {
                // 32-bit immediate load
                let points_to = analysis.points_to.entry(*dest).or_insert_with(HashSet::new);
                points_to.insert(*imm);
            }
        }
        InstructionType::LoadMultiple | InstructionType::StoreMultiple => {
            // Load/store multiple operations indicate pointer usage
            if let Some(Operand::Register(base_reg)) = inst.instruction.operands.get(0) {
                let points_to = analysis.points_to.entry(*base_reg).or_insert_with(HashSet::new);
                points_to.insert(0x80000000); // Default to RAM region
            }
        }
        _ => {
            // Other operations might involve pointers but are harder to analyze
        }
    }
}
