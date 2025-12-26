//! Integration tests for instruction decoding

use gcrecomp_core::recompiler::decoder::Instruction;

#[test]
fn test_decode_common_instructions() {
    // Test addi instruction (opcode 14)
    let word: u32 = 0x38600001; // addi r3, r0, 1
    let result = Instruction::decode(word, 0x80000000);
    assert!(result.is_ok(), "Should decode addi instruction");
    
    let decoded = result.unwrap();
    assert_eq!(decoded.address, 0x80000000);
    assert_eq!(decoded.instruction.opcode, 14);
}

#[test]
fn test_decode_branch_instruction() {
    // Test branch instruction (opcode 18)
    let word: u32 = 0x48000000; // b 0 (relative branch)
    let result = Instruction::decode(word, 0x80001000);
    assert!(result.is_ok(), "Should decode branch instruction");
    
    let decoded = result.unwrap();
    assert_eq!(decoded.address, 0x80001000);
    assert_eq!(decoded.instruction.opcode, 18);
}

#[test]
fn test_decode_load_instruction() {
    // Test lwz instruction (opcode 32)
    let word: u32 = 0x80030000; // lwz r0, 0(r3)
    let result = Instruction::decode(word, 0x80002000);
    assert!(result.is_ok(), "Should decode load instruction");
    
    let decoded = result.unwrap();
    assert_eq!(decoded.address, 0x80002000);
    assert_eq!(decoded.instruction.opcode, 32);
}

#[test]
fn test_decode_store_instruction() {
    // Test stw instruction (opcode 36)
    let word: u32 = 0x90030000; // stw r0, 0(r3)
    let result = Instruction::decode(word, 0x80003000);
    assert!(result.is_ok(), "Should decode store instruction");
    
    let decoded = result.unwrap();
    assert_eq!(decoded.address, 0x80003000);
    assert_eq!(decoded.instruction.opcode, 36);
}

#[test]
fn test_decode_extended_opcode() {
    // Test add instruction (opcode 31, extended opcode 266)
    let word: u32 = 0x7C030214; // add r0, r3, r4
    let result = Instruction::decode(word, 0x80004000);
    assert!(result.is_ok(), "Should decode extended opcode instruction");
    
    let decoded = result.unwrap();
    assert_eq!(decoded.address, 0x80004000);
    assert_eq!(decoded.instruction.opcode, 31);
}

