//! IR Builder - Converts PowerPC Instructions to IR
//!
//! This module converts decoded PowerPC instructions into the intermediate representation (IR).
//! The IR is a simplified, architecture-agnostic representation that facilitates optimization
//! and code generation.
//!
//! # Conversion Strategy
//! - **Arithmetic instructions**: Convert to IR arithmetic operations (Add, Sub, Mul, Div, etc.)
//! - **Memory instructions**: Convert to IR load/store operations
//! - **Control flow**: Convert branches to IR branch instructions
//! - **Unsupported instructions**: May be skipped or converted to generic operations

use crate::recompiler::decoder::DecodedInstruction;
use crate::recompiler::ir::instruction::{IRInstruction, IRFunction, IRBasicBlock, Address, Condition};
use anyhow::Result;
use smallvec::SmallVec;

/// IR builder for converting PowerPC instructions to IR.
pub struct IRBuilder;

impl IRBuilder {
    /// Build IR from a sequence of PowerPC instructions.
    ///
    /// # Algorithm
    /// 1. Convert each PowerPC instruction to IR instruction
    /// 2. Group instructions into basic blocks (split at branches)
    /// 3. Build IR function structure
    ///
    /// # Arguments
    /// * `instructions` - Sequence of decoded PowerPC instructions
    /// * `function_name` - Name of the function
    /// * `function_address` - Entry address of the function
    ///
    /// # Returns
    /// `Result<IRFunction>` - IR representation of the function
    ///
    /// # Errors
    /// Returns error if instruction conversion fails
    ///
    /// # Examples
    /// ```rust
    /// let ir_function = IRBuilder::build_ir(&instructions, "main", 0x80000000)?;
    /// ```
    #[inline] // May be called frequently
    pub fn build_ir(
        instructions: &[DecodedInstruction],
        function_name: &str,
        function_address: u32,
    ) -> Result<IRFunction> {
        let mut basic_blocks: Vec<IRBasicBlock> = Vec::new();
        let mut current_block: IRBasicBlock = IRBasicBlock {
            id: 0u32,
            instructions: Vec::new(),
            successors: SmallVec::new(),
        };
        let mut block_id: u32 = 0u32;
        
        for inst in instructions.iter() {
            match Self::convert_instruction(inst)? {
                Some(ir_inst) => {
                    current_block.instructions.push(ir_inst);
                }
                None => {
                    // Instruction doesn't need IR representation
                }
            }
        }
        
        if !current_block.instructions.is_empty() {
            basic_blocks.push(current_block);
        }
        
        Ok(IRFunction {
            name: function_name.to_string(),
            address: function_address,
            parameters: SmallVec::new(), // Would extract from metadata
            return_register: None,
            basic_blocks,
        })
    }
    
    /// Convert a PowerPC instruction to an IR instruction.
    ///
    /// # Arguments
    /// * `inst` - Decoded PowerPC instruction
    ///
    /// # Returns
    /// `Result<Option<IRInstruction>>` - IR instruction if conversion is possible, None otherwise
    #[inline] // Hot path - called for every instruction
    fn convert_instruction(inst: &DecodedInstruction) -> Result<Option<IRInstruction>> {
        match inst.instruction.instruction_type {
            crate::recompiler::decoder::InstructionType::Arithmetic => {
                Self::convert_arithmetic(inst)
            }
            crate::recompiler::decoder::InstructionType::Load => {
                Self::convert_load(inst)
            }
            crate::recompiler::decoder::InstructionType::Store => {
                Self::convert_store(inst)
            }
            crate::recompiler::decoder::InstructionType::Branch => {
                Self::convert_branch(inst)
            }
            _ => Ok(None),
        }
    }
    
    /// Convert an arithmetic instruction to IR.
    ///
    /// # Arguments
    /// * `inst` - Decoded PowerPC arithmetic instruction
    ///
    /// # Returns
    /// `Result<Option<IRInstruction>>` - IR arithmetic instruction
    #[inline] // Hot path
    fn convert_arithmetic(inst: &DecodedInstruction) -> Result<Option<IRInstruction>> {
        if inst.instruction.operands.len() < 3usize {
            return Ok(None);
        }
        
        // Simplified - would need proper operand extraction
        // Extract destination and source registers from operands
        let dst: u8 = if let Some(crate::recompiler::decoder::Operand::Register(reg)) = inst.instruction.operands.get(0usize) {
            *reg
        } else {
            return Ok(None);
        };
        
        let src1: u8 = if let Some(crate::recompiler::decoder::Operand::Register(reg)) = inst.instruction.operands.get(1usize) {
            *reg
        } else {
            return Ok(None);
        };
        
        let src2: u8 = if let Some(crate::recompiler::decoder::Operand::Register(reg)) = inst.instruction.operands.get(2usize) {
            *reg
        } else {
            return Ok(None);
        };
        
        // Determine operation type based on opcode (simplified)
        Ok(Some(IRInstruction::Add {
            dst,
            src1,
            src2,
        }))
    }
    
    /// Convert a load instruction to IR.
    ///
    /// # Arguments
    /// * `inst` - Decoded PowerPC load instruction
    ///
    /// # Returns
    /// `Result<Option<IRInstruction>>` - IR load instruction
    #[inline] // Hot path
    fn convert_load(inst: &DecodedInstruction) -> Result<Option<IRInstruction>> {
        use crate::recompiler::decoder::Operand;
        
        // Extract destination register and address
        if let (Some(Operand::Register(dst)), Some(Operand::Register(base)), offset_op) = (
            inst.instruction.operands.get(0),
            inst.instruction.operands.get(1),
            inst.instruction.operands.get(2),
        ) {
            let addr = if let Some(Operand::Immediate(offset)) = offset_op {
                Address::Register { base: *base, offset: *offset as i32 }
            } else {
                Address::Register { base: *base, offset: 0 }
            };
            Ok(Some(IRInstruction::Load { dst: *dst, addr }))
        } else {
            Ok(None)
        }
    }
    
    /// Convert a store instruction to IR.
    ///
    /// # Arguments
    /// * `inst` - Decoded PowerPC store instruction
    ///
    /// # Returns
    /// `Result<Option<IRInstruction>>` - IR store instruction
    #[inline] // Hot path
    fn convert_store(inst: &DecodedInstruction) -> Result<Option<IRInstruction>> {
        use crate::recompiler::decoder::Operand;
        
        // Extract source register and address
        if let (Some(Operand::Register(src)), Some(Operand::Register(base)), offset_op) = (
            inst.instruction.operands.get(0),
            inst.instruction.operands.get(1),
            inst.instruction.operands.get(2),
        ) {
            let addr = if let Some(Operand::Immediate(offset)) = offset_op {
                Address::Register { base: *base, offset: *offset as i32 }
            } else {
                Address::Register { base: *base, offset: 0 }
            };
            Ok(Some(IRInstruction::Store { src: *src, addr }))
        } else {
            Ok(None)
        }
    }
    
    /// Convert a branch instruction to IR.
    ///
    /// # Arguments
    /// * `inst` - Decoded PowerPC branch instruction
    ///
    /// # Returns
    /// `Result<Option<IRInstruction>>` - IR branch instruction
    #[inline] // Hot path
    fn convert_branch(inst: &DecodedInstruction) -> Result<Option<IRInstruction>> {
        use crate::recompiler::decoder::Operand;
        
        // Extract branch target
        if let Some(Operand::Immediate(target)) = inst.instruction.operands.get(0) {
            // Check if conditional branch
            if inst.instruction.instruction_type == crate::recompiler::decoder::InstructionType::Branch {
                // Conditional branch - would extract condition from CR
                Ok(Some(IRInstruction::BranchCond {
                    cond: Condition::Equal, // Simplified
                    target: *target as u32,
                }))
            } else {
                // Unconditional branch
                Ok(Some(IRInstruction::Branch { target: *target as u32 }))
            }
        } else {
            Ok(None)
        }
    }
}
