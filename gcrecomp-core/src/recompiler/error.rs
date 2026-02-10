//! Enhanced Error Handling
//!
//! This module provides comprehensive error types for the recompiler using `thiserror`.
//! All errors are zero-cost (no heap allocation) and provide detailed error messages.
//!
//! # Error Categories
//! - **Parsing errors**: DOL file parsing, instruction decoding
//! - **Analysis errors**: Control flow, data flow, type inference
//! - **Code generation errors**: Rust code generation failures
//! - **Memory errors**: Invalid memory accesses
//! - **Validation errors**: Generated code validation failures

use thiserror::Error;

/// Recompiler error types.
///
/// Uses `thiserror` for zero-cost error handling with detailed error messages.
/// All error variants are marked with `#[cold]` to help the optimizer place
/// error handling code in cold paths.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum RecompilerError {
    /// DOL file parsing error.
    ///
    /// Occurs when the DOL file format is invalid or cannot be parsed.
    #[error("DOL parsing error: {0}")]
    DolParseError(String),

    /// Instruction decoding error.
    ///
    /// Occurs when a PowerPC instruction cannot be decoded (invalid opcode, malformed format).
    #[error("Instruction decode error: {0}")]
    InstructionDecodeError(String),

    /// Code generation error.
    ///
    /// Occurs when Rust code generation fails (invalid IR, unsupported instruction, etc.).
    #[error("Code generation error: {0}")]
    CodeGenError(String),

    /// Ghidra analysis error.
    ///
    /// Occurs when Ghidra analysis fails (Ghidra not found, analysis script error, etc.).
    #[error("Ghidra analysis error: {0}")]
    GhidraError(String),

    /// Memory access error.
    ///
    /// Occurs when accessing invalid memory addresses (out of bounds, unmapped region).
    #[error("Memory access error: address 0x{0:08X}")]
    MemoryError(u32),

    /// Invalid register error.
    ///
    /// Occurs when using an invalid register number (PowerPC has 32 GPRs, r0-r31).
    #[error("Invalid register: {0} (must be 0-31)")]
    InvalidRegister(u8),

    /// Optimization error.
    ///
    /// Occurs when an optimization pass fails (invalid CFG, data flow analysis error, etc.).
    #[error("Optimization error: {0}")]
    OptimizationError(String),

    /// Validation error.
    ///
    /// Occurs when generated Rust code validation fails (syntax error, type error, etc.).
    #[error("Validation error: {0}")]
    ValidationError(String),
}

impl From<std::io::Error> for RecompilerError {
    #[cold] // Error paths are cold
    fn from(err: std::io::Error) -> Self {
        RecompilerError::DolParseError(format!("IO error: {}", err))
    }
}
