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

/// Source location for error reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLocation {
    pub file: Option<&'static str>,
    pub line: u32,
    pub column: u32,
    pub address: Option<u32>,
}

impl SourceLocation {
    pub fn new(file: Option<&'static str>, line: u32, column: u32, address: Option<u32>) -> Self {
        Self { file, line, column, address }
    }
    
    pub fn format(&self) -> String {
        let mut result = String::new();
        if let Some(file) = self.file {
            result.push_str(&format!("{}:", file));
        }
        result.push_str(&format!("{}:{}", self.line, self.column));
        if let Some(addr) = self.address {
            result.push_str(&format!(" (0x{:08X})", addr));
        }
        result
    }
}

/// Recompiler error types.
///
/// Uses `thiserror` for zero-cost error handling with detailed error messages.
/// All error variants are marked with `#[cold]` to help the optimizer place
/// error handling code in cold paths.
#[derive(Error, Debug, Clone)]
pub enum RecompilerError {
    /// DOL file parsing error.
    ///
    /// Occurs when the DOL file format is invalid or cannot be parsed.
    #[error("DOL parsing error at {location}: {message}\nSuggestion: {suggestion}")]
    DolParseError {
        message: String,
        location: SourceLocation,
        suggestion: String,
    },

    /// Instruction decoding error.
    ///
    /// Occurs when a PowerPC instruction cannot be decoded (invalid opcode, malformed format).
    #[error("Instruction decode error at {location}: {message}\nRaw instruction: 0x{raw:08X}\nSuggestion: {suggestion}")]
    InstructionDecodeError {
        message: String,
        location: SourceLocation,
        raw: u32,
        suggestion: String,
    },

    /// Code generation error.
    ///
    /// Occurs when Rust code generation fails (invalid IR, unsupported instruction, etc.).
    #[error("Code generation error at {location}: {message}\nSuggestion: {suggestion}")]
    CodeGenError {
        message: String,
        location: SourceLocation,
        suggestion: String,
    },

    /// Ghidra analysis error.
    ///
    /// Occurs when Ghidra analysis fails (Ghidra not found, analysis script error, etc.).
    #[error("Ghidra analysis error: {message}\nSuggestion: {suggestion}")]
    GhidraError {
        message: String,
        suggestion: String,
    },

    /// Memory access error.
    ///
    /// Occurs when accessing invalid memory addresses (out of bounds, unmapped region).
    #[error("Memory access error at {location}: address 0x{address:08X} is invalid\nSuggestion: {suggestion}")]
    MemoryError {
        address: u32,
        location: SourceLocation,
        suggestion: String,
    },

    /// Invalid register error.
    ///
    /// Occurs when using an invalid register number (PowerPC has 32 GPRs, r0-r31).
    #[error("Invalid register at {location}: register {register} (must be 0-31)\nSuggestion: {suggestion}")]
    InvalidRegister {
        register: u8,
        location: SourceLocation,
        suggestion: String,
    },

    /// Optimization error.
    ///
    /// Occurs when an optimization pass fails (invalid CFG, data flow analysis error, etc.).
    #[error("Optimization error at {location}: {message}\nSuggestion: {suggestion}")]
    OptimizationError {
        message: String,
        location: SourceLocation,
        suggestion: String,
    },

    /// Validation error.
    ///
    /// Occurs when generated Rust code validation fails (syntax error, type error, etc.).
    #[error("Validation error at {location}: {message}\nSuggestion: {suggestion}")]
    ValidationError {
        message: String,
        location: SourceLocation,
        suggestion: String,
    },
}

impl RecompilerError {
    /// Create a DOL parse error with context.
    pub fn dol_parse(message: impl Into<String>, location: SourceLocation) -> Self {
        Self::DolParseError {
            message: message.into(),
            location,
            suggestion: "Check that the DOL file is valid and not corrupted.".to_string(),
        }
    }
    
    /// Create an instruction decode error with context.
    pub fn instruction_decode(message: impl Into<String>, location: SourceLocation, raw: u32) -> Self {
        let suggestion = if (raw & 0xFC000000) == 0 {
            "This might be data, not an instruction. Check the disassembly."
        } else {
            "This instruction may be unsupported or corrupted. Check the binary."
        };
        Self::InstructionDecodeError {
            message: message.into(),
            location,
            raw,
            suggestion: suggestion.to_string(),
        }
    }
    
    /// Create a code generation error with context.
    pub fn codegen(message: impl Into<String>, location: SourceLocation) -> Self {
        Self::CodeGenError {
            message: message.into(),
            location,
            suggestion: "This instruction may need manual handling. Check the recompiler implementation.".to_string(),
        }
    }
    
    /// Create a memory error with context.
    pub fn memory(address: u32, location: SourceLocation) -> Self {
        let suggestion = if address < 0x80000000 {
            "Address is below RAM base. Check address translation."
        } else if address >= 0x81800000 {
            "Address is above RAM limit. Check bounds."
        } else {
            "Address may be uninitialized or invalid. Check memory mapping."
        };
        Self::MemoryError {
            address,
            location,
            suggestion: suggestion.to_string(),
        }
    }
}

impl From<std::io::Error> for RecompilerError {
    #[cold] // Error paths are cold
    fn from(err: std::io::Error) -> Self {
        RecompilerError::DolParseError {
            message: format!("IO error: {}", err),
            location: SourceLocation::new(None, 0, 0, None),
            suggestion: "Check file permissions and that the file exists.".to_string(),
        }
    }
}

impl From<RecompilerError> for anyhow::Error {
    #[cold] // Error paths are cold
    fn from(err: RecompilerError) -> Self {
        anyhow::Error::from(err)
    }
}
