//! PowerPC Instruction Decoder
//!
//! This module provides comprehensive decoding of PowerPC instructions from 32-bit words.
//! It extracts opcodes, instruction types, and operands, with aggressive memory optimization
//! to minimize memory footprint (saving even single bits where possible).
//!
//! # Memory Optimizations
//! - `InstructionType` uses `#[repr(u8)]` to save 3 bytes per enum (4 bytes -> 1 byte)
//! - `Operand` uses `SmallVec` for most instructions (≤4 operands) to avoid heap allocation
//! - Structs are packed to minimize padding
//! - Register indices use `u8` (PowerPC has 32 GPRs, fits in 5 bits)
//!
//! # Decoding Algorithm
//! The decoder uses a two-stage approach:
//! 1. Extract primary opcode (bits 26-31)
//! 2. For opcode 31 (extended), decode secondary opcode (bits 1-10)
//!
//! Most PowerPC instructions have 3-4 operands, making `SmallVec<[Operand; 4]>` optimal.

use anyhow::{Context, Result};
use smallvec::SmallVec;

/// PowerPC instruction representation with optimized memory layout.
///
/// # Memory Layout
/// - `opcode`: 6 bits (bits 26-31 of instruction word)
/// - `instruction_type`: 1 byte (enum with `#[repr(u8)]`)
/// - `operands`: SmallVec with inline capacity for 4 operands (most instructions have ≤4)
#[derive(Debug, Clone)]
#[repr(C)] // Ensure C-compatible layout for potential FFI
pub struct Instruction {
    /// Primary opcode (6 bits, stored as u32 for alignment but only uses 6 bits)
    pub opcode: u32,
    /// Instruction type category (1 byte enum)
    pub instruction_type: InstructionType,
    /// Instruction operands (register, immediate, address, etc.)
    /// Uses SmallVec to avoid heap allocation for common case (≤4 operands)
    pub operands: SmallVec<[Operand; 4]>,
}

/// PowerPC instruction type categories.
///
/// # Memory Optimization
/// Uses `#[repr(u8)]` to reduce size from 4 bytes (default enum size) to 1 byte,
/// saving 3 bytes per instruction. This is safe because we have <256 variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)] // Save 3 bytes per enum (4 bytes -> 1 byte)
pub enum InstructionType {
    /// Arithmetic operations (add, sub, mul, div, and, or, xor, etc.)
    Arithmetic = 0,
    /// Branch instructions (b, bl, bc, bclr, etc.)
    Branch = 1,
    /// Load instructions (lwz, lbz, lhz, etc.)
    Load = 2,
    /// Store instructions (stw, stb, sth, etc.)
    Store = 3,
    /// Compare instructions (cmpw, cmplw, cmpwi, etc.)
    Compare = 4,
    /// Move instructions (mr, mflr, mtlr, etc.)
    Move = 5,
    /// System instructions (sync, isync, cache control, etc.)
    System = 6,
    /// Floating-point operations (fadd, fsub, fmul, fdiv, etc.)
    FloatingPoint = 7,
    /// Condition register operations (mfcr, mtcr, crand, cror, etc.)
    ConditionRegister = 8,
    /// Shift operations (slw, srw, sraw, etc.)
    Shift = 9,
    /// Rotate operations (rlwinm, rlwnm, etc.)
    Rotate = 10,
    /// Unknown or unimplemented instruction
    Unknown = 11,
}

/// PowerPC instruction operand representation.
///
/// # Memory Optimization
/// Uses appropriate integer sizes:
/// - Register indices: `u8` (PowerPC has 32 GPRs, fits in 5 bits)
/// - Immediate values: `i16` for 16-bit immediates, `i32` for 32-bit
/// - Addresses: `u32` (full 32-bit address space)
/// - Special registers: `u16` (SPR encoding uses 10 bits)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operand {
    /// General-purpose register (GPR) - 32 registers (r0-r31), stored as u8
    Register(u8),
    /// Floating-point register (FPR) - 32 registers (f0-f31), stored as u8
    FpRegister(u8),
    /// 16-bit signed immediate value (SI field in instruction)
    Immediate(i16),
    /// 32-bit signed immediate value (used for branch targets, etc.)
    Immediate32(i32),
    /// 32-bit address (absolute or relative)
    Address(u32),
    /// Condition register field (4 bits, stored as u8)
    Condition(u8),
    /// Special-purpose register (SPR) - 10-bit encoding, stored as u16
    SpecialRegister(u16),
    /// Shift amount (5 bits, stored as u8)
    ShiftAmount(u8),
    /// Rotate mask (32 bits, stored as u32)
    Mask(u32),
}

/// Decoded PowerPC instruction with raw word and address for reference.
///
/// # Memory Layout
/// Packed to minimize padding:
/// - `instruction`: Contains opcode, type, and operands
/// - `raw`: Original 32-bit instruction word
/// - `address`: Memory address where this instruction is located (for function mapping)
#[derive(Debug, Clone)]
pub struct DecodedInstruction {
    /// Decoded instruction structure
    pub instruction: Instruction,
    /// Raw 32-bit instruction word (for debugging and re-encoding)
    pub raw: u32,
    /// Memory address where this instruction is located in the original binary
    /// Used for mapping instructions to functions and control flow analysis
    pub address: u32,
}

impl Instruction {
    /// Decode a 32-bit PowerPC instruction word into a structured representation.
    ///
    /// # Algorithm
    /// 1. Extract primary opcode (bits 26-31)
    /// 2. For opcode 31 (extended), extract secondary opcode (bits 1-10)
    /// 3. Extract operands based on instruction format
    /// 4. Return decoded instruction with type and operands
    ///
    /// # Arguments
    /// * `word` - 32-bit instruction word in big-endian format
    /// * `address` - Memory address where this instruction is located (for function mapping)
    ///
    /// # Returns
    /// `Result<DecodedInstruction>` - Decoded instruction or error if invalid
    ///
    /// # Errors
    /// Returns error if instruction cannot be decoded (invalid opcode, malformed format)
    ///
    /// # Examples
    /// ```rust
    /// // Decode an addi instruction: addi r3, r4, 42
    /// let word: u32 = 0x3864002A; // addi r3, r4, 42
    /// let address: u32 = 0x80000000;
    /// let decoded = Instruction::decode(word, address)?;
    /// assert_eq!(decoded.instruction.instruction_type, InstructionType::Arithmetic);
    /// assert_eq!(decoded.address, address);
    /// ```
    #[inline] // Hot path - called for every instruction
    pub fn decode(word: u32, address: u32) -> Result<DecodedInstruction> {
        // Check for invalid instruction patterns (likely data, not code)
        if Self::is_likely_data(word) {
            log::debug!("Instruction at 0x{:08X} (0x{:08X}) may be data, not code", address, word);
            // Still decode it, but mark as potentially invalid
        }
        
        // Extract primary opcode (bits 26-31)
        let opcode: u32 = (word >> 26) & 0x3F;

        // Decode instruction type and operands based on opcode
        let (instruction_type, operands): (InstructionType, SmallVec<[Operand; 4]>) = match opcode {
            // Opcode 31: Extended opcodes (arithmetic, logical, shifts, etc.)
            // Secondary opcode is in bits 1-10
            31 => Self::decode_extended(word)?,

            // Opcode 14: Add immediate (addi)
            // Format: addi RT, RA, SI
            // RT = bits 21-25, RA = bits 16-20, SI = bits 0-15 (sign-extended)
            14 => {
                let rt: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let si: i16 = (word & 0xFFFF) as i16; // Sign-extend handled by cast
                (
                    InstructionType::Arithmetic,
                    SmallVec::from_slice(&[
                        Operand::Register(rt),
                        Operand::Register(ra),
                        Operand::Immediate(si),
                    ]),
                )
            }

            // Opcode 15: Subtract from immediate (subfic)
            // Format: subfic RT, RA, SI
            15 => {
                let rt: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let si: i16 = (word & 0xFFFF) as i16;
                (
                    InstructionType::Arithmetic,
                    SmallVec::from_slice(&[
                        Operand::Register(rt),
                        Operand::Register(ra),
                        Operand::Immediate(si),
                    ]),
                )
            }

            // Opcode 32: Load word and zero (lwz)
            // Format: lwz RT, D(RA)
            // RT = bits 21-25, RA = bits 16-20, D = bits 0-15 (sign-extended offset)
            32 => {
                let rt: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let d: i16 = (word & 0xFFFF) as i16;
                (
                    InstructionType::Load,
                    SmallVec::from_slice(&[
                        Operand::Register(rt),
                        Operand::Register(ra),
                        Operand::Immediate(d),
                    ]),
                )
            }

            // Opcode 36: Store word (stw)
            // Format: stw RS, D(RA)
            36 => {
                let rs: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let d: i16 = (word & 0xFFFF) as i16;
                (
                    InstructionType::Store,
                    SmallVec::from_slice(&[
                        Operand::Register(rs),
                        Operand::Register(ra),
                        Operand::Immediate(d),
                    ]),
                )
            }

            // Opcode 18: Branch (b, ba, bl, bla)
            // Format: b LI, AA, LK
            // LI = bits 0-23 (24-bit signed offset, aligned to 4 bytes)
            // AA = bit 1 (absolute address flag)
            // LK = bit 0 (link flag - save return address)
            18 => {
                let li: i32 = ((word & 0x3FFFFFC) as i32) >> 2; // Sign-extend and align
                let aa: u8 = ((word >> 1) & 1) as u8;
                let lk: u8 = (word & 1) as u8;
                (
                    InstructionType::Branch,
                    SmallVec::from_slice(&[
                        Operand::Immediate32(li),
                        Operand::Immediate(aa as i16),
                        Operand::Immediate(lk as i16),
                    ]),
                )
            }

            // Opcode 16: Branch conditional (bc, bca, bcl, bcla)
            // Format: bc BO, BI, BD, AA, LK
            // BO = bits 21-25 (branch options)
            // BI = bits 16-20 (condition register bit)
            // BD = bits 2-15 (14-bit signed branch displacement)
            // AA = bit 1 (absolute address flag)
            // LK = bit 0 (link flag)
            16 => {
                let bo: u8 = ((word >> 21) & 0x1F) as u8;
                let bi: u8 = ((word >> 16) & 0x1F) as u8;
                let bd: i16 = ((word & 0xFFFC) as i16) >> 2; // Sign-extend and align
                let aa: u8 = ((word >> 1) & 1) as u8;
                let lk: u8 = (word & 1) as u8;
                (
                    InstructionType::Branch,
                    SmallVec::from_slice(&[
                        Operand::Condition(bo),
                        Operand::Condition(bi),
                        Operand::Immediate32(bd as i32),
                        Operand::Immediate(aa as i16),
                        Operand::Immediate(lk as i16),
                    ]),
                )
            }

            // Opcode 11: Compare word immediate (cmpwi)
            // Format: cmpwi BF, RA, SI
            // BF = bits 23-25 (condition register field)
            // RA = bits 16-20
            // SI = bits 0-15 (sign-extended)
            11 => {
                let bf: u8 = ((word >> 23) & 0x7) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let si: i16 = (word & 0xFFFF) as i16;
                (
                    InstructionType::Compare,
                    SmallVec::from_slice(&[
                        Operand::Condition(bf),
                        Operand::Register(ra),
                        Operand::Immediate(si),
                    ]),
                )
            }

            // Opcode 10: Compare logical word immediate (cmplwi)
            // Format: cmplwi BF, RA, UI
            // UI = bits 0-15 (unsigned immediate)
            10 => {
                let bf: u8 = ((word >> 23) & 0x7) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let ui: u16 = (word & 0xFFFF) as u16;
                (
                    InstructionType::Compare,
                    SmallVec::from_slice(&[
                        Operand::Condition(bf),
                        Operand::Register(ra),
                        Operand::Immediate(ui as i16), // Store as i16 for consistency
                    ]),
                )
            }

            // Opcode 28: AND immediate (andi.)
            // Format: andi. RT, RA, UI
            28 => {
                let rs: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let ui: u16 = (word & 0xFFFF) as u16;
                (
                    InstructionType::Arithmetic,
                    SmallVec::from_slice(&[
                        Operand::Register(rs),
                        Operand::Register(ra),
                        Operand::Immediate(ui as i16),
                    ]),
                )
            }

            // Opcode 24: OR immediate (ori)
            // Format: ori RT, RA, UI
            24 => {
                let rs: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let ui: u16 = (word & 0xFFFF) as u16;
                (
                    InstructionType::Arithmetic,
                    SmallVec::from_slice(&[
                        Operand::Register(rs),
                        Operand::Register(ra),
                        Operand::Immediate(ui as i16),
                    ]),
                )
            }

            // Opcode 26: XOR immediate (xori)
            // Format: xori RT, RA, UI
            26 => {
                let rs: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let ui: u16 = (word & 0xFFFF) as u16;
                (
                    InstructionType::Arithmetic,
                    SmallVec::from_slice(&[
                        Operand::Register(rs),
                        Operand::Register(ra),
                        Operand::Immediate(ui as i16),
                    ]),
                )
            }

            // Opcode 34: Load byte and zero (lbz)
            // Format: lbz RT, D(RA)
            34 => {
                let rt: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let d: i16 = (word & 0xFFFF) as i16;
                (
                    InstructionType::Load,
                    SmallVec::from_slice(&[
                        Operand::Register(rt),
                        Operand::Register(ra),
                        Operand::Immediate(d),
                    ]),
                )
            }

            // Opcode 40: Load halfword and zero (lhz)
            // Format: lhz RT, D(RA)
            40 => {
                let rt: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let d: i16 = (word & 0xFFFF) as i16;
                (
                    InstructionType::Load,
                    SmallVec::from_slice(&[
                        Operand::Register(rt),
                        Operand::Register(ra),
                        Operand::Immediate(d),
                    ]),
                )
            }

            // Opcode 42: Load halfword algebraic (lha)
            // Format: lha RT, D(RA)
            42 => {
                let rt: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let d: i16 = (word & 0xFFFF) as i16;
                (
                    InstructionType::Load,
                    SmallVec::from_slice(&[
                        Operand::Register(rt),
                        Operand::Register(ra),
                        Operand::Immediate(d),
                    ]),
                )
            }

            // Opcode 38: Store byte (stb)
            // Format: stb RS, D(RA)
            38 => {
                let rs: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let d: i16 = (word & 0xFFFF) as i16;
                (
                    InstructionType::Store,
                    SmallVec::from_slice(&[
                        Operand::Register(rs),
                        Operand::Register(ra),
                        Operand::Immediate(d),
                    ]),
                )
            }

            // Opcode 44: Store halfword (sth)
            // Format: sth RS, D(RA)
            44 => {
                let rs: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let d: i16 = (word & 0xFFFF) as i16;
                (
                    InstructionType::Store,
                    SmallVec::from_slice(&[
                        Operand::Register(rs),
                        Operand::Register(ra),
                        Operand::Immediate(d),
                    ]),
                )
            }

            // Opcode 33: Load word with update (lwzu)
            // Format: lwzu RT, D(RA) - updates RA with effective address
            33 => {
                let rt: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let d: i16 = (word & 0xFFFF) as i16;
                (
                    InstructionType::Load,
                    SmallVec::from_slice(&[
                        Operand::Register(rt),
                        Operand::Register(ra),
                        Operand::Immediate(d),
                    ]),
                )
            }

            // Opcode 37: Store word with update (stwu)
            // Format: stwu RS, D(RA) - updates RA with effective address
            37 => {
                let rs: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let d: i16 = (word & 0xFFFF) as i16;
                (
                    InstructionType::Store,
                    SmallVec::from_slice(&[
                        Operand::Register(rs),
                        Operand::Register(ra),
                        Operand::Immediate(d),
                    ]),
                )
            }

            // Opcode 48: Floating-point load single (lfs)
            // Format: lfs FRT, D(RA)
            48 => {
                let frt: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let d: i16 = (word & 0xFFFF) as i16;
                (
                    InstructionType::FloatingPoint,
                    SmallVec::from_slice(&[
                        Operand::FpRegister(frt),
                        Operand::Register(ra),
                        Operand::Immediate(d),
                    ]),
                )
            }

            // Opcode 50: Floating-point load double (lfd)
            // Format: lfd FRT, D(RA)
            50 => {
                let frt: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let d: i16 = (word & 0xFFFF) as i16;
                (
                    InstructionType::FloatingPoint,
                    SmallVec::from_slice(&[
                        Operand::FpRegister(frt),
                        Operand::Register(ra),
                        Operand::Immediate(d),
                    ]),
                )
            }

            // Opcode 52: Floating-point store single (stfs)
            // Format: stfs FRS, D(RA)
            52 => {
                let frs: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let d: i16 = (word & 0xFFFF) as i16;
                (
                    InstructionType::FloatingPoint,
                    SmallVec::from_slice(&[
                        Operand::FpRegister(frs),
                        Operand::Register(ra),
                        Operand::Immediate(d),
                    ]),
                )
            }

            // Opcode 54: Floating-point store double (stfd)
            // Format: stfd FRS, D(RA)
            54 => {
                let frs: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let d: i16 = (word & 0xFFFF) as i16;
                (
                    InstructionType::FloatingPoint,
                    SmallVec::from_slice(&[
                        Operand::FpRegister(frs),
                        Operand::Register(ra),
                        Operand::Immediate(d),
                    ]),
                )
            }

            // Opcode 35: Load byte with update (lbzu)
            // Format: lbzu RT, D(RA) - updates RA with effective address
            35 => {
                let rt: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let d: i16 = (word & 0xFFFF) as i16;
                (
                    InstructionType::Load,
                    SmallVec::from_slice(&[
                        Operand::Register(rt),
                        Operand::Register(ra),
                        Operand::Immediate(d),
                    ]),
                )
            }

            // Opcode 41: Load halfword with update (lhzu)
            // Format: lhzu RT, D(RA) - updates RA with effective address
            41 => {
                let rt: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let d: i16 = (word & 0xFFFF) as i16;
                (
                    InstructionType::Load,
                    SmallVec::from_slice(&[
                        Operand::Register(rt),
                        Operand::Register(ra),
                        Operand::Immediate(d),
                    ]),
                )
            }

            // Opcode 43: Load halfword algebraic with update (lhau)
            // Format: lhau RT, D(RA) - updates RA with effective address
            43 => {
                let rt: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let d: i16 = (word & 0xFFFF) as i16;
                (
                    InstructionType::Load,
                    SmallVec::from_slice(&[
                        Operand::Register(rt),
                        Operand::Register(ra),
                        Operand::Immediate(d),
                    ]),
                )
            }

            // Opcode 39: Store byte with update (stbu)
            // Format: stbu RS, D(RA) - updates RA with effective address
            39 => {
                let rs: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let d: i16 = (word & 0xFFFF) as i16;
                (
                    InstructionType::Store,
                    SmallVec::from_slice(&[
                        Operand::Register(rs),
                        Operand::Register(ra),
                        Operand::Immediate(d),
                    ]),
                )
            }

            // Opcode 45: Store halfword with update (sthu)
            // Format: sthu RS, D(RA) - updates RA with effective address
            45 => {
                let rs: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let d: i16 = (word & 0xFFFF) as i16;
                (
                    InstructionType::Store,
                    SmallVec::from_slice(&[
                        Operand::Register(rs),
                        Operand::Register(ra),
                        Operand::Immediate(d),
                    ]),
                )
            }

            // Opcode 49: Floating-point load single with update (lfsu)
            // Format: lfsu FRT, D(RA) - updates RA with effective address
            49 => {
                let frt: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let d: i16 = (word & 0xFFFF) as i16;
                (
                    InstructionType::FloatingPoint,
                    SmallVec::from_slice(&[
                        Operand::FpRegister(frt),
                        Operand::Register(ra),
                        Operand::Immediate(d),
                    ]),
                )
            }

            // Opcode 51: Floating-point load double with update (lfdu)
            // Format: lfdu FRT, D(RA) - updates RA with effective address
            51 => {
                let frt: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let d: i16 = (word & 0xFFFF) as i16;
                (
                    InstructionType::FloatingPoint,
                    SmallVec::from_slice(&[
                        Operand::FpRegister(frt),
                        Operand::Register(ra),
                        Operand::Immediate(d),
                    ]),
                )
            }

            // Opcode 53: Floating-point store single with update (stfsu)
            // Format: stfsu FRS, D(RA) - updates RA with effective address
            53 => {
                let frs: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let d: i16 = (word & 0xFFFF) as i16;
                (
                    InstructionType::FloatingPoint,
                    SmallVec::from_slice(&[
                        Operand::FpRegister(frs),
                        Operand::Register(ra),
                        Operand::Immediate(d),
                    ]),
                )
            }

            // Opcode 55: Floating-point store double with update (stfdu)
            // Format: stfdu FRS, D(RA) - updates RA with effective address
            55 => {
                let frs: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let d: i16 = (word & 0xFFFF) as i16;
                (
                    InstructionType::FloatingPoint,
                    SmallVec::from_slice(&[
                        Operand::FpRegister(frs),
                        Operand::Register(ra),
                        Operand::Immediate(d),
                    ]),
                )
            }

            // Opcode 0: Illegal instruction (trap)
            // Format: trap - causes system trap
            0 => (InstructionType::System, SmallVec::new()),

            // Opcode 1: Trap word immediate (twi)
            // Format: twi TO, RA, SI
            // TO = bits 6-10 (trap conditions), RA = bits 16-20, SI = bits 0-15
            1 => {
                let to: u8 = ((word >> 6) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let si: i16 = (word & 0xFFFF) as i16;
                (
                    InstructionType::System,
                    SmallVec::from_slice(&[
                        Operand::Condition(to),
                        Operand::Register(ra),
                        Operand::Immediate(si),
                    ]),
                )
            }

            // Opcode 2: Multiply low immediate (mulli)
            // Format: mulli RT, RA, SI
            2 => {
                let rt: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let si: i16 = (word & 0xFFFF) as i16;
                (
                    InstructionType::Arithmetic,
                    SmallVec::from_slice(&[
                        Operand::Register(rt),
                        Operand::Register(ra),
                        Operand::Immediate(si),
                    ]),
                )
            }

            // Opcode 3: Subtract from immediate carrying (subfic)
            // Already implemented as opcode 15, but opcode 3 is also used for some variants
            // Opcode 3: Load word algebraic (lwa) - 64-bit only, not on GameCube
            // For GameCube compatibility, treat as unknown or similar to lwz
            3 => {
                // On GameCube, this might be used differently, but we'll decode it as load
                let rt: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let d: i16 = (word & 0xFFFF) as i16;
                (
                    InstructionType::Load,
                    SmallVec::from_slice(&[
                        Operand::Register(rt),
                        Operand::Register(ra),
                        Operand::Immediate(d),
                    ]),
                )
            }

            // Opcode 4: Add carrying (addc)
            // Format: addc RT, RA, RB - handled in extended opcodes
            // Opcode 4: Load word and reserve indexed (lwarx) - extended opcode
            // For primary opcode 4, treat as reserved/unknown on 32-bit
            4 => (InstructionType::Unknown, SmallVec::new()),

            // Opcode 5: Subtract from carrying (subfc)
            // Format: subfc RT, RA, RB - handled in extended opcodes
            // Opcode 5: Store word conditional indexed (stwcx.) - extended opcode
            // For primary opcode 5, treat as reserved/unknown on 32-bit
            5 => (InstructionType::Unknown, SmallVec::new()),

            // Opcode 6: Add extended (adde)
            // Format: adde RT, RA, RB - handled in extended opcodes
            // Opcode 6: Load double word (ld) - 64-bit only, not on GameCube
            6 => (InstructionType::Unknown, SmallVec::new()),

            // Opcode 7: Subtract from extended (subfe)
            // Format: subfe RT, RA, RB - handled in extended opcodes
            // Opcode 7: Store double word (std) - 64-bit only, not on GameCube
            7 => (InstructionType::Unknown, SmallVec::new()),

            // Opcode 8: Add extended carrying (addze)
            // Format: addze RT, RA - handled in extended opcodes
            // Opcode 8: Load floating-point as integer word (lfq) - not on GameCube
            8 => (InstructionType::Unknown, SmallVec::new()),

            // Opcode 9: Subtract from extended zero (subfze)
            // Format: subfze RT, RA - handled in extended opcodes
            // Opcode 9: Store floating-point as integer word (stfq) - not on GameCube
            9 => (InstructionType::Unknown, SmallVec::new()),

            // Opcode 10: Compare logical word immediate (cmplwi) - already implemented above

            // Opcode 11: Compare word immediate (cmpwi) - already implemented above

            // Opcode 12: Add immediate shifted (addis)
            // Format: addis RT, RA, SI
            12 => {
                let rt: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let si: i16 = (word & 0xFFFF) as i16;
                (
                    InstructionType::Arithmetic,
                    SmallVec::from_slice(&[
                        Operand::Register(rt),
                        Operand::Register(ra),
                        Operand::Immediate(si),
                    ]),
                )
            }

            // Opcode 13: Compare immediate (cmpi)
            // Format: cmpi BF, L, RA, SI
            // BF = bits 23-25, L = bit 21, RA = bits 16-20, SI = bits 0-15
            13 => {
                let bf: u8 = ((word >> 23) & 0x7) as u8;
                let l: u8 = ((word >> 21) & 1) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let si: i16 = (word & 0xFFFF) as i16;
                (
                    InstructionType::Compare,
                    SmallVec::from_slice(&[
                        Operand::Condition(bf),
                        Operand::Immediate(l as i16),
                        Operand::Register(ra),
                        Operand::Immediate(si),
                    ]),
                )
            }

            // Opcode 14: Add immediate (addi) - already implemented above

            // Opcode 15: Subtract from immediate (subfic) - already implemented above

            // Opcode 16: Branch conditional (bc) - already implemented above

            // Opcode 17: Sc (system call) - not typically used on GameCube
            17 => (InstructionType::System, SmallVec::new()),

            // Opcode 18: Branch (b) - already implemented above

            // Opcode 19: Branch conditional to count register (bcctr)
            // Format: bcctr BO, BI, LK
            // BO = bits 21-25, BI = bits 16-20, LK = bit 0
            19 => {
                let bo: u8 = ((word >> 21) & 0x1F) as u8;
                let bi: u8 = ((word >> 16) & 0x1F) as u8;
                let lk: u8 = (word & 1) as u8;
                (
                    InstructionType::Branch,
                    SmallVec::from_slice(&[
                        Operand::Condition(bo),
                        Operand::Condition(bi),
                        Operand::Immediate(lk as i16),
                    ]),
                )
            }

            // Opcode 20: Rotate left word immediate then AND with mask (rlwimi)
            // Format: rlwimi RA, RS, SH, MB, ME
            // Handled in extended opcodes, but primary opcode 20 is also used
            20 => {
                let rs: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let sh: u8 = ((word >> 11) & 0x1F) as u8;
                let mb: u8 = ((word >> 6) & 0x1F) as u8;
                let me: u8 = (word & 0x1F) as u8;
                let mask: u32 = compute_mask(mb, me);
                (
                    InstructionType::Rotate,
                    SmallVec::from_slice(&[
                        Operand::Register(rs),
                        Operand::Register(ra),
                        Operand::ShiftAmount(sh),
                        Operand::Mask(mask),
                    ]),
                )
            }

            // Opcode 21: Rotate left word immediate then AND with mask (rlwinm)
            // Format: rlwinm RA, RS, SH, MB, ME
            21 => {
                let rs: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let sh: u8 = ((word >> 11) & 0x1F) as u8;
                let mb: u8 = ((word >> 6) & 0x1F) as u8;
                let me: u8 = (word & 0x1F) as u8;
                let mask: u32 = compute_mask(mb, me);
                (
                    InstructionType::Rotate,
                    SmallVec::from_slice(&[
                        Operand::Register(rs),
                        Operand::Register(ra),
                        Operand::ShiftAmount(sh),
                        Operand::Mask(mask),
                    ]),
                )
            }

            // Opcode 22: Rotate left word then AND with mask (rlwnm)
            // Format: rlwnm RA, RS, RB, MB, ME
            // Handled in extended opcodes
            22 => {
                let rs: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let rb: u8 = ((word >> 11) & 0x1F) as u8;
                let mb: u8 = ((word >> 6) & 0x1F) as u8;
                let me: u8 = (word & 0x1F) as u8;
                let mask: u32 = compute_mask(mb, me);
                (
                    InstructionType::Rotate,
                    SmallVec::from_slice(&[
                        Operand::Register(rs),
                        Operand::Register(ra),
                        Operand::Register(rb),
                        Operand::Mask(mask),
                    ]),
                )
            }

            // Opcode 23: Rotate left word immediate then OR immediate (rlwimi)
            // Format: rlwimi RA, RS, SH, MB, ME
            // Similar to opcode 20, but with OR semantics
            23 => {
                let rs: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let sh: u8 = ((word >> 11) & 0x1F) as u8;
                let mb: u8 = ((word >> 6) & 0x1F) as u8;
                let me: u8 = (word & 0x1F) as u8;
                let mask: u32 = compute_mask(mb, me);
                (
                    InstructionType::Rotate,
                    SmallVec::from_slice(&[
                        Operand::Register(rs),
                        Operand::Register(ra),
                        Operand::ShiftAmount(sh),
                        Operand::Mask(mask),
                    ]),
                )
            }

            // Opcode 24: OR immediate (ori) - already implemented above

            // Opcode 25: OR immediate shifted (oris)
            // Format: oris RT, RA, UI
            25 => {
                let rt: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let ui: u16 = (word & 0xFFFF) as u16;
                (
                    InstructionType::Arithmetic,
                    SmallVec::from_slice(&[
                        Operand::Register(rt),
                        Operand::Register(ra),
                        Operand::Immediate(ui as i16),
                    ]),
                )
            }

            // Opcode 26: XOR immediate (xori) - already implemented above

            // Opcode 27: XOR immediate shifted (xoris)
            // Format: xoris RT, RA, UI
            27 => {
                let rt: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let ui: u16 = (word & 0xFFFF) as u16;
                (
                    InstructionType::Arithmetic,
                    SmallVec::from_slice(&[
                        Operand::Register(rt),
                        Operand::Register(ra),
                        Operand::Immediate(ui as i16),
                    ]),
                )
            }

            // Opcode 28: AND immediate (andi.) - already implemented above

            // Opcode 29: AND immediate shifted (andis.)
            // Format: andis. RT, RA, UI
            29 => {
                let rt: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let ui: u16 = (word & 0xFFFF) as u16;
                (
                    InstructionType::Arithmetic,
                    SmallVec::from_slice(&[
                        Operand::Register(rt),
                        Operand::Register(ra),
                        Operand::Immediate(ui as i16),
                    ]),
                )
            }

            // Opcode 30: Load word and reserve (lwarx)
            // Format: lwarx RT, RA, RB
            // Handled in extended opcodes, but primary opcode 30 is reserved
            30 => (InstructionType::Unknown, SmallVec::new()),

            // Opcode 31: Extended opcodes - already handled above

            // Opcode 32: Load word and zero (lwz) - already implemented above

            // Opcode 33: Load word with update (lwzu) - already implemented above

            // Opcode 34: Load byte and zero (lbz) - already implemented above

            // Opcode 35: Load byte with update (lbzu) - already implemented above

            // Opcode 36: Store word (stw) - already implemented above

            // Opcode 37: Store word with update (stwu) - already implemented above

            // Opcode 38: Store byte (stb) - already implemented above

            // Opcode 39: Store byte with update (stbu) - already implemented above

            // Opcode 40: Load halfword and zero (lhz) - already implemented above

            // Opcode 41: Load halfword with update (lhzu) - already implemented above

            // Opcode 42: Load halfword algebraic (lha) - already implemented above

            // Opcode 43: Load halfword algebraic with update (lhau) - already implemented above

            // Opcode 44: Store halfword (sth) - already implemented above

            // Opcode 45: Store halfword with update (sthu) - already implemented above

            // Opcode 46: Load multiple word (lmw)
            // Format: lmw RT, D(RA)
            46 => {
                let rt: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let d: i16 = (word & 0xFFFF) as i16;
                (
                    InstructionType::Load,
                    SmallVec::from_slice(&[
                        Operand::Register(rt),
                        Operand::Register(ra),
                        Operand::Immediate(d),
                    ]),
                )
            }

            // Opcode 47: Store multiple word (stmw)
            // Format: stmw RS, D(RA)
            47 => {
                let rs: u8 = ((word >> 21) & 0x1F) as u8;
                let ra: u8 = ((word >> 16) & 0x1F) as u8;
                let d: i16 = (word & 0xFFFF) as i16;
                (
                    InstructionType::Store,
                    SmallVec::from_slice(&[
                        Operand::Register(rs),
                        Operand::Register(ra),
                        Operand::Immediate(d),
                    ]),
                )
            }

            // Opcode 48: Floating-point load single (lfs) - already implemented above

            // Opcode 49: Floating-point load single with update (lfsu) - already implemented above

            // Opcode 50: Floating-point load double (lfd) - already implemented above

            // Opcode 51: Floating-point load double with update (lfdu) - already implemented above

            // Opcode 52: Floating-point store single (stfs) - already implemented above

            // Opcode 53: Floating-point store single with update (stfsu) - already implemented above

            // Opcode 54: Floating-point store double (stfd) - already implemented above

            // Opcode 55: Floating-point store double with update (stfdu) - already implemented above

            // Opcode 56: Load floating-point as integer word (lfiwax)
            // Format: lfiwax FRT, RA, RB
            // Handled in extended opcodes
            56 => (InstructionType::Unknown, SmallVec::new()),

            // Opcode 57: Load floating-point as integer word zero (lfiwzx)
            // Format: lfiwzx FRT, RA, RB
            // Handled in extended opcodes
            57 => (InstructionType::Unknown, SmallVec::new()),

            // Opcode 58: Store floating-point as integer word (stfiwx)
            // Format: stfiwx FRS, RA, RB
            // Handled in extended opcodes
            58 => (InstructionType::Unknown, SmallVec::new()),

            // Opcode 59: Floating-point operations (primary opcode 59)
            // Format: Various floating-point instructions
            // Handled in extended opcodes (opcode 63)
            59 => (InstructionType::Unknown, SmallVec::new()),

            // Opcode 60: Floating-point operations (primary opcode 60)
            // Format: Various floating-point instructions
            // Handled in extended opcodes (opcode 63)
            60 => (InstructionType::Unknown, SmallVec::new()),

            // Opcode 61: Floating-point operations (primary opcode 61)
            // Format: Various floating-point instructions
            // Handled in extended opcodes (opcode 63)
            61 => (InstructionType::Unknown, SmallVec::new()),

            // Opcode 62: Floating-point operations (primary opcode 62)
            // Format: Various floating-point instructions
            // Handled in extended opcodes (opcode 63)
            62 => (InstructionType::Unknown, SmallVec::new()),

            // Opcode 63: Floating-point operations
            // Format: Various floating-point instructions (fadd, fsub, fmul, fdiv, etc.)
            // Handled in extended opcodes
            63 => Self::decode_extended(word)?,

            // Opcode 31 with specific patterns for move instructions
            // Move from link register (mflr) - extended opcode 8
            31 if ((word >> 21) & 0x1F) == 8 && (word & 0x7FF) == 0 => {
                let rt: u8 = ((word >> 21) & 0x1F) as u8;
                (
                    InstructionType::Move,
                    SmallVec::from_slice(&[Operand::Register(rt)]),
                )
            }
            // Move to link register (mtlr) - extended opcode 9
            31 if ((word >> 21) & 0x1F) == 9 && (word & 0x7FF) == 0 => {
                let rs: u8 = ((word >> 21) & 0x1F) as u8;
                (
                    InstructionType::Move,
                    SmallVec::from_slice(&[Operand::Register(rs)]),
                )
            }
            // Move from count register (mfctr) - extended opcode 9, sub-opcode 9
            31 if ((word >> 21) & 0x1F) == 9
                && ((word >> 11) & 0x1F) == 9
                && (word & 0x7FF) == 0 =>
            {
                let rt: u8 = ((word >> 21) & 0x1F) as u8;
                (
                    InstructionType::Move,
                    SmallVec::from_slice(&[Operand::Register(rt)]),
                )
            }
            // Move to count register (mtctr) - extended opcode 9, sub-opcode 9
            31 if ((word >> 21) & 0x1F) == 9
                && ((word >> 11) & 0x1F) == 9
                && (word & 0x7FF) == 0 =>
            {
                let rs: u8 = ((word >> 21) & 0x1F) as u8;
                (
                    InstructionType::Move,
                    SmallVec::from_slice(&[Operand::Register(rs)]),
                )
            }

            // Unknown opcode - return unknown instruction type
            _ => {
                // Check if this might be data rather than code
                if Self::is_likely_data(word) {
                    log::warn!("Unknown opcode 0x{:02X} at 0x{:08X} (0x{:08X}) - may be data, not code", 
                              opcode, address, word);
                } else {
                    log::warn!("Unknown opcode 0x{:02X} at 0x{:08X} (0x{:08X})", 
                              opcode, address, word);
                }
                (InstructionType::Unknown, SmallVec::new())
            },
        };

        // Validate decoded instruction
        if let Err(e) = Self::validate_decoded_instruction(&instruction_type, &operands) {
            log::warn!("Invalid instruction at 0x{:08X}: {}", address, e);
        }

        Ok(DecodedInstruction {
            instruction: Instruction {
                opcode,
                instruction_type,
                operands,
            },
            raw: word,
            address,
        })
    }
    
    /// Check if a word is likely data rather than code
    fn is_likely_data(word: u32) -> bool {
        // Patterns that suggest data:
        // - All zeros
        // - Repeated patterns (0x00000000, 0xFFFFFFFF, etc.)
        // - Very low opcode with all zeros in operand fields
        if word == 0 || word == 0xFFFFFFFF {
            return true;
        }
        
        // Check for patterns like 0x0000XXXX or 0xXXXX0000
        if (word & 0xFFFF0000) == 0 || (word & 0x0000FFFF) == 0 {
            // Might be data, but not definitive
            return false;
        }
        
        false
    }
    
    /// Validate decoded instruction for consistency
    fn validate_decoded_instruction(
        instruction_type: &InstructionType,
        operands: &SmallVec<[Operand; 4]>,
    ) -> Result<()> {
        // Validate register operands are in valid range (0-31)
        for operand in operands.iter() {
            match operand {
                Operand::Register(r) | Operand::FpRegister(r) => {
                    if *r > 31 {
                        anyhow::bail!("Invalid register number: {} (must be 0-31)", r);
                    }
                }
                Operand::Condition(c) => {
                    if *c > 7 {
                        anyhow::bail!("Invalid condition register field: {} (must be 0-7)", c);
                    }
                }
                Operand::ShiftAmount(s) => {
                    if *s > 31 {
                        anyhow::bail!("Invalid shift amount: {} (must be 0-31)", s);
                    }
                }
                _ => {}
            }
        }
        
        // Validate instruction-specific constraints
        match instruction_type {
            InstructionType::Load | InstructionType::Store => {
                // Load/store should have at least 2 operands (register and address/immediate)
                if operands.len() < 2 {
                    anyhow::bail!("Load/store instruction requires at least 2 operands");
                }
            }
            InstructionType::Branch => {
                // Branch should have at least 1 operand (target)
                if operands.is_empty() {
                    anyhow::bail!("Branch instruction requires at least 1 operand");
                }
            }
            _ => {}
        }
        
        Ok(())
    }
}

/// Decode extended opcodes (opcode 31 instructions).
///
/// Extended opcodes use a secondary opcode field in bits 1-10 of the instruction word.
/// This function handles arithmetic, logical, shift, rotate, floating-point, and system instructions.
///
/// # Arguments
/// * `word` - 32-bit instruction word with opcode 31
///
/// # Returns
/// `Result<(InstructionType, SmallVec<[Operand; 4]>)>` - Instruction type and operands
#[inline] // Hot path for extended opcodes
fn decode_extended(word: u32) -> Result<(InstructionType, SmallVec<[Operand; 4]>)> {
    // Extract secondary opcode (bits 1-10)
    let extended_opcode: u32 = (word >> 1) & 0x3FF;

    // Extract common register fields
    let ra: u8 = ((word >> 16) & 0x1F) as u8;
    let rb: u8 = ((word >> 11) & 0x1F) as u8;
    let rs: u8 = ((word >> 21) & 0x1F) as u8;
    let rt: u8 = ((word >> 21) & 0x1F) as u8;
    let rc: bool = (word & 1) != 0; // Record bit (update condition register)

    // Check for specific instruction patterns first (move instructions)
    // Move from link register (mflr) - RT field = 8, all other fields = 0
    if ((word >> 21) & 0x1F) == 8 && (word & 0x7FF) == 0 {
        return Ok((
            InstructionType::Move,
            SmallVec::from_slice(&[Operand::Register(rt)]),
        ));
    }
    // Move to link register (mtlr) - RS field = 9, all other fields = 0
    if ((word >> 21) & 0x1F) == 9 && (word & 0x7FF) == 0 {
        return Ok((
            InstructionType::Move,
            SmallVec::from_slice(&[Operand::Register(rs)]),
        ));
    }

    // Decode based on extended opcode
    match extended_opcode {
        // Extended opcode 266: Add (add)
        // Format: add RT, RA, RB
        // Only if primary opcode is 31 (not 63)
        266 if (word >> 26) == 31 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[
                Operand::Register(rt),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 40: Subtract from (subf)
        // Format: subf RT, RA, RB (RT = RB - RA)
        // Only if primary opcode is 31 (not 63, which is fneg)
        40 if (word >> 26) == 31 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[
                Operand::Register(rt),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 138: Add carrying (addc)
        // Format: addc RT, RA, RB (RT = RA + RB, with carry)
        138 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[
                Operand::Register(rt),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 10: Add extended (adde)
        // Format: adde RT, RA, RB (RT = RA + RB + CA, with carry)
        10 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[
                Operand::Register(rt),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 202: Add extended carrying (addze)
        // Format: addze RT, RA (RT = RA + CA, with carry)
        202 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[Operand::Register(rt), Operand::Register(ra)]),
        )),

        // Extended opcode 234: Add to minus one extended (addme)
        // Format: addme RT, RA (RT = RA + CA - 1, with carry)
        234 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[Operand::Register(rt), Operand::Register(ra)]),
        )),

        // Extended opcode 74: Subtract from extended zero (subfze)
        // Format: subfze RT, RA (RT = CA - RA - 1, with carry)
        74 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[Operand::Register(rt), Operand::Register(ra)]),
        )),

        // Extended opcode 106: Subtract from minus one extended (subfme)
        // Format: subfme RT, RA (RT = CA - RA - 2, with carry)
        106 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[Operand::Register(rt), Operand::Register(ra)]),
        )),

        // Extended opcode 75: Negate (neg)
        // Format: neg RT, RA (RT = -RA)
        75 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[Operand::Register(rt), Operand::Register(ra)]),
        )),

        // Extended opcode 104: Negate with overflow (nego)
        // Format: nego RT, RA (RT = -RA, sets overflow)
        104 if (word >> 26) == 31 && ra != 0 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[Operand::Register(rt), Operand::Register(ra)]),
        )),

        // Extended opcode 232: Add carrying with overflow (addco)
        // Format: addco RT, RA, RB (RT = RA + RB, with carry and overflow)
        232 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[
                Operand::Register(rt),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 233: Add extended with overflow (addeo)
        // Format: addeo RT, RA, RB (RT = RA + RB + CA, with carry and overflow)
        233 if (word >> 26) == 31 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[
                Operand::Register(rt),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 234: Add to minus one extended with overflow (addmeo)
        // Format: addmeo RT, RA (RT = RA + CA - 1, with carry and overflow)
        234 if (word >> 26) == 31 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[Operand::Register(rt), Operand::Register(ra)]),
        )),

        // Extended opcode 202: Add extended carrying with overflow (addzeo)
        // Format: addzeo RT, RA (RT = RA + CA, with carry and overflow)
        202 if (word >> 26) == 31 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[Operand::Register(rt), Operand::Register(ra)]),
        )),

        // Extended opcode 8: Subtract from carrying with overflow (subfco)
        // Format: subfco RT, RA, RB (RT = RB - RA, with carry and overflow)
        8 if (word >> 26) == 31 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[
                Operand::Register(rt),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 136: Subtract from extended with overflow (subfeo)
        // Format: subfeo RT, RA, RB (RT = RB - RA - (1 - CA), with carry and overflow)
        136 if (word >> 26) == 31 && ra != 0 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[
                Operand::Register(rt),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 74: Subtract from extended zero with overflow (subfzeo)
        // Format: subfzeo RT, RA (RT = CA - RA - 1, with carry and overflow)
        74 if (word >> 26) == 31 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[Operand::Register(rt), Operand::Register(ra)]),
        )),

        // Extended opcode 106: Subtract from minus one extended with overflow (subfmeo)
        // Format: subfmeo RT, RA (RT = CA - RA - 2, with carry and overflow)
        106 if (word >> 26) == 31 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[Operand::Register(rt), Operand::Register(ra)]),
        )),

        // Extended opcode 107: Multiply low word with overflow (mullwo)
        // Format: mullwo RT, RA, RB (RT = RA * RB, sets overflow)
        107 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[
                Operand::Register(rt),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 200: Divide word with overflow (divwo)
        // Format: divwo RT, RA, RB (RT = RA / RB, sets overflow)
        200 if (word >> 26) == 31 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[
                Operand::Register(rt),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 201: Divide word unsigned with overflow (divwuo)
        // Format: divwuo RT, RA, RB (RT = RA / RB unsigned, sets overflow)
        201 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[
                Operand::Register(rt),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 235: Multiply low word (mullw)
        // Format: mullw RT, RA, RB
        235 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[
                Operand::Register(rt),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 233: Multiply high word (mulhw)
        // Format: mulhw RT, RA, RB
        233 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[
                Operand::Register(rt),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 11: Multiply high word unsigned (mulhwu)
        // Format: mulhwu RT, RA, RB
        11 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[
                Operand::Register(rt),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 200: Divide word unsigned (divwu)
        // Format: divwu RT, RA, RB (RT = RA / RB, unsigned)
        200 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[
                Operand::Register(rt),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 104: Divide word (divw) - already implemented above
        // Extended opcode 40: Subtract from (subf) - already implemented above

        // Extended opcode 8: Subtract from carrying (subfc)
        // Format: subfc RT, RA, RB (RT = RB - RA, with carry)
        8 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[
                Operand::Register(rt),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 136: Subtract from extended (subfe)
        // Format: subfe RT, RA, RB (RT = RB - RA - (1 - CA), with carry)
        // Only if primary opcode is 31 (not 63, which is fnabs)
        136 if (word >> 26) == 31 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[
                Operand::Register(rt),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 104: Divide word (divw)
        // Format: divw RT, RA, RB (RT = RA / RB)
        // Only if primary opcode is 31 (not 63)
        104 if (word >> 26) == 31 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[
                Operand::Register(rt),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 28: AND (and)
        // Format: and RS, RA, RB
        // Only if primary opcode is 31 (not 63)
        28 if (word >> 26) == 31 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[
                Operand::Register(rs),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 60: AND with complement (andc)
        // Format: andc RS, RA, RB (RS = RA & ~RB)
        60 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[
                Operand::Register(rs),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 444: OR (or)
        // Format: or RS, RA, RB
        // Only if primary opcode is 31 (not 63)
        444 if (word >> 26) == 31 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[
                Operand::Register(rs),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 412: OR with complement (orc)
        // Format: orc RS, RA, RB (RS = RA | ~RB)
        412 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[
                Operand::Register(rs),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 316: XOR (xor)
        // Format: xor RS, RA, RB
        // Only if primary opcode is 31 (not 63)
        316 if (word >> 26) == 31 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[
                Operand::Register(rs),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 476: NAND (nand)
        // Format: nand RS, RA, RB
        // Only if primary opcode is 31 (not 63)
        476 if (word >> 26) == 31 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[
                Operand::Register(rs),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 124: NOR (nor)
        // Format: nor RS, RA, RB
        // Only if primary opcode is 31 (not 63)
        124 if (word >> 26) == 31 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[
                Operand::Register(rs),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 284: Equivalent (eqv)
        // Format: eqv RS, RA, RB (RS = ~(RA ^ RB))
        284 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[
                Operand::Register(rs),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 24: Shift left word (slw)
        // Format: slw RA, RS, RB (RA = RS << (RB & 0x1F))
        // Only if primary opcode is 31 (not 63)
        24 if (word >> 26) == 31 => {
            let sh: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::Shift,
                SmallVec::from_slice(&[
                    Operand::Register(rs),
                    Operand::Register(ra),
                    Operand::ShiftAmount(sh),
                ]),
            ))
        }

        // Extended opcode 536: Shift right word (srw)
        // Format: srw RA, RS, RB (RA = RS >> (RB & 0x1F))
        // Only if primary opcode is 31 (not 63)
        536 if (word >> 26) == 31 => {
            let sh: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::Shift,
                SmallVec::from_slice(&[
                    Operand::Register(rs),
                    Operand::Register(ra),
                    Operand::ShiftAmount(sh),
                ]),
            ))
        }

        // Extended opcode 824: Shift left word immediate (slwi)
        // Format: slwi RA, RS, SH (RA = RS << SH)
        // This is actually rlwinm with MB=0, ME=31-SH
        824 => {
            let sh: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::Shift,
                SmallVec::from_slice(&[
                    Operand::Register(rs),
                    Operand::Register(ra),
                    Operand::ShiftAmount(sh),
                ]),
            ))
        }

        // Extended opcode 792: Shift right word immediate (srwi)
        // Format: srwi RA, RS, SH (RA = RS >> SH)
        // This is actually rlwinm with SH=32-SH, MB=SH, ME=31
        792 if (word >> 26) == 31 => {
            let sh: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::Shift,
                SmallVec::from_slice(&[
                    Operand::Register(rs),
                    Operand::Register(ra),
                    Operand::ShiftAmount(sh),
                ]),
            ))
        }

        // Extended opcode 794: Shift right algebraic word (sraw)
        // Format: sraw RA, RS, RB (arithmetic right shift)
        794 => {
            let sh: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::Shift,
                SmallVec::from_slice(&[
                    Operand::Register(rs),
                    Operand::Register(ra),
                    Operand::ShiftAmount(sh),
                ]),
            ))
        }

        // Extended opcode 826: Shift right algebraic word immediate (srawi)
        // Format: srawi RA, RS, SH (arithmetic right shift by immediate)
        826 => {
            let sh: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::Shift,
                SmallVec::from_slice(&[
                    Operand::Register(rs),
                    Operand::Register(ra),
                    Operand::ShiftAmount(sh),
                ]),
            ))
        }

        // Extended opcode 26: Count leading zeros word (cntlzw)
        // Format: cntlzw RA, RS
        26 => Ok((
            InstructionType::Arithmetic,
            SmallVec::from_slice(&[Operand::Register(rs), Operand::Register(ra)]),
        )),

        // Extended opcode 0: Compare word (cmpw)
        // Format: cmpw BF, RA, RB
        // Only if primary opcode is 31 and extended opcode is 0
        0 if (word >> 26) == 31 && ((word >> 1) & 0x3FF) == 0 => {
            let bf: u8 = ((word >> 23) & 0x7) as u8;
            Ok((
                InstructionType::Compare,
                SmallVec::from_slice(&[
                    Operand::Condition(bf),
                    Operand::Register(ra),
                    Operand::Register(rb),
                ]),
            ))
        }

        // Extended opcode 32: Compare logical word (cmplw)
        // Format: cmplw BF, RA, RB
        32 => {
            let bf: u8 = ((word >> 23) & 0x7) as u8;
            Ok((
                InstructionType::Compare,
                SmallVec::from_slice(&[
                    Operand::Condition(bf),
                    Operand::Register(ra),
                    Operand::Register(rb),
                ]),
            ))
        }

        // Extended opcode 20: Load word and reserve indexed (lwarx)
        // Format: lwarx RT, RA, RB (load word and set reservation)
        20 if (word >> 26) == 31 => Ok((
            InstructionType::Load,
            SmallVec::from_slice(&[
                Operand::Register(rt),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 23: Load word indexed (lwzx)
        // Format: lwzx RT, RA, RB
        23 => Ok((
            InstructionType::Load,
            SmallVec::from_slice(&[
                Operand::Register(rt),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 150: Store word conditional indexed (stwcx.)
        // Format: stwcx. RS, RA, RB (store word conditional, sets CR0)
        150 if (word >> 26) == 31 => Ok((
            InstructionType::Store,
            SmallVec::from_slice(&[
                Operand::Register(rs),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 87: Load byte indexed (lbzx)
        // Format: lbzx RT, RA, RB
        87 => Ok((
            InstructionType::Load,
            SmallVec::from_slice(&[
                Operand::Register(rt),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 279: Load halfword indexed (lhzx)
        // Format: lhzx RT, RA, RB
        279 => Ok((
            InstructionType::Load,
            SmallVec::from_slice(&[
                Operand::Register(rt),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 343: Load halfword algebraic indexed (lhax)
        // Format: lhax RT, RA, RB
        343 => Ok((
            InstructionType::Load,
            SmallVec::from_slice(&[
                Operand::Register(rt),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 151: Store word indexed (stwx)
        // Format: stwx RS, RA, RB
        151 => Ok((
            InstructionType::Store,
            SmallVec::from_slice(&[
                Operand::Register(rs),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 215: Store byte indexed (stbx)
        // Format: stbx RS, RA, RB
        215 => Ok((
            InstructionType::Store,
            SmallVec::from_slice(&[
                Operand::Register(rs),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 407: Store halfword indexed (sthx)
        // Format: sthx RS, RA, RB
        407 => Ok((
            InstructionType::Store,
            SmallVec::from_slice(&[
                Operand::Register(rs),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 55: Load word with update indexed (lwzux)
        // Format: lwzux RT, RA, RB - updates RA with effective address
        55 => Ok((
            InstructionType::Load,
            SmallVec::from_slice(&[
                Operand::Register(rt),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 119: Load byte with update indexed (lbzux)
        // Format: lbzux RT, RA, RB - updates RA with effective address
        119 => Ok((
            InstructionType::Load,
            SmallVec::from_slice(&[
                Operand::Register(rt),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 311: Load halfword with update indexed (lhzux)
        // Format: lhzux RT, RA, RB - updates RA with effective address
        311 => Ok((
            InstructionType::Load,
            SmallVec::from_slice(&[
                Operand::Register(rt),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 375: Store word with update indexed (stwux)
        // Format: stwux RS, RA, RB - updates RA with effective address
        375 => Ok((
            InstructionType::Store,
            SmallVec::from_slice(&[
                Operand::Register(rs),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 439: Store byte with update indexed (stbux)
        // Format: stbux RS, RA, RB - updates RA with effective address
        439 => Ok((
            InstructionType::Store,
            SmallVec::from_slice(&[
                Operand::Register(rs),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 695: Store halfword with update indexed (sthux)
        // Format: sthux RS, RA, RB - updates RA with effective address
        695 => Ok((
            InstructionType::Store,
            SmallVec::from_slice(&[
                Operand::Register(rs),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 567: Floating-point load single indexed (lfsx)
        // Format: lfsx FRT, RA, RB
        567 => {
            let frt: u8 = ((word >> 21) & 0x1F) as u8;
            Ok((
                InstructionType::FloatingPoint,
                SmallVec::from_slice(&[
                    Operand::FpRegister(frt),
                    Operand::Register(ra),
                    Operand::Register(rb),
                ]),
            ))
        }

        // Extended opcode 599: Floating-point load double indexed (lfdx)
        // Format: lfdx FRT, RA, RB
        599 => {
            let frt: u8 = ((word >> 21) & 0x1F) as u8;
            Ok((
                InstructionType::FloatingPoint,
                SmallVec::from_slice(&[
                    Operand::FpRegister(frt),
                    Operand::Register(ra),
                    Operand::Register(rb),
                ]),
            ))
        }

        // Extended opcode 663: Floating-point store single indexed (stfsx)
        // Format: stfsx FRS, RA, RB
        663 => {
            let frs: u8 = ((word >> 21) & 0x1F) as u8;
            Ok((
                InstructionType::FloatingPoint,
                SmallVec::from_slice(&[
                    Operand::FpRegister(frs),
                    Operand::Register(ra),
                    Operand::Register(rb),
                ]),
            ))
        }

        // Extended opcode 727: Floating-point store double indexed (stfdx)
        // Format: stfdx FRS, RA, RB
        727 => {
            let frs: u8 = ((word >> 21) & 0x1F) as u8;
            Ok((
                InstructionType::FloatingPoint,
                SmallVec::from_slice(&[
                    Operand::FpRegister(frs),
                    Operand::Register(ra),
                    Operand::Register(rb),
                ]),
            ))
        }

        // Extended opcode 597: Load multiple word (lmw)
        // Format: lmw RT, D(RA) - loads words from RA+D to RT, RT+1, ..., RT+31
        // Note: Conflicts with lswi, but lmw uses primary opcode 46, lswi uses extended opcode
        // This is handled in primary opcode 46

        // Extended opcode 533: Store multiple word (stmw)
        // Format: stmw RS, D(RA) - stores words from RS, RS+1, ..., RS+31 to RA+D
        // Note: Conflicts with stswi, but stmw uses primary opcode 47, stswi uses extended opcode
        // This is handled in primary opcode 47

        // Extended opcode 16: Branch to link register (blr)
        // Format: blr - branch to address in link register
        16 if (word & 0x03E00001) == 0x00000001 => Ok((
            InstructionType::Branch,
            SmallVec::from_slice(&[Operand::Register(0)]), // Placeholder for LR
        )),

        // Extended opcode 528: Branch to count register (bctr)
        // Format: bctr - branch to address in count register
        // Only if primary opcode is 31 (not 63)
        528 if (word >> 26) == 31 && (word & 0x03E00001) == 0x00000001 => Ok((
            InstructionType::Branch,
            SmallVec::from_slice(&[Operand::Register(9)]), // Placeholder for CTR
        )),

        // Extended opcode 528: Branch conditional to count register (bcctr)
        // Format: bcctr BO, BI - conditional branch to CTR
        // Only if primary opcode is 31 (not 63)
        528 if (word >> 26) == 31 => {
            let bo: u8 = ((word >> 21) & 0x1F) as u8;
            let bi: u8 = ((word >> 16) & 0x1F) as u8;
            Ok((
                InstructionType::Branch,
                SmallVec::from_slice(&[Operand::Condition(bo), Operand::Condition(bi)]),
            ))
        }

        // Extended opcode 16: Branch conditional to link register (bclr)
        // Format: bclr BO, BI - conditional branch to LR
        // Only if primary opcode is 31 (not 63)
        16 if (word >> 26) == 31 => {
            let bo: u8 = ((word >> 21) & 0x1F) as u8;
            let bi: u8 = ((word >> 16) & 0x1F) as u8;
            Ok((
                InstructionType::Branch,
                SmallVec::from_slice(&[Operand::Condition(bo), Operand::Condition(bi)]),
            ))
        }

        // Extended opcode 21: Rotate left word immediate then mask insert (rlwinm)
        // Format: rlwinm RA, RS, SH, MB, ME
        // Only if primary opcode is 31 (to distinguish from floating-point add)
        21 if (word >> 26) == 31 => {
            let sh: u8 = ((word >> 11) & 0x1F) as u8;
            let mb: u8 = ((word >> 6) & 0x1F) as u8;
            let me: u8 = (word & 0x1F) as u8;
            let mask: u32 = compute_mask(mb, me);
            Ok((
                InstructionType::Rotate,
                SmallVec::from_slice(&[
                    Operand::Register(rs),
                    Operand::Register(ra),
                    Operand::ShiftAmount(sh),
                    Operand::Mask(mask),
                ]),
            ))
        }

        // Extended opcode 20: Rotate left word then AND with mask (rlwnm)
        // Format: rlwnm RA, RS, RB, MB, ME
        // Only if primary opcode is 31
        20 if (word >> 26) == 31 => {
            let mb: u8 = ((word >> 6) & 0x1F) as u8;
            let me: u8 = (word & 0x1F) as u8;
            let mask: u32 = compute_mask(mb, me);
            Ok((
                InstructionType::Rotate,
                SmallVec::from_slice(&[
                    Operand::Register(rs),
                    Operand::Register(ra),
                    Operand::Register(rb),
                    Operand::Mask(mask),
                ]),
            ))
        }

        // Extended opcode 19: Rotate left word immediate then mask insert (rlwimi)
        // Format: rlwimi RA, RS, SH, MB, ME
        // Only if primary opcode is 31
        19 if (word >> 26) == 31 => {
            let sh: u8 = ((word >> 11) & 0x1F) as u8;
            let mb: u8 = ((word >> 6) & 0x1F) as u8;
            let me: u8 = (word & 0x1F) as u8;
            let mask: u32 = compute_mask(mb, me);
            Ok((
                InstructionType::Rotate,
                SmallVec::from_slice(&[
                    Operand::Register(rs),
                    Operand::Register(ra),
                    Operand::ShiftAmount(sh),
                    Operand::Mask(mask),
                ]),
            ))
        }

        // Extended opcode 21: Floating-point add (fadd)
        // Format: fadd FRT, FRA, FRB
        // Only if primary opcode is 63 (floating-point instruction)
        21 => {
            let frt: u8 = ((word >> 21) & 0x1F) as u8;
            let fra: u8 = ((word >> 16) & 0x1F) as u8;
            let frb: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::FloatingPoint,
                SmallVec::from_slice(&[
                    Operand::FpRegister(frt),
                    Operand::FpRegister(fra),
                    Operand::FpRegister(frb),
                ]),
            ))
        }

        // Extended opcode 20: Floating-point subtract (fsub)
        // Format: fsub FRT, FRA, FRB
        20 => {
            let frt: u8 = ((word >> 21) & 0x1F) as u8;
            let fra: u8 = ((word >> 16) & 0x1F) as u8;
            let frb: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::FloatingPoint,
                SmallVec::from_slice(&[
                    Operand::FpRegister(frt),
                    Operand::FpRegister(fra),
                    Operand::FpRegister(frb),
                ]),
            ))
        }

        // Extended opcode 25: Floating-point multiply (fmul)
        // Format: fmul FRT, FRA, FRC, FRB (FRA * FRC for some variants)
        // Only if primary opcode is 63
        25 if (word >> 26) == 63 => {
            let frt: u8 = ((word >> 21) & 0x1F) as u8;
            let fra: u8 = ((word >> 16) & 0x1F) as u8;
            let frc: u8 = ((word >> 6) & 0x1F) as u8;
            let frb: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::FloatingPoint,
                SmallVec::from_slice(&[
                    Operand::FpRegister(frt),
                    Operand::FpRegister(fra),
                    Operand::FpRegister(frc),
                    Operand::FpRegister(frb),
                ]),
            ))
        }

        // Extended opcode 14: Floating-point multiply-add (fmadd)
        // Format: fmadd FRT, FRA, FRC, FRB (FRT = FRA * FRC + FRB)
        // Only if primary opcode is 63
        14 if (word >> 26) == 63 => {
            let frt: u8 = ((word >> 21) & 0x1F) as u8;
            let fra: u8 = ((word >> 16) & 0x1F) as u8;
            let frc: u8 = ((word >> 6) & 0x1F) as u8;
            let frb: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::FloatingPoint,
                SmallVec::from_slice(&[
                    Operand::FpRegister(frt),
                    Operand::FpRegister(fra),
                    Operand::FpRegister(frc),
                    Operand::FpRegister(frb),
                ]),
            ))
        }

        // Extended opcode 15: Floating-point multiply-subtract (fmsub)
        // Format: fmsub FRT, FRA, FRC, FRB (FRT = FRA * FRC - FRB)
        // Only if primary opcode is 63
        15 if (word >> 26) == 63 => {
            let frt: u8 = ((word >> 21) & 0x1F) as u8;
            let fra: u8 = ((word >> 16) & 0x1F) as u8;
            let frc: u8 = ((word >> 6) & 0x1F) as u8;
            let frb: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::FloatingPoint,
                SmallVec::from_slice(&[
                    Operand::FpRegister(frt),
                    Operand::FpRegister(fra),
                    Operand::FpRegister(frc),
                    Operand::FpRegister(frb),
                ]),
            ))
        }

        // Extended opcode 28: Floating-point negative multiply-add (fnmadd)
        // Format: fnmadd FRT, FRA, FRC, FRB (FRT = -(FRA * FRC + FRB))
        // Only if primary opcode is 63
        28 if (word >> 26) == 63 => {
            let frt: u8 = ((word >> 21) & 0x1F) as u8;
            let fra: u8 = ((word >> 16) & 0x1F) as u8;
            let frc: u8 = ((word >> 6) & 0x1F) as u8;
            let frb: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::FloatingPoint,
                SmallVec::from_slice(&[
                    Operand::FpRegister(frt),
                    Operand::FpRegister(fra),
                    Operand::FpRegister(frc),
                    Operand::FpRegister(frb),
                ]),
            ))
        }

        // Extended opcode 29: Floating-point negative multiply-subtract (fnmsub)
        // Format: fnmsub FRT, FRA, FRC, FRB (FRT = -(FRA * FRC - FRB))
        // Only if primary opcode is 63
        29 if (word >> 26) == 63 => {
            let frt: u8 = ((word >> 21) & 0x1F) as u8;
            let fra: u8 = ((word >> 16) & 0x1F) as u8;
            let frc: u8 = ((word >> 6) & 0x1F) as u8;
            let frb: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::FloatingPoint,
                SmallVec::from_slice(&[
                    Operand::FpRegister(frt),
                    Operand::FpRegister(fra),
                    Operand::FpRegister(frc),
                    Operand::FpRegister(frb),
                ]),
            ))
        }

        // Extended opcode 32: Floating-point square root (fsqrt)
        // Format: fsqrt FRT, FRB
        // Only if primary opcode is 63
        32 if (word >> 26) == 63 => {
            let frt: u8 = ((word >> 21) & 0x1F) as u8;
            let frb: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::FloatingPoint,
                SmallVec::from_slice(&[Operand::FpRegister(frt), Operand::FpRegister(frb)]),
            ))
        }

        // Extended opcode 33: Floating-point square root single (fsqrts)
        // Format: fsqrts FRT, FRB
        // Only if primary opcode is 63
        33 if (word >> 26) == 63 => {
            let frt: u8 = ((word >> 21) & 0x1F) as u8;
            let frb: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::FloatingPoint,
                SmallVec::from_slice(&[Operand::FpRegister(frt), Operand::FpRegister(frb)]),
            ))
        }

        // Extended opcode 38: Floating-point select (fsel)
        // Format: fsel FRT, FRA, FRC, FRB (FRT = FRA >= 0 ? FRC : FRB)
        // Only if primary opcode is 63
        38 if (word >> 26) == 63 => {
            let frt: u8 = ((word >> 21) & 0x1F) as u8;
            let fra: u8 = ((word >> 16) & 0x1F) as u8;
            let frc: u8 = ((word >> 6) & 0x1F) as u8;
            let frb: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::FloatingPoint,
                SmallVec::from_slice(&[
                    Operand::FpRegister(frt),
                    Operand::FpRegister(fra),
                    Operand::FpRegister(frc),
                    Operand::FpRegister(frb),
                ]),
            ))
        }

        // Extended opcode 72: Floating-point move register (fmr)
        // Format: fmr FRT, FRB
        // Only if primary opcode is 63
        72 if (word >> 26) == 63 => {
            let frt: u8 = ((word >> 21) & 0x1F) as u8;
            let frb: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::FloatingPoint,
                SmallVec::from_slice(&[Operand::FpRegister(frt), Operand::FpRegister(frb)]),
            ))
        }

        // Extended opcode 583: Floating-point move from integer word (fctiw)
        // Format: fctiw FRT, FRB (convert integer word to FP)
        // Only if primary opcode is 63
        583 if (word >> 26) == 63 => {
            let frt: u8 = ((word >> 21) & 0x1F) as u8;
            let frb: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::FloatingPoint,
                SmallVec::from_slice(&[Operand::FpRegister(frt), Operand::FpRegister(frb)]),
            ))
        }

        // Extended opcode 711: Floating-point move from integer word zero (fctiwz)
        // Format: fctiwz FRT, FRB (convert integer word to FP, zero upper)
        // Only if primary opcode is 63
        711 if (word >> 26) == 63 => {
            let frt: u8 = ((word >> 21) & 0x1F) as u8;
            let frb: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::FloatingPoint,
                SmallVec::from_slice(&[Operand::FpRegister(frt), Operand::FpRegister(frb)]),
            ))
        }

        // Extended opcode 815: Floating-point move to integer word zero (fctiwz)
        // Format: fctiwz FRT, FRB (convert FP to integer word, round toward zero)
        // Only if primary opcode is 63
        815 if (word >> 26) == 63 => {
            let frt: u8 = ((word >> 21) & 0x1F) as u8;
            let frb: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::FloatingPoint,
                SmallVec::from_slice(&[Operand::FpRegister(frt), Operand::FpRegister(frb)]),
            ))
        }

        // Extended opcode 70: Floating-point move to condition register (mffs)
        // Format: mffs FRT (move FPSCR to FRT)
        // Only if primary opcode is 63
        70 if (word >> 26) == 63 => {
            let frt: u8 = ((word >> 21) & 0x1F) as u8;
            Ok((
                InstructionType::FloatingPoint,
                SmallVec::from_slice(&[Operand::FpRegister(frt)]),
            ))
        }

        // Extended opcode 134: Floating-point move from condition register (mtfsf)
        // Format: mtfsf BF, FRB (move FRB to FPSCR field BF)
        // Only if primary opcode is 63
        134 if (word >> 26) == 63 => {
            let bf: u8 = ((word >> 23) & 0x7) as u8;
            let frb: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::FloatingPoint,
                SmallVec::from_slice(&[Operand::Condition(bf), Operand::FpRegister(frb)]),
            ))
        }

        // Extended opcode 711: Floating-point move from condition register field (mtfsfi)
        // Format: mtfsfi BF, IMM (move immediate to FPSCR field BF)
        // Only if primary opcode is 63
        711 if (word >> 26) == 63 && ((word >> 12) & 0x7) != 0 => {
            let bf: u8 = ((word >> 23) & 0x7) as u8;
            let imm: u8 = ((word >> 12) & 0xF) as u8;
            Ok((
                InstructionType::FloatingPoint,
                SmallVec::from_slice(&[Operand::Condition(bf), Operand::Immediate(imm as i16)]),
            ))
        }

        // Extended opcode 18: Floating-point divide (fdiv)
        // Format: fdiv FRT, FRA, FRB
        // Only if primary opcode is 63
        18 if (word >> 26) == 63 => {
            let frt: u8 = ((word >> 21) & 0x1F) as u8;
            let fra: u8 = ((word >> 16) & 0x1F) as u8;
            let frb: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::FloatingPoint,
                SmallVec::from_slice(&[
                    Operand::FpRegister(frt),
                    Operand::FpRegister(fra),
                    Operand::FpRegister(frb),
                ]),
            ))
        }

        // Extended opcode 0: Floating-point compare (fcmpu/fcmpo)
        // Format: fcmpu BF, FRA, FRB
        // Only if primary opcode is 63
        0 if (word >> 26) == 63 => {
            let bf: u8 = ((word >> 23) & 0x7) as u8;
            let fra: u8 = ((word >> 16) & 0x1F) as u8;
            let frb: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::FloatingPoint,
                SmallVec::from_slice(&[
                    Operand::Condition(bf),
                    Operand::FpRegister(fra),
                    Operand::FpRegister(frb),
                ]),
            ))
        }

        // Extended opcode 15: Floating-point convert to integer word (fctiw)
        // Format: fctiw FRT, FRB
        // Only if primary opcode is 63
        15 if (word >> 26) == 63 => {
            let frt: u8 = ((word >> 21) & 0x1F) as u8;
            let frb: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::FloatingPoint,
                SmallVec::from_slice(&[Operand::FpRegister(frt), Operand::FpRegister(frb)]),
            ))
        }

        // Extended opcode 31: Floating-point convert to integer word with round toward zero (fctiwz)
        // Format: fctiwz FRT, FRB
        // Only if primary opcode is 63
        31 if (word >> 26) == 63 => {
            let frt: u8 = ((word >> 21) & 0x1F) as u8;
            let frb: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::FloatingPoint,
                SmallVec::from_slice(&[Operand::FpRegister(frt), Operand::FpRegister(frb)]),
            ))
        }

        // Extended opcode 12: Floating-point round to single precision (frsp)
        // Format: frsp FRT, FRB
        // Only if primary opcode is 63
        12 if (word >> 26) == 63 => {
            let frt: u8 = ((word >> 21) & 0x1F) as u8;
            let frb: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::FloatingPoint,
                SmallVec::from_slice(&[Operand::FpRegister(frt), Operand::FpRegister(frb)]),
            ))
        }

        // Extended opcode 264: Floating-point absolute value (fabs)
        // Format: fabs FRT, FRB
        // Only if primary opcode is 63
        264 if (word >> 26) == 63 => {
            let frt: u8 = ((word >> 21) & 0x1F) as u8;
            let frb: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::FloatingPoint,
                SmallVec::from_slice(&[Operand::FpRegister(frt), Operand::FpRegister(frb)]),
            ))
        }

        // Extended opcode 136: Floating-point negative absolute value (fnabs)
        // Format: fnabs FRT, FRB
        // Only if primary opcode is 63
        136 if (word >> 26) == 63 => {
            let frt: u8 = ((word >> 21) & 0x1F) as u8;
            let frb: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::FloatingPoint,
                SmallVec::from_slice(&[Operand::FpRegister(frt), Operand::FpRegister(frb)]),
            ))
        }

        // Extended opcode 40: Floating-point negate (fneg)
        // Format: fneg FRT, FRB
        // Only if primary opcode is 63
        40 if (word >> 26) == 63 => {
            let frt: u8 = ((word >> 21) & 0x1F) as u8;
            let frb: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::FloatingPoint,
                SmallVec::from_slice(&[Operand::FpRegister(frt), Operand::FpRegister(frb)]),
            ))
        }

        // Extended opcode 339: Move from special-purpose register (mfspr)
        // Format: mfspr RT, SPR
        // SPR encoding: ((SPR[0:4] << 5) | SPR[5:9])
        339 => {
            let rt: u8 = ((word >> 21) & 0x1F) as u8;
            let spr: u16 = (((word >> 16) & 0x1F) << 5) | ((word >> 11) & 0x1F);
            Ok((
                InstructionType::System,
                SmallVec::from_slice(&[Operand::Register(rt), Operand::SpecialRegister(spr)]),
            ))
        }

        // Extended opcode 467: Move to special-purpose register (mtspr)
        // Format: mtspr SPR, RS
        467 => {
            let rs: u8 = ((word >> 21) & 0x1F) as u8;
            let spr: u16 = (((word >> 16) & 0x1F) << 5) | ((word >> 11) & 0x1F);
            Ok((
                InstructionType::System,
                SmallVec::from_slice(&[Operand::Register(rs), Operand::SpecialRegister(spr)]),
            ))
        }

        // Extended opcode 19: Move from condition register (mfcr)
        // Format: mfcr RT
        // Only if primary opcode is 31 (not 63)
        19 if (word >> 26) == 31 => {
            let rt: u8 = ((word >> 21) & 0x1F) as u8;
            Ok((
                InstructionType::ConditionRegister,
                SmallVec::from_slice(&[Operand::Register(rt)]),
            ))
        }

        // Extended opcode 83: Move from condition register field (mfcrf)
        // Format: mfcrf RT, CRM (move specific CR field)
        83 => {
            let rt: u8 = ((word >> 21) & 0x1F) as u8;
            let crm: u8 = ((word >> 12) & 0xFF) as u8;
            Ok((
                InstructionType::ConditionRegister,
                SmallVec::from_slice(&[Operand::Register(rt), Operand::Condition(crm)]),
            ))
        }

        // Extended opcode 144: Move to condition register (mtcr)
        // Format: mtcr RS
        // Only if primary opcode is 31 (not 63)
        144 if (word >> 26) == 31 => {
            let rs: u8 = ((word >> 21) & 0x1F) as u8;
            Ok((
                InstructionType::ConditionRegister,
                SmallVec::from_slice(&[Operand::Register(rs)]),
            ))
        }

        // Extended opcode 146: Move to condition register field (mtcrf)
        // Format: mtcrf CRM, RS (move to specific CR field)
        146 => {
            let rs: u8 = ((word >> 21) & 0x1F) as u8;
            let crm: u8 = ((word >> 12) & 0xFF) as u8;
            Ok((
                InstructionType::ConditionRegister,
                SmallVec::from_slice(&[Operand::Register(rs), Operand::Condition(crm)]),
            ))
        }

        // Extended opcode 210: Move from XER (mfxer)
        // Format: mfxer RT
        210 => {
            let rt: u8 = ((word >> 21) & 0x1F) as u8;
            Ok((
                InstructionType::System,
                SmallVec::from_slice(&[Operand::Register(rt)]),
            ))
        }

        // Extended opcode 242: Move to XER (mtxer)
        // Format: mtxer RS
        242 => {
            let rs: u8 = ((word >> 21) & 0x1F) as u8;
            Ok((
                InstructionType::System,
                SmallVec::from_slice(&[Operand::Register(rs)]),
            ))
        }

        // Extended opcode 512: Move from link register (mflr)
        // Format: mflr RT
        512 => {
            let rt: u8 = ((word >> 21) & 0x1F) as u8;
            Ok((
                InstructionType::Move,
                SmallVec::from_slice(&[Operand::Register(rt)]),
            ))
        }

        // Extended opcode 576: Move to link register (mtlr)
        // Format: mtlr RS
        576 => {
            let rs: u8 = ((word >> 21) & 0x1F) as u8;
            Ok((
                InstructionType::Move,
                SmallVec::from_slice(&[Operand::Register(rs)]),
            ))
        }

        // Extended opcode 528: Move from count register (mfctr)
        // Format: mfctr RT
        528 if (word >> 26) == 31 => {
            let rt: u8 = ((word >> 21) & 0x1F) as u8;
            Ok((
                InstructionType::Move,
                SmallVec::from_slice(&[Operand::Register(rt)]),
            ))
        }

        // Extended opcode 592: Move to count register (mtctr)
        // Format: mtctr RS
        592 => {
            let rs: u8 = ((word >> 21) & 0x1F) as u8;
            Ok((
                InstructionType::Move,
                SmallVec::from_slice(&[Operand::Register(rs)]),
            ))
        }

        // Extended opcode 257: Condition register AND (crand)
        // Format: crand BT, BA, BB
        257 => {
            let bt: u8 = ((word >> 21) & 0x1F) as u8;
            let ba: u8 = ((word >> 16) & 0x1F) as u8;
            let bb: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::ConditionRegister,
                SmallVec::from_slice(&[
                    Operand::Condition(bt),
                    Operand::Condition(ba),
                    Operand::Condition(bb),
                ]),
            ))
        }

        // Extended opcode 449: Condition register OR (cror)
        // Format: cror BT, BA, BB
        449 => {
            let bt: u8 = ((word >> 21) & 0x1F) as u8;
            let ba: u8 = ((word >> 16) & 0x1F) as u8;
            let bb: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::ConditionRegister,
                SmallVec::from_slice(&[
                    Operand::Condition(bt),
                    Operand::Condition(ba),
                    Operand::Condition(bb),
                ]),
            ))
        }

        // Extended opcode 193: Condition register XOR (crxor)
        // Format: crxor BT, BA, BB
        193 => {
            let bt: u8 = ((word >> 21) & 0x1F) as u8;
            let ba: u8 = ((word >> 16) & 0x1F) as u8;
            let bb: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::ConditionRegister,
                SmallVec::from_slice(&[
                    Operand::Condition(bt),
                    Operand::Condition(ba),
                    Operand::Condition(bb),
                ]),
            ))
        }

        // Extended opcode 225: Condition register NAND (crnand)
        // Format: crnand BT, BA, BB
        225 => {
            let bt: u8 = ((word >> 21) & 0x1F) as u8;
            let ba: u8 = ((word >> 16) & 0x1F) as u8;
            let bb: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::ConditionRegister,
                SmallVec::from_slice(&[
                    Operand::Condition(bt),
                    Operand::Condition(ba),
                    Operand::Condition(bb),
                ]),
            ))
        }

        // Extended opcode 33: Condition register NOR (crnor)
        // Format: crnor BT, BA, BB
        33 => {
            let bt: u8 = ((word >> 21) & 0x1F) as u8;
            let ba: u8 = ((word >> 16) & 0x1F) as u8;
            let bb: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::ConditionRegister,
                SmallVec::from_slice(&[
                    Operand::Condition(bt),
                    Operand::Condition(ba),
                    Operand::Condition(bb),
                ]),
            ))
        }

        // Extended opcode 289: Condition register equivalent (creqv)
        // Format: creqv BT, BA, BB
        289 => {
            let bt: u8 = ((word >> 21) & 0x1F) as u8;
            let ba: u8 = ((word >> 16) & 0x1F) as u8;
            let bb: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::ConditionRegister,
                SmallVec::from_slice(&[
                    Operand::Condition(bt),
                    Operand::Condition(ba),
                    Operand::Condition(bb),
                ]),
            ))
        }

        // Extended opcode 129: Condition register AND with complement (crandc)
        // Format: crandc BT, BA, BB
        129 => {
            let bt: u8 = ((word >> 21) & 0x1F) as u8;
            let ba: u8 = ((word >> 16) & 0x1F) as u8;
            let bb: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::ConditionRegister,
                SmallVec::from_slice(&[
                    Operand::Condition(bt),
                    Operand::Condition(ba),
                    Operand::Condition(bb),
                ]),
            ))
        }

        // Extended opcode 417: Condition register OR with complement (crorc)
        // Format: crorc BT, BA, BB
        417 => {
            let bt: u8 = ((word >> 21) & 0x1F) as u8;
            let ba: u8 = ((word >> 16) & 0x1F) as u8;
            let bb: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::ConditionRegister,
                SmallVec::from_slice(&[
                    Operand::Condition(bt),
                    Operand::Condition(ba),
                    Operand::Condition(bb),
                ]),
            ))
        }

        // Cache control instructions (system instructions)
        // Extended opcode 86: Data cache block flush (dcbf)
        // Format: dcbf RA, RB
        86 => Ok((
            InstructionType::System,
            SmallVec::from_slice(&[Operand::Register(ra), Operand::Register(rb)]),
        )),
        // Extended opcode 54: Data cache block store (dcbst)
        // Format: dcbst RA, RB
        54 => Ok((
            InstructionType::System,
            SmallVec::from_slice(&[Operand::Register(ra), Operand::Register(rb)]),
        )),
        // Extended opcode 278: Data cache block touch (dcbt)
        // Format: dcbt RA, RB
        278 => Ok((
            InstructionType::System,
            SmallVec::from_slice(&[Operand::Register(ra), Operand::Register(rb)]),
        )),
        // Extended opcode 246: Data cache block touch for store (dcbtst)
        // Format: dcbtst RA, RB
        246 => Ok((
            InstructionType::System,
            SmallVec::from_slice(&[Operand::Register(ra), Operand::Register(rb)]),
        )),
        // Extended opcode 1014: Data cache block set to zero (dcbz)
        // Format: dcbz RA, RB
        1014 => Ok((
            InstructionType::System,
            SmallVec::from_slice(&[Operand::Register(ra), Operand::Register(rb)]),
        )),
        // Extended opcode 470: Instruction cache block invalidate (icbi)
        // Format: icbi RA, RB
        470 => Ok((
            InstructionType::System,
            SmallVec::from_slice(&[Operand::Register(ra), Operand::Register(rb)]),
        )),

        // Memory synchronization instructions
        // Extended opcode 598: Synchronize (sync)
        598 => Ok((InstructionType::System, SmallVec::new())),
        // Extended opcode 150: Instruction synchronize (isync)
        150 => Ok((InstructionType::System, SmallVec::new())),
        // Extended opcode 854: Enforce in-order execution of I/O (eieio)
        854 => Ok((InstructionType::System, SmallVec::new())),

        // String operations (rare on GameCube, but included for completeness)
        // Extended opcode 597: Load string word immediate (lswi)
        // Format: lswi RT, RA, NB - loads NB bytes starting at RA into RT, RT+1, ...
        // Note: This conflicts with lmw, but lswi uses different encoding
        // Extended opcode 533: Store string word immediate (stswi)
        // Format: stswi RS, RA, NB - stores NB bytes from RS, RS+1, ... starting at RA
        // Note: This conflicts with stmw, but stswi uses different encoding
        // Extended opcode 534: Load string word indexed (lswx)
        // Format: lswx RT, RA, RB - loads bytes starting at RA+RB into RT, RT+1, ...
        // Only if primary opcode is 31 (not 63)
        534 if (word >> 26) == 31 => Ok((
            InstructionType::Load,
            SmallVec::from_slice(&[
                Operand::Register(rt),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),
        // Extended opcode 662: Store string word indexed (stswx)
        // Format: stswx RS, RA, RB - stores bytes from RS, RS+1, ... starting at RA+RB
        // Only if primary opcode is 31 (not 63)
        662 if (word >> 26) == 31 => Ok((
            InstructionType::Store,
            SmallVec::from_slice(&[
                Operand::Register(rs),
                Operand::Register(ra),
                Operand::Register(rb),
            ]),
        )),

        // Extended opcode 597: Load string word immediate (lswi)
        // Format: lswi RT, RA, NB - loads NB bytes starting at RA into RT, RT+1, ...
        // Only if primary opcode is 31 (not 63)
        597 if (word >> 26) == 31 => {
            let rt: u8 = ((word >> 21) & 0x1F) as u8;
            let ra: u8 = ((word >> 16) & 0x1F) as u8;
            let nb: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::Load,
                SmallVec::from_slice(&[
                    Operand::Register(rt),
                    Operand::Register(ra),
                    Operand::Immediate(nb as i16),
                ]),
            ))
        }

        // Extended opcode 533: Store string word immediate (stswi)
        // Format: stswi RS, RA, NB - stores NB bytes from RS, RS+1, ... starting at RA
        // Only if primary opcode is 31 (not 63)
        533 if (word >> 26) == 31 => {
            let rs: u8 = ((word >> 21) & 0x1F) as u8;
            let ra: u8 = ((word >> 16) & 0x1F) as u8;
            let nb: u8 = ((word >> 11) & 0x1F) as u8;
            Ok((
                InstructionType::Store,
                SmallVec::from_slice(&[
                    Operand::Register(rs),
                    Operand::Register(ra),
                    Operand::Immediate(nb as i16),
                ]),
            ))
        }

        // TLB management instructions (system-level, rare)
        // Extended opcode 306: TLB invalidate entry (tlbie)
        // Format: tlbie RA, RB
        306 => Ok((
            InstructionType::System,
            SmallVec::from_slice(&[Operand::Register(ra), Operand::Register(rb)]),
        )),
        // Extended opcode 566: TLB synchronize (tlbsync)
        // Format: tlbsync
        566 => Ok((InstructionType::System, SmallVec::new())),

        // Unknown extended opcode
        _ => Ok((InstructionType::Unknown, SmallVec::new())),
    }
}

/// Compute rotate mask from MB (mask begin) and ME (mask end) fields.
///
/// The mask is used in rotate-and-mask instructions (rlwinm, rlwnm, etc.).
/// MB and ME are 5-bit fields specifying which bits to preserve.
///
/// # Arguments
/// * `mb` - Mask begin (5 bits, 0-31)
/// * `me` - Mask end (5 bits, 0-31)
///
/// # Returns
/// `u32` - 32-bit mask with bits MB through ME set
///
/// # Algorithm
/// If MB <= ME: set bits MB through ME (inclusive)
/// If MB > ME: wraparound case - set bits 0 through ME and MB through 31
#[inline] // Called frequently for rotate instructions
fn compute_mask(mb: u8, me: u8) -> u32 {
    let mut mask: u32 = 0u32;

    if mb <= me {
        // Normal case: set bits MB through ME (inclusive)
        for i in mb..=me {
            mask |= 1u32 << (31u32 - i as u32);
        }
    } else {
        // Wraparound case: set bits 0 through ME and MB through 31
        for i in 0u8..=me {
            mask |= 1u32 << (31u32 - i as u32);
        }
        for i in mb..32u8 {
            mask |= 1u32 << (31u32 - i as u32);
        }
    }

    mask
}
