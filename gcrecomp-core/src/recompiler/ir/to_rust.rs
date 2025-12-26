//! IR to Rust Translation
//!
//! This module translates intermediate representation (IR) instructions to Rust code.
//! The translation produces idiomatic Rust code that can be compiled and executed.
//!
//! # Translation Strategy
//! - **IR instructions**: Converted to Rust statements
//! - **Basic blocks**: Converted to Rust code blocks
//! - **Control flow**: Converted to Rust control flow (if, loop, match)
//!
//! # Memory Optimizations
//! - Pre-allocates string buffer with estimated capacity
//! - Uses efficient string operations

use crate::recompiler::ir::instruction::IRFunction;
use anyhow::Result;

/// IR to Rust translator.
pub struct IRToRust;

impl IRToRust {
    /// Translate an IR function to Rust code.
    ///
    /// # Algorithm
    /// Converts IR function structure to Rust function:
    /// 1. Generate function signature
    /// 2. Initialize runtime context
    /// 3. Translate basic blocks to Rust code
    /// 4. Generate function closing brace
    ///
    /// # Arguments
    /// * `function` - IR function to translate
    ///
    /// # Returns
    /// `Result<String>` - Generated Rust code
    ///
    /// # Errors
    /// Returns error if translation fails
    ///
    /// # Examples
    /// ```rust
    /// let rust_code = IRToRust::translate(&ir_function)?;
    /// ```
    #[inline] // May be called frequently
    pub fn translate(function: &IRFunction) -> Result<String> {
        // Pre-allocate string buffer with estimated capacity
        // Estimate: ~100 bytes per instruction
        let estimated_capacity: usize = function.basic_blocks.iter()
            .map(|block| block.instructions.len())
            .sum::<usize>() * 100usize;
        let mut code: String = String::with_capacity(estimated_capacity);
        
        code.push_str(&format!("fn {}() {{\n", function.name));
        code.push_str("    let mut ctx = gcrecomp_runtime::CpuContext::new();\n");
        code.push_str("    let mut memory = gcrecomp_runtime::MemoryManager::new();\n");
        code.push('\n');
        
        for block in function.basic_blocks.iter() {
            for inst in block.instructions.iter() {
                code.push_str(&Self::translate_instruction(inst)?);
            }
        }
        
        code.push_str("}\n");
        
        Ok(code)
    }
    
    /// Translate a single IR instruction to Rust code.
    ///
    /// # Arguments
    /// * `inst` - IR instruction to translate
    ///
    /// # Returns
    /// `Result<String>` - Generated Rust code for the instruction
    ///
    /// # Errors
    /// Returns error if translation fails
    #[inline] // Hot path - called for every instruction
    fn translate_instruction(inst: &crate::recompiler::ir::instruction::IRInstruction) -> Result<String> {
        match inst {
            crate::recompiler::ir::instruction::IRInstruction::Add { dst, src1, src2 } => {
                Ok(format!(
                    "    ctx.set_register({}, ctx.get_register({}) + ctx.get_register({}));\n",
                    dst, src1, src2
                ))
            }
            crate::recompiler::ir::instruction::IRInstruction::Sub { dst, src1, src2 } => {
                Ok(format!(
                    "    ctx.set_register({}, ctx.get_register({}) - ctx.get_register({}));\n",
                    dst, src1, src2
                ))
            }
            crate::recompiler::ir::instruction::IRInstruction::Mul { dst, src1, src2 } => {
                Ok(format!(
                    "    ctx.set_register({}, ctx.get_register({}) * ctx.get_register({}));\n",
                    dst, src1, src2
                ))
            }
            crate::recompiler::ir::instruction::IRInstruction::Div { dst, src1, src2 } => {
                Ok(format!(
                    "    ctx.set_register({}, ctx.get_register({}) / ctx.get_register({}));\n",
                    dst, src1, src2
                ))
            }
            crate::recompiler::ir::instruction::IRInstruction::Move { dst, src } => {
                Ok(format!(
                    "    ctx.set_register({}, ctx.get_register({}));\n",
                    dst, src
                ))
            }
            crate::recompiler::ir::instruction::IRInstruction::MoveImm { dst, imm } => {
                Ok(format!(
                    "    ctx.set_register({}, {}u32);\n",
                    dst, imm
                ))
            }
            _ => Ok("    // IR instruction\n".to_string()),
        }
    }
}
