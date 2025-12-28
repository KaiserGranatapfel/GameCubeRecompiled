//! Data Flow Analysis
//!
//! This module provides data flow analysis for PowerPC instructions, including:
//! - Def-use chain construction (tracking where variables are defined and used)
//! - Live variable analysis (determining which variables are live at each point)
//! - Dead code elimination (removing unused definitions)
//!
//! # Memory Optimizations
//! - `DefUseChain.definitions` and `uses` use `SmallVec` (typically small lists)
//! - `Definition` and `Use` structs are packed to minimize padding
//! - Live variable sets use `BitVec` instead of `HashSet<u8>` for efficiency
//!   - Saves memory: 1 bit per register instead of 8 bytes (pointer) + overhead
//!   - Faster membership tests: O(1) bit access with better cache locality
//!
//! # Data Flow Analysis Algorithms
//! ## Def-Use Chain Construction
//! Scans instructions to identify:
//! - **Definitions**: Instructions that write to registers
//! - **Uses**: Instructions that read from registers
//!
//! ## Live Variable Analysis
//! Uses iterative data flow analysis (worklist algorithm):
//! - **Live at exit**: Union of live at entry of all successors
//! - **Live at entry**: (Live at exit - Killed) ∪ Generated
//!   - **Killed**: Registers defined in this block
//!   - **Generated**: Registers used in this block
//!
//! Iterates until fixed point (no changes).

use crate::recompiler::analysis::control_flow::ControlFlowGraph;
use crate::recompiler::decoder::{DecodedInstruction, Operand};
use bitvec::prelude::*;
use smallvec::SmallVec;
use std::collections::HashMap;

/// Definition-use chain for a register.
///
/// Tracks all definitions and uses of a register, enabling data flow optimizations.
///
/// # Memory Optimization
/// - `definitions` and `uses`: Use `SmallVec` - typically small lists (most registers have few defs/uses)
/// - `register`: Uses `u8` (PowerPC has 32 GPRs, fits in 5 bits)
#[derive(Debug, Clone)]
pub struct DefUseChain {
    /// Register number (0-31 for PowerPC GPRs)
    pub register: u8,
    /// Definitions of this register (instructions that write to it)
    /// Uses SmallVec - most registers have few definitions
    pub definitions: SmallVec<[Definition; 4]>,
    /// Uses of this register (instructions that read from it)
    /// Uses SmallVec - most registers have few uses
    pub uses: SmallVec<[Use; 8]>,
}

/// Definition of a register (instruction that writes to it).
///
/// # Memory Optimization
/// Packed struct to minimize padding:
/// - `instruction_index`: `usize` (required for indexing)
/// - `address`: `u32` (32-bit address space)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)] // Ensure C-compatible layout, minimize padding
pub struct Definition {
    /// Index of defining instruction in instruction sequence
    pub instruction_index: usize,
    /// Address of defining instruction in original binary
    pub address: u32,
}

/// Use of a register (instruction that reads from it).
///
/// # Memory Optimization
/// Packed struct to minimize padding:
/// - `instruction_index`: `usize` (required for indexing)
/// - `address`: `u32` (32-bit address space)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)] // Ensure C-compatible layout, minimize padding
pub struct Use {
    /// Index of using instruction in instruction sequence
    pub instruction_index: usize,
    /// Address of using instruction in original binary
    pub address: u32,
}

/// Live variable analysis results.
///
/// # Memory Optimization
/// - Live variable sets use `BitVec<u32>` instead of `HashSet<u8>`
///   - Saves memory: 1 bit per register (32 bits = 4 bytes) vs 8 bytes per register + overhead
///   - Faster membership tests: O(1) bit access with better cache locality
///   - Efficient set operations: bitwise AND/OR for intersection/union
///
/// # Algorithm
/// Uses iterative data flow analysis:
/// - Initialize all blocks with empty live sets
/// - Iterate until fixed point:
///   - Live at exit = union of live at entry of successors
///   - Live at entry = (Live at exit - Killed) ∪ Generated
#[derive(Debug, Clone)]
pub struct LiveVariableAnalysis {
    /// Live variables at entry of each basic block
    /// Key: block ID (u32), Value: bit vector of live registers (1 bit per register)
    pub live_at_entry: HashMap<u32, BitVec<u32>>,
    /// Live variables at exit of each basic block
    /// Key: block ID (u32), Value: bit vector of live registers (1 bit per register)
    pub live_at_exit: HashMap<u32, BitVec<u32>>,
}

/// Data flow analyzer for building def-use chains and performing live variable analysis.
pub struct DataFlowAnalyzer;

impl DataFlowAnalyzer {
    /// Build definition-use chains for all registers.
    ///
    /// # Algorithm
    /// Scans all instructions to identify:
    /// - **Definitions**: Instructions that write to registers (destination operands)
    /// - **Uses**: Instructions that read from registers (source operands)
    ///
    /// # Arguments
    /// * `instructions` - Sequence of decoded PowerPC instructions
    ///
    /// # Returns
    /// `HashMap<u8, DefUseChain>` - Map from register number to def-use chain
    ///
    /// # Examples
    /// ```rust
    /// let instructions = vec![/* decoded instructions */];
    /// let chains = DataFlowAnalyzer::build_def_use_chains(&instructions);
    /// if let Some(chain) = chains.get(&3) {
    ///     println!("Register r3 has {} definitions and {} uses",
    ///              chain.definitions.len(), chain.uses.len());
    /// }
    /// ```
    #[inline] // May be called frequently
    pub fn build_def_use_chains(instructions: &[DecodedInstruction]) -> HashMap<u8, DefUseChain> {
        let mut chains: HashMap<u8, DefUseChain> = HashMap::new();
        let mut definitions: HashMap<u8, SmallVec<[Definition; 4]>> = HashMap::new();
        let mut uses: HashMap<u8, SmallVec<[Use; 8]>> = HashMap::new();

        let mut instruction_address: u32 = 0u32;
        for (idx, inst) in instructions.iter().enumerate() {
            // Find definitions (instructions that write to registers)
            if let Some(def_reg) = Self::get_definition_register(inst) {
                definitions
                    .entry(def_reg)
                    .or_insert_with(SmallVec::new)
                    .push(Definition {
                        instruction_index: idx,
                        address: instruction_address,
                    });
            }

            // Find uses (instructions that read from registers)
            for use_reg in Self::get_use_registers(inst) {
                uses.entry(use_reg).or_insert_with(SmallVec::new).push(Use {
                    instruction_index: idx,
                    address: instruction_address,
                });
            }

            instruction_address = instruction_address.wrapping_add(4); // PowerPC instructions are 4 bytes
        }

        // Combine definitions and uses into chains
        // PowerPC has 32 GPRs (r0-r31)
        for reg in 0u8..32u8 {
            let defs: SmallVec<[Definition; 4]> = definitions.remove(&reg).unwrap_or_default();
            let uses_list: SmallVec<[Use; 8]> = uses.remove(&reg).unwrap_or_default();

            if !defs.is_empty() || !uses_list.is_empty() {
                chains.insert(
                    reg,
                    DefUseChain {
                        register: reg,
                        definitions: defs,
                        uses: uses_list,
                    },
                );
            }
        }

        chains
    }

    /// Extract the register that is defined (written to) by an instruction.
    ///
    /// # Algorithm
    /// Checks if instruction writes to a register:
    /// - Arithmetic instructions: first operand is destination
    /// - Load instructions: first operand is destination
    /// - Move instructions: first operand is destination
    ///
    /// # Arguments
    /// * `inst` - Decoded instruction to analyze
    ///
    /// # Returns
    /// `Option<u8>` - Register number if instruction defines a register, None otherwise
    #[inline] // Hot path - called for every instruction
    fn get_definition_register(inst: &DecodedInstruction) -> Option<u8> {
        // Check if instruction writes to a register (first operand is usually destination)
        if !inst.instruction.operands.is_empty() {
            if let Operand::Register(reg) = &inst.instruction.operands[0] {
                // Check if this is a write operation
                match inst.instruction.instruction_type {
                    crate::recompiler::decoder::InstructionType::Arithmetic
                    | crate::recompiler::decoder::InstructionType::Load
                    | crate::recompiler::decoder::InstructionType::Move => {
                        return Some(*reg);
                    }
                    _ => {}
                }
            }
        }
        None
    }

    /// Extract all registers that are used (read from) by an instruction.
    ///
    /// # Algorithm
    /// Scans all operands for register uses, excluding the definition register (if any).
    ///
    /// # Arguments
    /// * `inst` - Decoded instruction to analyze
    ///
    /// # Returns
    /// `SmallVec<[u8; 4]>` - List of register numbers used by this instruction
    #[inline] // Hot path - called for every instruction
    fn get_use_registers(inst: &DecodedInstruction) -> SmallVec<[u8; 4]> {
        let mut uses: SmallVec<[u8; 4]> = SmallVec::new();

        // Check all operands for register uses
        let def_reg: Option<u8> = Self::get_definition_register(inst);
        for operand in inst.instruction.operands.iter() {
            match operand {
                Operand::Register(reg) => {
                    // Skip if this is the definition register
                    if def_reg != Some(*reg) {
                        uses.push(*reg);
                    }
                }
                Operand::FpRegister(_) => {
                    // Floating-point registers are also uses
                    // (for now, we only track GPRs)
                }
                _ => {}
            }
        }

        uses
    }

    /// Perform live variable analysis on a control flow graph.
    ///
    /// # Algorithm
    /// Uses iterative data flow analysis (worklist algorithm):
    /// 1. Initialize all blocks with empty live sets
    /// 2. Iterate until fixed point:
    ///    - Live at exit = union of live at entry of all successors
    ///    - Live at entry = (Live at exit - Killed) ∪ Generated
    ///      - **Killed**: Registers defined in this block
    ///      - **Generated**: Registers used in this block
    ///
    /// # Arguments
    /// * `cfg` - Control flow graph to analyze
    ///
    /// # Returns
    /// `LiveVariableAnalysis` - Live variable sets at entry and exit of each block
    ///
    /// # Examples
    /// ```rust
    /// let live_analysis = DataFlowAnalyzer::live_variable_analysis(&cfg);
    /// if let Some(live) = live_analysis.live_at_entry.get(&0) {
    ///     println!("Block 0 has {} live variables at entry", live.count_ones());
    /// }
    /// ```
    #[inline] // May be called frequently
    pub fn live_variable_analysis(cfg: &ControlFlowGraph) -> LiveVariableAnalysis {
        let mut live_at_entry: HashMap<u32, BitVec<u32>> = HashMap::new();
        let mut live_at_exit: HashMap<u32, BitVec<u32>> = HashMap::new();
        let mut changed: bool = true;

        // Initialize all blocks with empty live sets
        // Use BitVec with 32 bits (one per PowerPC GPR)
        for block in cfg.nodes.iter() {
            live_at_entry.insert(block.id, bitvec![u32, Lsb0; 0; 32]);
            live_at_exit.insert(block.id, bitvec![u32, Lsb0; 0; 32]);
        }

        // Iterative data flow analysis
        while changed {
            changed = false;

            for block in cfg.nodes.iter() {
                // Compute live at exit (union of live at entry of successors)
                let mut exit_live: BitVec<u32> = bitvec![u32, Lsb0; 0; 32];
                for &succ in block.successors.iter() {
                    if let Some(entry_live) = live_at_entry.get(&succ) {
                        exit_live |= entry_live; // Bitwise OR for union
                    }
                }

                // Compute live at entry (gen ∪ (exit - kill))
                let mut entry_live: BitVec<u32> = exit_live.clone();

                // Remove killed registers (defined in this block)
                for inst in block.instructions.iter() {
                    if let Some(killed) = Self::get_definition_register(inst) {
                        entry_live.set(killed as usize, false);
                    }
                }

                // Add generated registers (used in this block)
                for inst in block.instructions.iter() {
                    for used in Self::get_use_registers(inst).iter() {
                        entry_live.set(*used as usize, true);
                    }
                }

                // Check if changed
                let old_entry: BitVec<u32> = live_at_entry
                    .get(&block.id)
                    .cloned()
                    .unwrap_or_else(|| bitvec![u32, Lsb0; 0; 32]);
                if old_entry != entry_live {
                    changed = true;
                    live_at_entry.insert(block.id, entry_live);
                    live_at_exit.insert(block.id, exit_live);
                }
            }
        }

        LiveVariableAnalysis {
            live_at_entry,
            live_at_exit,
        }
    }

    /// Eliminate dead code using live variable analysis.
    ///
    /// # Algorithm
    /// Removes instructions that define registers that are never used.
    /// This is a simple implementation - a full implementation would need to track
    /// uses across basic blocks using live variable analysis.
    ///
    /// # Arguments
    /// * `instructions` - Sequence of decoded PowerPC instructions
    /// * `live_analysis` - Live variable analysis results
    ///
    /// # Returns
    /// `Vec<DecodedInstruction>` - Optimized instruction sequence with dead code removed
    ///
    /// # Examples
    /// ```rust
    /// let optimized = DataFlowAnalyzer::eliminate_dead_code(&instructions, &live_analysis);
    /// println!("Removed {} dead instructions", instructions.len() - optimized.len());
    /// ```
    #[inline] // May be called frequently
    pub fn eliminate_dead_code(
        instructions: &[DecodedInstruction],
        _live_analysis: &LiveVariableAnalysis,
    ) -> Vec<DecodedInstruction> {
        let mut optimized: Vec<DecodedInstruction> = Vec::new();

        // Simple dead code elimination: remove definitions that are never used
        // In a full implementation, would use live_analysis to track uses across blocks
        for inst in instructions.iter() {
            if let Some(def_reg) = Self::get_definition_register(inst) {
                // Check if register is ever used
                let mut is_used: bool = false;
                for other_inst in instructions.iter() {
                    if Self::get_use_registers(other_inst).contains(&def_reg) {
                        is_used = true;
                        break;
                    }
                }

                if is_used {
                    optimized.push(inst.clone());
                }
                // Otherwise, skip (dead code)
            } else {
                optimized.push(inst.clone());
            }
        }

        optimized
    }
}
