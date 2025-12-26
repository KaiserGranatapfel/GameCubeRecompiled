//! Unit tests for code generation

use gcrecomp_core::recompiler::analysis::FunctionMetadata;
use gcrecomp_core::recompiler::codegen::CodeGenerator;
use gcrecomp_core::recompiler::decoder::{
    DecodedInstruction, Instruction, InstructionType, Operand,
};
use smallvec::SmallVec;

fn create_test_instruction(opcode: u8, inst_type: InstructionType) -> DecodedInstruction {
    DecodedInstruction {
        instruction: Instruction {
            opcode,
            instruction_type: inst_type,
            operands: SmallVec::new(),
            raw: (opcode as u32) << 26,
        },
        address: 0x80000000,
        raw: (opcode as u32) << 26,
    }
}

#[test]
fn test_codegen_initialization() {
    let codegen = CodeGenerator::new();
    assert!(
        codegen.optimize,
        "Should have optimizations enabled by default"
    );
}

#[test]
fn test_codegen_with_optimizations() {
    let codegen = CodeGenerator::new().with_optimizations(true);
    assert!(codegen.optimize, "Should respect optimization setting");
}

#[test]
fn test_codegen_without_optimizations() {
    let codegen = CodeGenerator::new().with_optimizations(false);
    assert!(
        !codegen.optimize,
        "Should disable optimizations when requested"
    );
}

#[test]
fn test_generate_function_empty() {
    let mut codegen = CodeGenerator::new();
    let metadata = FunctionMetadata {
        address: 0x80000000,
        name: "test_function".to_string(),
        size: 0,
        calling_convention: "cdecl".to_string(),
        parameters: vec![],
        return_type: None,
        local_variables: vec![],
        basic_blocks: vec![],
    };

    let instructions = vec![];
    let result = codegen.generate_function(&metadata, &instructions);

    // Should generate at least function signature
    assert!(
        result.is_ok(),
        "Should generate function even with no instructions"
    );
    let code = result.unwrap();
    assert!(
        code.contains("test_function"),
        "Should include function name"
    );
    assert!(code.contains("pub fn"), "Should be a public function");
}

#[test]
fn test_sanitize_identifier() {
    let codegen = CodeGenerator::new();

    assert_eq!(
        codegen.sanitize_identifier("test_function"),
        "test_function"
    );
    assert_eq!(
        codegen.sanitize_identifier("test-function"),
        "test_function"
    );
    assert_eq!(
        codegen.sanitize_identifier("test.function"),
        "test_function"
    );
    assert_eq!(
        codegen.sanitize_identifier("test function"),
        "test_function"
    );
}
