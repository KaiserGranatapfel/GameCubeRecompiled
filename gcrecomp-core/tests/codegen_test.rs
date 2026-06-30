//! Unit tests for code generation

use gcrecomp_core::recompiler::analysis::FunctionMetadata;
use gcrecomp_core::recompiler::codegen::CodeGenerator;
use gcrecomp_core::recompiler::decoder::{DecodedInstruction, Instruction, InstructionType};
use smallvec::SmallVec;

fn _create_test_instruction(opcode: u32, inst_type: InstructionType) -> DecodedInstruction {
    DecodedInstruction {
        instruction: Instruction {
            opcode,
            instruction_type: inst_type,
            operands: SmallVec::new(),
        },
        address: 0x80000000,
        raw: opcode << 26,
    }
}

#[test]
fn test_codegen_initialization() {
    // Just verify that codegen can be created
    let _codegen = CodeGenerator::new();
}

#[test]
fn test_codegen_with_optimizations() {
    let _codegen = CodeGenerator::new().with_optimizations(true);
}

#[test]
fn test_codegen_without_optimizations() {
    let _codegen = CodeGenerator::new().with_optimizations(false);
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

/// Generate a full function from raw PowerPC instruction words.
fn gen(words: &[u32]) -> String {
    let mut cg = CodeGenerator::new();
    let instrs: Vec<DecodedInstruction> = words
        .iter()
        .enumerate()
        .map(|(i, &w)| Instruction::decode(w, 0x8000_3000 + (i as u32) * 4).unwrap())
        .collect();
    let md = FunctionMetadata {
        address: 0x8000_3000,
        name: "f".to_string(),
        size: (words.len() * 4) as u32,
        calling_convention: "default".to_string(),
        parameters: vec![],
        return_type: None,
        local_variables: vec![],
        basic_blocks: vec![],
    };
    cg.generate_function(&md, &instrs).unwrap()
}

#[test]
fn test_bl_generates_real_call() {
    // bl +0x10 ; blr  — opcode 18 with LK set must dispatch a real call, not a stub.
    let code = gen(&[0x4800_0011, 0x4E80_0020]);
    assert!(
        code.contains("call_function_by_address"),
        "bl must emit a dispatcher call:\n{code}"
    );
    assert!(!code.contains("untranslated"), "no stubs:\n{code}");
}

#[test]
fn test_fp_load_store_arith_translate() {
    // lfs f1,8(r3) ; fadds f1,f2,f3 ; stfs f1,8(r3) ; blr
    let code = gen(&[0xC023_0008, 0xEC22_182A, 0xD023_0008, 0x4E80_0020]);
    assert!(code.contains("set_fpr"), "lfs/fadds set an FPR:\n{code}");
    assert!(code.contains("read_u32"), "lfs reads memory:\n{code}");
    assert!(code.contains("write_u32"), "stfs writes memory:\n{code}");
    assert!(
        !code.contains("untranslated"),
        "FP must not be stubbed:\n{code}"
    );
}

#[test]
fn test_mulli_translates_to_multiply() {
    // mulli r3,r4,3 ; stw r3,0(r5) ; blr — opcode 7 must be a real multiply.
    // The store consumes r3 so dead-code elimination keeps the multiply.
    let code = gen(&[0x1C64_0003, 0x9065_0000, 0x4E80_0020]);
    assert!(
        code.contains("wrapping_mul"),
        "mulli is a multiply:\n{code}"
    );
    assert!(!code.contains("untranslated"), "no stubs:\n{code}");
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
