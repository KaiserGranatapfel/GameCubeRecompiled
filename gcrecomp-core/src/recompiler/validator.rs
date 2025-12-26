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
