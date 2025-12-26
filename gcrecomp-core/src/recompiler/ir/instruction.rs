//! Intermediate Representation (IR) Instructions
//!
//! This module defines the intermediate representation used for code generation.
//! The IR is a simplified, architecture-agnostic representation of PowerPC instructions
//! that facilitates optimization and code generation.
//!
//! # Memory Optimizations
//! - `IRInstruction` uses `#[repr(u8)]` to save 3 bytes per enum variant
//! - `Address` and `Condition` use `#[repr(u8)]` for size optimization
//! - `IRFunction.parameters` uses `SmallVec<[u8; 8]>` (most functions have ≤8 parameters)
//! - `IRBasicBlock.successors` uses `SmallVec<[usize; 2]>` (most blocks have ≤2 successors)
//! - Block IDs use `u32` instead of `usize` to save space on 64-bit systems
//!
//! # IR Design
//! The IR is designed to be:
//! - **Simple**: Fewer instruction types than PowerPC ISA
//! - **Optimizable**: Easy to apply optimizations (dead code elimination, constant folding)
//! - **Target-agnostic**: Can be translated to any target architecture

use smallvec::SmallVec;

/// Intermediate representation instruction.
///
/// # Memory Optimization
/// Uses `#[repr(u8)]` to reduce size from default enum size (typically 4-8 bytes)
/// to 1 byte for the discriminant, saving 3-7 bytes per instruction.
///
/// # Instruction Categories
/// - **Arithmetic**: Integer arithmetic operations
/// - **Memory**: Load and store operations
/// - **Control Flow**: Branches, calls, and returns
/// - **Floating Point**: FP arithmetic operations
/// - **Move**: Register moves and immediate loads
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)] // Save 3-7 bytes per enum (default size -> 1 byte)
pub enum IRInstruction {
    /// Add: `dst = src1 + src2`
    /// Uses u8 for register indices (PowerPC has 32 GPRs, fits in 5 bits)
    Add { dst: u8, src1: u8, src2: u8 } = 0,
    /// Subtract: `dst = src1 - src2`
    Sub { dst: u8, src1: u8, src2: u8 } = 1,
    /// Multiply: `dst = src1 * src2`
    Mul { dst: u8, src1: u8, src2: u8 } = 2,
    /// Divide: `dst = src1 / src2`
    Div { dst: u8, src1: u8, src2: u8 } = 3,
    /// Bitwise AND: `dst = src1 & src2`
    And { dst: u8, src1: u8, src2: u8 } = 4,
    /// Bitwise OR: `dst = src1 | src2`
    Or { dst: u8, src1: u8, src2: u8 } = 5,
    /// Bitwise XOR: `dst = src1 ^ src2`
    Xor { dst: u8, src1: u8, src2: u8 } = 6,
    
    /// Load from memory: `dst = *addr`
    /// Address can be register-relative or constant
    Load { dst: u8, addr: Address } = 7,
    /// Store to memory: `*addr = src`
    Store { src: u8, addr: Address } = 8,
    
    /// Unconditional branch to target address
    Branch { target: u32 } = 9,
    /// Conditional branch: branch to target if condition is true
    BranchCond { cond: Condition, target: u32 } = 10,
    /// Function call: call function at target address
    Call { target: u32 } = 11,
    /// Return from function
    Return = 12,
    
    /// Floating-point add: `dst = src1 + src2`
    FAdd { dst: u8, src1: u8, src2: u8 } = 13,
    /// Floating-point subtract: `dst = src1 - src2`
    FSub { dst: u8, src1: u8, src2: u8 } = 14,
    /// Floating-point multiply: `dst = src1 * src2`
    FMul { dst: u8, src1: u8, src2: u8 } = 15,
    /// Floating-point divide: `dst = src1 / src2`
    FDiv { dst: u8, src1: u8, src2: u8 } = 16,
    
    /// Move register: `dst = src`
    Move { dst: u8, src: u8 } = 17,
    /// Move immediate: `dst = imm`
    MoveImm { dst: u8, imm: u32 } = 18,
}

/// Memory address representation.
///
/// # Memory Optimization
/// Uses `#[repr(u8)]` to reduce size from default enum size to 1 byte.
///
/// # Address Modes
/// - **Register-relative**: Base register + signed offset (common for stack/local variables)
/// - **Constant**: Absolute address (for global variables, function addresses)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)] // Save 3-7 bytes per enum
pub enum Address {
    /// Register-relative addressing: `base + offset`
    /// Base register (u8): PowerPC has 32 GPRs, fits in 5 bits
    /// Offset (i32): Signed 32-bit offset for stack/local variable access
    Register { base: u8, offset: i32 } = 0,
    /// Constant address: absolute 32-bit address
    Constant(u32) = 1,
}

/// Branch condition for conditional branches.
///
/// # Memory Optimization
/// Uses `#[repr(u8)]` to reduce size from default enum size to 1 byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)] // Save 3-7 bytes per enum
pub enum Condition {
    /// Equal: `a == b`
    Equal = 0,
    /// Not equal: `a != b`
    NotEqual = 1,
    /// Less than: `a < b` (signed)
    LessThan = 2,
    /// Greater than: `a > b` (signed)
    GreaterThan = 3,
    /// Less than or equal: `a <= b` (signed)
    LessThanOrEqual = 4,
    /// Greater than or equal: `a >= b` (signed)
    GreaterThanOrEqual = 5,
}

/// Intermediate representation of a function.
///
/// # Memory Optimization
/// - `parameters`: Uses `SmallVec<[u8; 8]>` - most functions have ≤8 parameters (PowerPC calling convention)
/// - `basic_blocks`: Uses `Vec` (functions can have many blocks, heap allocation is appropriate)
/// - `return_register`: Uses `Option<u8>` (1 byte + 1 byte discriminant = 2 bytes total)
///
/// # Function Structure
/// A function consists of:
/// - Name and address (for linking and debugging)
/// - Parameter list (register indices for PowerPC calling convention: r3-r10)
/// - Return register (if function returns a value, typically r3)
/// - Basic blocks (control flow graph nodes)
#[derive(Debug, Clone)]
#[repr(C)] // Ensure C-compatible layout
pub struct IRFunction {
    /// Function name (for debugging and linking)
    pub name: String,
    /// Function entry address in original binary
    pub address: u32,
    /// Function parameters (register indices)
    /// Uses SmallVec with inline capacity for 8 parameters (PowerPC calling convention)
    /// Most functions have ≤8 parameters, avoiding heap allocation
    pub parameters: SmallVec<[u8; 8]>,
    /// Return register (if function returns a value)
    /// Uses Option<u8> - 1 byte for register + 1 byte discriminant = 2 bytes total
    pub return_register: Option<u8>,
    /// Basic blocks in this function (control flow graph)
    /// Uses Vec because functions can have many blocks, heap allocation is appropriate
    pub basic_blocks: Vec<IRBasicBlock>,
}

/// Intermediate representation of a basic block.
///
/// # Memory Optimization
/// - `id`: Uses `u32` instead of `usize` to save 4 bytes on 64-bit systems
/// - `instructions`: Uses `Vec` (blocks can have many instructions)
/// - `successors`: Uses `SmallVec<[u32; 2]>` - most blocks have ≤2 successors (if-then-else, loop)
///
/// # Basic Block Properties
/// A basic block is a sequence of instructions with:
/// - Single entry point (first instruction)
/// - Single exit point (last instruction is a branch/return)
/// - No internal branches (linear execution)
#[derive(Debug, Clone)]
#[repr(C)] // Ensure C-compatible layout
pub struct IRBasicBlock {
    /// Basic block identifier (unique within function)
    /// Uses u32 instead of usize to save 4 bytes on 64-bit systems
    /// Maximum 4 billion blocks per function is more than sufficient
    pub id: u32,
    /// Instructions in this basic block (in execution order)
    /// Uses Vec because blocks can have many instructions
    pub instructions: Vec<IRInstruction>,
    /// Successor basic block IDs (targets of branches)
    /// Uses SmallVec with inline capacity for 2 successors (most blocks have ≤2)
    /// Typical cases: if-then-else (2 successors), loop (1-2 successors), fall-through (1 successor)
    pub successors: SmallVec<[u32; 2]>,
}
