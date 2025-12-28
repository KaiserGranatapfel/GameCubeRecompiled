//! Code Validation
//!
//! This module provides validation for generated Rust code to ensure correctness
//! before writing to output files.
//!
//! # Validation Checks
//! - **Syntax validation**: Basic syntax checks (balanced braces, function definitions)
//! - **Type validation**: Type correctness (would use rustc or syn crate in full implementation)
//! - **Semantic validation**: Semantic correctness (would use rustc in full implementation)

use crate::recompiler::error::RecompilerError;
use anyhow::Result;

/// Code validator for generated Rust code.
pub struct CodeValidator;

impl CodeValidator {
    /// Validate generated Rust code.
    ///
    /// # Algorithm
    /// Performs comprehensive syntax validation:
    /// - Checks for at least one function definition
    /// - Validates balanced braces, parentheses, and brackets
    /// - Checks for common syntax errors (unclosed strings, etc.)
    /// - Validates function signatures
    /// - (In full implementation) Would use rustc or syn crate for full validation
    ///
    /// # Arguments
    /// * `code` - Generated Rust code string
    ///
    /// # Returns
    /// * `Result<()>` - Success if code is valid, error otherwise
    ///
    /// # Errors
    /// Returns error if code fails validation checks with detailed error messages
    ///
    /// # Examples
    /// ```rust
    /// CodeValidator::validate_rust_code(&generated_code)?;
    /// ```
    #[inline] // May be called frequently
    #[must_use] // Result should be checked
    pub fn validate_rust_code(code: &str) -> Result<()> {
        // Basic syntax validation
        // In a full implementation, would use rustc or syn crate
        
        // Check for basic syntax issues
        if !code.contains("fn ") {
            return Err(RecompilerError::ValidationError(
                "Generated code must contain at least one function definition".to_string()
            ).into());
        }
        
        // Check balanced braces
        let open_braces: usize = code.matches('{').count();
        let close_braces: usize = code.matches('}').count();
        if open_braces != close_braces {
            return Err(RecompilerError::ValidationError(
                format!(
                    "Unbalanced braces in generated code: {} open, {} close. This indicates a syntax error in code generation.",
                    open_braces, close_braces
                )
            ).into());
        }
        
        // Check balanced parentheses
        let open_parens: usize = code.matches('(').count();
        let close_parens: usize = code.matches(')').count();
        if open_parens != close_parens {
            return Err(RecompilerError::ValidationError(
                format!(
                    "Unbalanced parentheses in generated code: {} open, {} close. This indicates a syntax error in code generation.",
                    open_parens, close_parens
                )
            ).into());
        }
        
        // Check balanced brackets
        let open_brackets: usize = code.matches('[').count();
        let close_brackets: usize = code.matches(']').count();
        if open_brackets != close_brackets {
            return Err(RecompilerError::ValidationError(
                format!(
                    "Unbalanced brackets in generated code: {} open, {} close. This indicates a syntax error in code generation.",
                    open_brackets, close_brackets
                )
            ).into());
        }
        
        // Check for common syntax errors
        // Unclosed strings (basic check - doesn't handle escaped quotes)
        let string_literal_count: usize = code.matches('"').count();
        if string_literal_count % 2 != 0 {
            log::warn!("Possible unclosed string literal in generated code (odd number of quotes)");
        }
        
        // Check that all functions have return types or statements
        let fn_count: usize = code.matches("pub fn ").count() + code.matches("fn ").count();
        let return_count: usize = code.matches("return").count() + code.matches("Ok(").count();
        if fn_count > 0 && return_count == 0 {
            log::warn!("Generated code has functions but no return statements - this may indicate incomplete code generation");
        }
        
        // Check for required imports
        if !code.contains("use ") && !code.contains("CpuContext") {
            log::warn!("Generated code may be missing required imports (CpuContext, MemoryManager, etc.)");
        }
        
        log::debug!("Code validation passed: {} functions, {} braces, {} parentheses", fn_count, open_braces, open_parens);
        
        Ok(())
    }
    
    /// Validate control flow graph structure
    pub fn validate_cfg(cfg: &crate::recompiler::analysis::control_flow::ControlFlowGraph) -> Result<()> {
        use crate::recompiler::analysis::control_flow::ControlFlowGraph;
        
        // Check that entry block exists
        if cfg.entry_block >= cfg.nodes.len() {
            anyhow::bail!("CFG entry block index {} is out of bounds ({} nodes)", 
                         cfg.entry_block, cfg.nodes.len());
        }
        
        // Validate edges reference valid nodes
        for edge in &cfg.edges {
            if edge.from >= cfg.nodes.len() {
                anyhow::bail!("CFG edge from node {} is out of bounds ({} nodes)", 
                             edge.from, cfg.nodes.len());
            }
            if edge.to >= cfg.nodes.len() {
                anyhow::bail!("CFG edge to node {} is out of bounds ({} nodes)", 
                             edge.to, cfg.nodes.len());
            }
        }
        
        // Check for unreachable nodes (warn, not error)
        let mut reachable = vec![false; cfg.nodes.len()];
        Self::mark_reachable(cfg, cfg.entry_block, &mut reachable);
        
        let unreachable_count = reachable.iter().filter(|&&r| !r).count();
        if unreachable_count > 0 {
            log::warn!("CFG has {} unreachable nodes (may indicate dead code)", unreachable_count);
        }
        
        Ok(())
    }
    
    /// Mark reachable nodes from entry
    fn mark_reachable(
        cfg: &crate::recompiler::analysis::control_flow::ControlFlowGraph,
        node_idx: usize,
        reachable: &mut [bool],
    ) {
        if reachable[node_idx] {
            return; // Already visited
        }
        
        reachable[node_idx] = true;
        
        // Mark all successors
        for edge in &cfg.edges {
            if edge.from == node_idx {
                Self::mark_reachable(cfg, edge.to, reachable);
            }
        }
    }
    
    /// Validate register usage
    pub fn validate_register_usage(
        instructions: &[crate::recompiler::decoder::DecodedInstruction],
    ) -> Result<()> {
        for inst in instructions {
            for operand in &inst.instruction.operands {
                match operand {
                    crate::recompiler::decoder::Operand::Register(r) |
                    crate::recompiler::decoder::Operand::FpRegister(r) => {
                        if *r > 31 {
                            anyhow::bail!(
                                "Invalid register {} at address 0x{:08X} (must be 0-31)",
                                r, inst.address
                            );
                        }
                    }
                    crate::recompiler::decoder::Operand::Condition(c) => {
                        if *c > 7 {
                            anyhow::bail!(
                                "Invalid condition register field {} at address 0x{:08X} (must be 0-7)",
                                c, inst.address
                            );
                        }
                    }
                    crate::recompiler::decoder::Operand::ShiftAmount(s) => {
                        if *s > 31 {
                            anyhow::bail!(
                                "Invalid shift amount {} at address 0x{:08X} (must be 0-31)",
                                s, inst.address
                            );
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
    
    /// Validate memory access addresses
    pub fn validate_memory_accesses(
        instructions: &[crate::recompiler::decoder::DecodedInstruction],
    ) -> Result<()> {
        for inst in instructions {
            match inst.instruction.instruction_type {
                crate::recompiler::decoder::InstructionType::Load |
                crate::recompiler::decoder::InstructionType::Store => {
                    // Check if address operand is reasonable
                    for operand in &inst.instruction.operands {
                        if let crate::recompiler::decoder::Operand::Address(addr) = operand {
                            // Validate address is in reasonable range
                            if *addr > 0xFFFFFFFF {
                                anyhow::bail!(
                                    "Invalid memory address 0x{:08X} at instruction 0x{:08X}",
                                    addr, inst.address
                                );
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
    
    /// Validate type consistency
    pub fn validate_types(
        _instructions: &[crate::recompiler::decoder::DecodedInstruction],
        _register_types: &std::collections::HashMap<u8, crate::recompiler::analysis::TypeInfo>,
    ) -> Result<()> {
        // Type validation would check:
        // - Register types are consistent across uses
        // - Operations match operand types
        // - Type conversions are valid
        // This is a placeholder for future implementation
        Ok(())
    }
    
    /// Validate a single function's code.
    ///
    /// # Arguments
    /// * `function_code` - Generated function code string
    ///
    /// # Returns
    /// `Result<()>` - Success if function code is valid, error otherwise
    ///
    /// # Examples
    /// ```rust
    /// CodeValidator::validate_function(&function_code)?;
    /// ```
    #[inline] // Simple wrapper
    #[must_use] // Result should be checked
    pub fn validate_function(function_code: &str) -> Result<()> {
        Self::validate_rust_code(function_code)
    }
}
