//! Tests for instruction decoding edge cases

use gcrecomp_core::recompiler::decoder::{Instruction, DecodedInstruction};
use anyhow::Result;

#[test]
fn test_invalid_opcode() {
    // Test handling of invalid opcode
    let word = 0xFFFFFFFF; // All ones - likely data, not code
    let result = Instruction::decode(word, 0x80000000);
    
    // Should decode as Unknown, not panic
    assert!(result.is_ok());
    if let Ok(decoded) = result {
        assert_eq!(decoded.instruction.instruction_type, 
                   gcrecomp_core::recompiler::decoder::InstructionType::Unknown);
    }
}

#[test]
fn test_zero_instruction() {
    // Test handling of zero instruction (likely data)
    let word = 0x00000000;
    let result = Instruction::decode(word, 0x80000000);
    
    // Should decode, but may be marked as data
    assert!(result.is_ok());
}

#[test]
fn test_invalid_register() {
    // Test that decoder validates register numbers
    // This is handled in validation, not decoding
    // Decoder should still decode, validation catches it
    let word = 0x3864002A; // addi r3, r4, 42 (valid)
    let result = Instruction::decode(word, 0x80000000);
    assert!(result.is_ok());
}

#[test]
fn test_overlapping_instructions() {
    // PowerPC has fixed 32-bit instructions, so overlapping shouldn't occur
    // But test the detection function
    use gcrecomp_core::recompiler::decoder::detect_overlapping_instructions;
    
    let instructions = vec![
        DecodedInstruction {
            instruction: Instruction {
                opcode: 14,
                instruction_type: gcrecomp_core::recompiler::decoder::InstructionType::Arithmetic,
                operands: smallvec::smallvec![],
            },
            raw: 0x3864002A,
            address: 0x80000000,
        },
        DecodedInstruction {
            instruction: Instruction {
                opcode: 14,
                instruction_type: gcrecomp_core::recompiler::decoder::InstructionType::Arithmetic,
                operands: smallvec::smallvec![],
            },
            raw: 0x3864002B,
            address: 0x80000004, // Normal spacing
        },
    ];
    
    let overlaps = detect_overlapping_instructions(&instructions);
    assert_eq!(overlaps.len(), 0); // Should be no overlaps
}

