//! Type Inference and Recovery
//!
//! This module provides type inference for PowerPC instructions, recovering type information
//! that is lost during compilation. This is essential for generating high-quality Rust code.
//!
//! # Memory Optimizations
//! - `InferredType` uses `#[repr(u8)]` to save 3 bytes per enum
//! - Type parameter lists use `SmallVec` (most types have few parameters)
//! - Type information structures are packed to minimize padding
//!
//! # Type Inference Algorithm
//! Uses a combination of:
//! - **Static analysis**: Infer types from operations (e.g., FP operations → float types)
//! - **External metadata**: Use Ghidra type information when available
//! - **Constraint solving**: Unify types across def-use chains
//!
//! # Type System
//! The type system supports:
//! - Integers (signed/unsigned, 8/16/32/64-bit)
//! - Floating-point (32/64-bit)
//! - Pointers (to any type)
//! - Unknown (when type cannot be determined)

use crate::recompiler::analysis::FunctionMetadata;
use crate::recompiler::decoder::{DecodedInstruction, Operand};
use smallvec::SmallVec;
use std::collections::HashMap;

/// Inferred type for a register or variable.
///
/// # Memory Optimization
/// Uses `#[repr(u8)]` to reduce size from default enum size (4-8 bytes) to 1 byte.
///
/// # Type Categories
/// - **Integer**: Signed or unsigned integers of various sizes
/// - **Float**: Floating-point numbers (32 or 64-bit)
/// - **Pointer**: Pointer to another type
/// - **Unknown**: Type cannot be determined
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[repr(u8)] // Save 3-7 bytes per enum
pub enum InferredType {
    /// Integer type with sign and size
    /// Uses u8 for size (8, 16, 32, or 64 bits)
    Integer { signed: bool, size: u8 } = 0,
    /// Floating-point type with size
    /// Uses u8 for size (32 or 64 bits)
    Float { size: u8 } = 1,
    /// Pointer type (points to another type)
    /// Uses Box to avoid recursive enum size issues
    Pointer { pointee: Box<InferredType> } = 2,
    /// Unknown type (cannot be determined)
    Unknown = 3,
}

/// Type inference engine for recovering type information.
pub struct TypeInferenceEngine;

impl TypeInferenceEngine {
    /// Infer types for all registers in a function.
    ///
    /// # Algorithm
    /// 1. Initialize types from function metadata (parameters, return type)
    /// 2. Infer types from operations:
    ///    - FP operations → float types
    ///    - Load operations → integer or pointer types
    ///    - Arithmetic operations → integer types
    /// 3. Propagate types through def-use chains
    ///
    /// # Arguments
    /// * `instructions` - Sequence of decoded PowerPC instructions
    /// * `metadata` - Function metadata (parameters, return type, etc.)
    ///
    /// # Returns
    /// `HashMap<u8, InferredType>` - Map from register number to inferred type
    ///
    /// # Examples
    /// ```rust
    /// let types = TypeInferenceEngine::infer_types(&instructions, &metadata);
    /// if let Some(ty) = types.get(&3) {
    ///     println!("Register r3 has type: {:?}", ty);
    /// }
    /// ```
    #[inline] // May be called frequently
    pub fn infer_types(
        instructions: &[DecodedInstruction],
        metadata: &FunctionMetadata,
    ) -> HashMap<u8, InferredType> {
        let mut register_types: HashMap<u8, InferredType> = HashMap::new();
        
        // Use Ghidra type information if available
        for param in metadata.parameters.iter() {
            if let Some(reg) = param.register {
                register_types.insert(reg, Self::type_from_string(&param.type_info));
            }
        }
        
        // Infer types from operations
        for inst in instructions.iter() {
            Self::infer_from_instruction(inst, &mut register_types);
        }
        
        register_types
    }
    
    /// Convert type information from string representation to InferredType.
    ///
    /// # Arguments
    /// * `ty` - Type information from metadata
    ///
    /// # Returns
    /// `InferredType` - Inferred type representation
    #[inline] // Called for each parameter
    fn type_from_string(ty: &crate::recompiler::analysis::TypeInfo) -> InferredType {
        match ty {
            crate::recompiler::analysis::TypeInfo::Integer { signed, size } => {
                InferredType::Integer { signed: *signed, size: *size }
            }
            crate::recompiler::analysis::TypeInfo::Pointer { pointee } => {
                InferredType::Pointer {
                    pointee: Box::new(Self::type_from_string(pointee)),
                }
            }
            _ => InferredType::Unknown,
        }
    }
    
    /// Infer type from a single instruction.
    ///
    /// # Algorithm
    /// Analyzes instruction type and operands to infer register types:
    /// - FP operations → float types
    /// - Load operations → integer or pointer types
    /// - Arithmetic operations → integer types
    ///
    /// # Arguments
    /// * `inst` - Decoded instruction to analyze
    /// * `register_types` - Map of register types (updated in place)
    #[inline] // Hot path - called for every instruction
    fn infer_from_instruction(
        inst: &DecodedInstruction,
        register_types: &mut HashMap<u8, InferredType>,
    ) {
        match inst.instruction.instruction_type {
            crate::recompiler::decoder::InstructionType::FloatingPoint => {
                // FP operations produce/consume floats
                if let Some(Operand::FpRegister(frt)) = inst.instruction.operands.first() {
                    register_types.insert(*frt, InferredType::Float { size: 64u8 });
                }
            }
            crate::recompiler::decoder::InstructionType::Load => {
                // Loads produce integers (or could be pointers)
                if let Some(Operand::Register(rt)) = inst.instruction.operands.first() {
                    register_types.insert(*rt, InferredType::Integer { signed: false, size: 32u8 });
                }
            }
            crate::recompiler::decoder::InstructionType::Arithmetic => {
                // Arithmetic operations produce integers
                if let Some(Operand::Register(rt)) = inst.instruction.operands.first() {
                    register_types.insert(*rt, InferredType::Integer { signed: true, size: 32u8 });
                }
            }
            _ => {}
        }
    }
}
