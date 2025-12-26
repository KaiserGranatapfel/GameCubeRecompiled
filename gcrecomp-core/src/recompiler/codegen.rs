// Rust code generator with optimizations
use anyhow::{Result, Context};
use crate::recompiler::decoder::{DecodedInstruction, InstructionType, Operand};
use crate::recompiler::analysis::FunctionMetadata;
use std::collections::HashMap;

pub struct CodeGenerator {
    indent_level: usize,
    register_map: HashMap<u8, String>,
    next_temp: usize,
    register_values: HashMap<u8, RegisterValue>,
    label_counter: usize,
    optimize: bool,
    function_calls: Vec<u32>, // Track function call targets
    basic_block_map: HashMap<u32, usize>, // Map addresses to basic block indices
}

#[derive(Debug, Clone)]
enum RegisterValue {
    Constant(u32),
    Variable(String),
    Unknown,
}

impl CodeGenerator {
    pub fn new() -> Self {
        Self {
            indent_level: 0,
            register_map: HashMap::new(),
            next_temp: 0,
            register_values: HashMap::new(),
            label_counter: 0,
            optimize: true,
            function_calls: Vec::new(),
            basic_block_map: HashMap::new(),
        }
    }

    pub fn with_optimizations(mut self, optimize: bool) -> Self {
        self.optimize = optimize;
        self
    }

    pub fn generate_function(
        &mut self,
        metadata: &FunctionMetadata,
        instructions: &[DecodedInstruction],
    ) -> Result<String> {
        let mut code = String::new();
        
        // Generate function signature
        code.push_str(&self.generate_function_signature(metadata)?);
        code.push_str(" {\n");
        
        self.indent_level += 1;
        
        // Generate function body
        code.push_str(&self.generate_function_body(instructions)?);
        
        self.indent_level -= 1;
        code.push_str("}\n");
        
        Ok(code)
    }

    fn generate_function_signature(&self, metadata: &FunctionMetadata) -> Result<String> {
        let mut sig = String::new();
        
        // Function name - include address for uniqueness and dispatcher matching
        let func_name = if metadata.name.is_empty() || metadata.name.starts_with("sub_") {
            format!("func_0x{:08X}", metadata.address)
        } else {
            format!("{}_{:08X}", self.sanitize_identifier(&metadata.name), metadata.address)
        };
        
        sig.push_str("pub fn ");
        sig.push_str(&func_name);
        sig.push_str("(");
        
        // Standard function signature: ctx and memory (PowerPC calling convention)
        sig.push_str("ctx: &mut CpuContext, memory: &mut MemoryManager");
        
        // Note: Parameters are passed via registers (r3-r10) in PowerPC calling convention
        // They're already in ctx when the function is called, so we don't need explicit parameters
        
        sig.push_str(") -> Result<Option<u32>>");
        
        Ok(sig)
    }

    fn generate_function_body(&mut self, instructions: &[DecodedInstruction]) -> Result<String> {
        // Use control flow analysis to generate better code
        let cfg = crate::recompiler::analysis::control_flow::ControlFlowAnalyzer::build_cfg(instructions, 0)
            .unwrap_or_else(|_| {
                // Fallback to basic block construction
                crate::recompiler::analysis::control_flow::ControlFlowGraph {
                    nodes: vec![],
                    edges: vec![],
                    entry_block: 0,
                }
            });
        
        // Use data flow analysis for optimizations
        let def_use_chains = crate::recompiler::analysis::data_flow::DataFlowAnalyzer::build_def_use_chains(instructions);
        let live_analysis = if !cfg.nodes.is_empty() {
            Some(crate::recompiler::analysis::data_flow::DataFlowAnalyzer::live_variable_analysis(&cfg))
        } else {
            None
        };
        
        // Optimize instructions using data flow analysis
        let optimized_instructions = if let Some(ref live) = live_analysis {
            crate::recompiler::analysis::data_flow::DataFlowAnalyzer::eliminate_dead_code(instructions, live)
        } else {
            instructions.to_vec()
        };
        
        self.generate_function_body_impl(&optimized_instructions)
    }
    
    fn generate_function_body_impl(&mut self, instructions: &[DecodedInstruction]) -> Result<String> {
        let mut code = String::new();
        
        // Note: ctx and memory are passed as parameters, no need to initialize
        // Parameters are already in registers r3-r10 per PowerPC calling convention
        // No need to load them explicitly - they're already in ctx when function is called
        
        code.push('\n');
        
        // Setup stack frame if function has local variables
        if !instructions.is_empty() {
            code.push_str(&self.indent());
            code.push_str("// Stack frame setup\n");
            code.push_str(&self.indent());
            code.push_str("let stack_base = ctx.get_register(1); // r1 is stack pointer\n");
            code.push('\n');
        }
        
        // Build control flow graph for better code generation
        let basic_blocks = self.build_basic_blocks(instructions)?;
        
        // Generate code for each basic block
        for (block_idx, block) in basic_blocks.iter().enumerate() {
            if block_idx > 0 {
                code.push_str(&self.indent());
                code.push_str(&format!("// Basic block {}\n", block_idx));
            }
            
            for (inst_idx, instruction) in block.iter().enumerate() {
                match self.generate_instruction(instruction) {
                    Ok(inst_code) => {
                        code.push_str(&inst_code);
                    }
                    Err(e) => {
                        // Comprehensive error recovery
                        code.push_str(&self.indent());
                        code.push_str(&format!(
                            "// Error generating instruction {}: {}\n",
                            inst_idx, e
                        ));
                        code.push_str(&self.indent());
                        code.push_str(&format!("// Raw instruction: 0x{:08X}\n", instruction.raw));
                        code.push_str(&self.indent());
                        code.push_str(&format!("// Instruction type: {:?}\n", instruction.instruction.instruction_type));
                        code.push_str(&self.indent());
                        code.push_str("// Fallback: generating generic instruction handler\n");
                        code.push_str(&self.indent());
                        code.push_str(&format!(
                            "// TODO: Implement proper handling for opcode 0x{:02X}\n",
                            instruction.instruction.opcode
                        ));
                        code.push_str(&self.indent());
                        code.push_str("// Continuing with next instruction...\n");
                        
                        // Try to generate at least something
                        if let Ok(fallback) = self.generate_generic(instruction) {
                            code.push_str(&fallback);
                        }
                    }
                }
            }
        }
        
        // Teardown stack frame
        if !instructions.is_empty() {
            code.push('\n');
            code.push_str(&self.indent());
            code.push_str("// Stack frame teardown\n");
            code.push_str(&self.indent());
            code.push_str("ctx.set_register(1, stack_base);\n");
        }
        
        // Return value (PowerPC calling convention: return value in r3)
        code.push_str(&self.indent());
        code.push_str("// Return value is in r3 (PowerPC calling convention)\n");
        code.push_str(&self.indent());
        code.push_str("Ok(Some(ctx.get_register(3)))\n");
        
        Ok(code)
    }
    
    fn build_basic_blocks(&self, instructions: &[DecodedInstruction]) -> Result<Vec<Vec<&DecodedInstruction>>> {
        // Simple basic block construction: split at branches
        let mut blocks = Vec::new();
        let mut current_block = Vec::new();
        
        for inst in instructions {
            current_block.push(inst);
            
            // If this is a branch, end the block
            if matches!(inst.instruction.instruction_type, InstructionType::Branch) {
                blocks.push(current_block);
                current_block = Vec::new();
            }
        }
        
        // Add remaining instructions as final block
        if !current_block.is_empty() {
            blocks.push(current_block);
        }
        
        Ok(blocks)
    }

    fn generate_instruction(&mut self, inst: &DecodedInstruction) -> Result<String> {
        let mut code = String::new();
        
        // Add instruction address comment for debugging
        if self.optimize {
            code.push_str(&self.indent());
            code.push_str(&format!("// 0x{:08X}: ", inst.raw));
        }
        
        match inst.instruction.instruction_type {
            InstructionType::Arithmetic => {
                code.push_str(&self.generate_arithmetic(&inst.instruction)?);
            }
            InstructionType::Load => {
                code.push_str(&self.generate_load(&inst.instruction)?);
            }
            InstructionType::Store => {
                code.push_str(&self.generate_store(&inst.instruction)?);
            }
            InstructionType::Branch => {
                code.push_str(&self.generate_branch(&inst.instruction)?);
            }
            InstructionType::Compare => {
                code.push_str(&self.generate_compare(&inst.instruction)?);
            }
            InstructionType::Move => {
                code.push_str(&self.generate_move(&inst.instruction)?);
            }
            InstructionType::System => {
                code.push_str(&self.generate_system(&inst.instruction)?);
            }
            InstructionType::FloatingPoint => {
                code.push_str(&self.generate_floating_point(&inst.instruction)?);
            }
            InstructionType::ConditionRegister => {
                code.push_str(&self.generate_condition_register(&inst.instruction)?);
            }
            InstructionType::Shift => {
                code.push_str(&self.generate_shift(&inst.instruction)?);
            }
            InstructionType::Rotate => {
                code.push_str(&self.generate_rotate(&inst.instruction)?);
            }
            _ => {
                // Try to generate a generic instruction handler
                code.push_str(&self.generate_generic(&inst)?);
            }
        }
        
        Ok(code)
    }

    fn generate_arithmetic(&mut self, inst: &crate::recompiler::decoder::Instruction) -> Result<String> {
        let mut code = String::new();
        
        if inst.operands.len() < 2 {
            anyhow::bail!("Arithmetic instruction requires at least 2 operands");
        }
        
        let rt_reg = match &inst.operands[0] {
            Operand::Register(r) => *r,
            _ => anyhow::bail!("First operand must be a register"),
        };
        
        let ra_reg = match &inst.operands[1] {
            Operand::Register(r) => *r,
            _ => anyhow::bail!("Second operand must be a register"),
        };
        
        // Determine operation based on opcode and extended opcode
        let (op, update_cr) = match inst.opcode {
            14 => ("+", false),  // addi
            15 => ("-", false),   // subi
            12 => ("&", false),   // andi
            13 => ("|", false),   // ori
            10 => ("^", false),   // xori
            11 => ("&", false),   // andis
            31 => {
                // Extended opcode - decode from instruction
                let ext_opcode = (inst.raw >> 1) & 0x3FF;
                match ext_opcode {
                    266 => ("+", false),  // add
                    40 => ("-", false),   // subf
                    28 => ("&", false),   // and
                    444 => ("|", false),  // or
                    316 => ("^", false),  // xor
                    235 => ("*", false),  // mullw
                    233 => ("*", false),  // mulhw
                    104 => ("/", false),  // divw
                    536 => ("<<", false), // slw
                    824 => (">>", false), // srw
                    792 => (">>", false), // sraw
                    _ => ("+", false),
                }
            }
            _ => ("+", false),
        };
        
        // Get second operand (register or immediate)
        let (rb_expr, rb_value) = if inst.operands.len() > 2 {
            match &inst.operands[2] {
                Operand::Register(r) => {
                    let reg_val = self.get_register_value(*r);
                    (format!("ctx.get_register({})", r), reg_val)
                }
                Operand::Immediate(i) => {
                    let val = *i as u32;
                    (format!("{}u32", val), Some(RegisterValue::Constant(val)))
                }
                Operand::Immediate32(i) => {
                    let val = *i as u32;
                    (format!("{}u32", val), Some(RegisterValue::Constant(val)))
                }
                _ => ("0u32".to_string(), Some(RegisterValue::Constant(0))),
            }
        } else {
            ("0u32".to_string(), Some(RegisterValue::Constant(0)))
        };
        
        // Handle shift operations specially
        let operation_code = if op == "<<" || op == ">>" {
            if op == "<<" {
                format!("ctx.get_register({}) << ({} & 0x1F)", ra_reg, rb_expr)
            } else {
                format!("ctx.get_register({}) >> ({} & 0x1F)", ra_reg, rb_expr)
            }
        } else {
            format!("ctx.get_register({}) {} {}", ra_reg, op, rb_expr)
        };
        
        // Optimize: if both operands are constants, compute at compile time
        let ra_value = self.get_register_value(ra_reg);
        if let (Some(RegisterValue::Constant(a)), Some(RegisterValue::Constant(b))) = (ra_value, rb_value) {
            let result = match op {
                "+" => a.wrapping_add(b),
                "-" => a.wrapping_sub(b),
                "*" => a.wrapping_mul(b),
                "&" => a & b,
                "|" => a | b,
                "^" => a ^ b,
                "<<" => a << (b & 0x1F),
                ">>" => a >> (b & 0x1F),
                _ => a,
            };
            code.push_str(&self.indent());
            code.push_str(&format!(
                "ctx.set_register({}, {}u32); // Optimized: constant folding\n",
                rt_reg, result
            ));
            self.set_register_value(rt_reg, RegisterValue::Constant(result));
        } else {
            code.push_str(&self.indent());
            code.push_str(&format!(
                "ctx.set_register({}, {});\n",
                rt_reg, operation_code
            ));
            self.set_register_value(rt_reg, RegisterValue::Unknown);
        }
        
        // Update condition register if needed
        if update_cr {
            code.push_str(&self.indent());
            code.push_str(&format!(
                "let result = ctx.get_register({});\n",
                rt_reg
            ));
            code.push_str(&self.indent());
            code.push_str("let cr_field = if result == 0 { 0x2u8 } else if (result as i32) < 0 { 0x8u8 } else { 0x4u8 };\n");
            code.push_str(&self.indent());
            code.push_str("ctx.set_cr_field(0, cr_field);\n");
        }
        
        Ok(code)
    }
    
    fn get_register_value(&self, reg: u8) -> Option<RegisterValue> {
        self.register_values.get(&reg).cloned()
    }
    
    fn set_register_value(&mut self, reg: u8, value: RegisterValue) {
        self.register_values.insert(reg, value);
    }

    fn generate_load(&mut self, inst: &crate::recompiler::decoder::Instruction) -> Result<String> {
        let mut code = String::new();
        
        if inst.operands.len() < 3 {
            anyhow::bail!("Load instruction requires 3 operands");
        }
        
        let rt_reg = match &inst.operands[0] {
            Operand::Register(r) => *r,
            _ => anyhow::bail!("First operand must be a register"),
        };
        
        let ra_reg = match &inst.operands[1] {
            Operand::Register(r) => *r,
            _ => anyhow::bail!("Second operand must be a register"),
        };
        
        let offset = match &inst.operands[2] {
            Operand::Immediate(i) => *i as i32,
            _ => 0,
        };
        
        // Optimize: if base address is constant, compute address at compile time
        let base_value = self.get_register_value(ra_reg);
        if let Some(RegisterValue::Constant(base)) = base_value {
            let addr = base.wrapping_add(offset as u32);
            code.push_str(&self.indent());
            code.push_str(&format!(
                "let value = memory.read_u32(0x{:08X}u32).unwrap_or(0u32); // Optimized: constant address\n",
                addr
            ));
        } else {
            code.push_str(&self.indent());
            code.push_str(&format!(
                "let addr = ctx.get_register({}) as u32 + {}i32 as u32;\n",
                ra_reg, offset
            ));
            code.push_str(&self.indent());
            code.push_str("let value = memory.read_u32(addr).unwrap_or(0u32);\n");
        }
        
        code.push_str(&self.indent());
        code.push_str(&format!("ctx.set_register({}, value);\n", rt_reg));
        self.set_register_value(rt_reg, RegisterValue::Unknown);
        
        Ok(code)
    }

    fn generate_store(&mut self, inst: &crate::recompiler::decoder::Instruction) -> Result<String> {
        let mut code = String::new();
        
        if inst.operands.len() < 3 {
            anyhow::bail!("Store instruction requires 3 operands");
        }
        
        let rs_reg = match &inst.operands[0] {
            Operand::Register(r) => *r,
            _ => anyhow::bail!("First operand must be a register"),
        };
        
        let ra_reg = match &inst.operands[1] {
            Operand::Register(r) => *r,
            _ => anyhow::bail!("Second operand must be a register"),
        };
        
        let offset = match &inst.operands[2] {
            Operand::Immediate(i) => *i as i32,
            _ => 0,
        };
        
        // Optimize: if base address is constant, compute address at compile time
        let base_value = self.get_register_value(ra_reg);
        let value_expr = if let Some(RegisterValue::Constant(val)) = self.get_register_value(rs_reg) {
            format!("{}u32", val)
        } else {
            format!("ctx.get_register({})", rs_reg)
        };
        
        if let Some(RegisterValue::Constant(base)) = base_value {
            let addr = base.wrapping_add(offset as u32);
            code.push_str(&self.indent());
            code.push_str(&format!(
                "memory.write_u32(0x{:08X}u32, {}).unwrap_or(()); // Optimized: constant address\n",
                addr, value_expr
            ));
        } else {
            code.push_str(&self.indent());
            code.push_str(&format!(
                "let addr = ctx.get_register({}) as u32 + {}i32 as u32;\n",
                ra_reg, offset
            ));
            code.push_str(&self.indent());
            code.push_str(&format!("memory.write_u32(addr, {}).unwrap_or(());\n", value_expr));
        }
        
        Ok(code)
    }

    fn generate_branch(&mut self, inst: &crate::recompiler::decoder::Instruction) -> Result<String> {
        let mut code = String::new();
        
        if inst.operands.is_empty() {
            anyhow::bail!("Branch instruction requires operands");
        }
        
        // Handle different branch types
        match inst.operands.len() {
            1 => {
                // Unconditional branch (b, ba, bl, bla)
                let target = match &inst.operands[0] {
                    Operand::Immediate32(li) => *li,
                    Operand::Address(addr) => *addr,
                    _ => anyhow::bail!("Branch target must be immediate or address"),
                };
                
                // Check if this is a function call (bl/bla) or regular branch
                let is_call = inst.opcode == 18 && (inst.raw & 1) != 0; // Link bit set
                
                if is_call {
                    self.function_calls.push(target);
                    code.push_str(&self.indent());
                    code.push_str(&format!(
                        "// Function call to 0x{:08X}\n",
                        target
                    ));
                    code.push_str(&self.indent());
                    code.push_str("// Save return address in link register\n");
                    code.push_str(&self.indent());
                    code.push_str("let saved_lr = ctx.lr;\n");
                    code.push_str(&self.indent());
                    code.push_str("ctx.lr = ctx.pc + 4;\n");
                    code.push_str(&self.indent());
                    code.push_str("// Call recompiled function via dispatcher\n");
                    code.push_str(&self.indent());
                    code.push_str(&format!(
                        "match call_function_by_address(0x{:08X}u32, ctx, memory) {{\n",
                        target
                    ));
                    self.indent_level += 1;
                    code.push_str(&self.indent());
                    code.push_str("Ok(result) => {\n");
                    self.indent_level += 1;
                    code.push_str(&self.indent());
                    code.push_str("// Function call succeeded, result in r3 (PowerPC calling convention)\n");
                    code.push_str(&self.indent());
                    code.push_str("if let Some(ret_val) = result {\n");
                    self.indent_level += 1;
                    code.push_str(&self.indent());
                    code.push_str("ctx.set_register(3, ret_val); // Return value in r3\n");
                    self.indent_level -= 1;
                    code.push_str(&self.indent());
                    code.push_str("}\n");
                    code.push_str(&self.indent());
                    code.push_str("ctx.lr = saved_lr; // Restore link register\n");
                    self.indent_level -= 1;
                    code.push_str(&self.indent());
                    code.push_str("}\n");
                    code.push_str(&self.indent());
                    code.push_str("Err(e) => {\n");
                    self.indent_level += 1;
                    code.push_str(&self.indent());
                    code.push_str(&format!(
                        "log::warn!(\"Function call to 0x{:08X} failed: {{:?}}\", e);\n",
                        target
                    ));
                    code.push_str(&self.indent());
                    code.push_str("ctx.lr = saved_lr; // Restore link register\n");
                    self.indent_level -= 1;
                    code.push_str(&self.indent());
                    code.push_str("}\n");
                    self.indent_level -= 1;
                    code.push_str(&self.indent());
                    code.push_str("}\n");
                } else {
                    code.push_str(&self.indent());
                    code.push_str(&format!(
                        "ctx.pc = 0x{:08X}u32; // Unconditional branch\n",
                        target
                    ));
                    code.push_str(&self.indent());
                    code.push_str("return; // Branch out of function\n");
                }
            }
            3..=5 => {
                // Conditional branch (bc, bca, bcl, bcla)
                let bo = match &inst.operands[0] {
                    Operand::Condition(c) => *c,
                    _ => anyhow::bail!("First operand must be condition"),
                };
                
                let bi = if inst.operands.len() > 1 {
                    match &inst.operands[1] {
                        Operand::Condition(c) => *c,
                        _ => anyhow::bail!("Second operand must be condition"),
                    }
                } else {
                    0
                };
                
                let target = if inst.operands.len() > 2 {
                    match &inst.operands[2] {
                        Operand::Immediate(bd) => *bd as i32,
                        Operand::Address(addr) => *addr as i32,
                        _ => 0,
                    }
                } else {
                    0
                };
                
                let label = self.next_label();
                code.push_str(&self.indent());
                code.push_str(&format!(
                    "let cr_bit = (ctx.get_cr_field({}) >> {}) & 1;\n",
                    bi / 4, bi % 4
                ));
                code.push_str(&self.indent());
                code.push_str(&format!(
                    "if cr_bit != 0 {{\n"
                ));
                self.indent_level += 1;
                code.push_str(&self.indent());
                code.push_str(&format!(
                    "ctx.pc = ctx.pc + {}i32 as u32; // Conditional branch\n",
                    target
                ));
                code.push_str(&self.indent());
                code.push_str("return; // Branch taken\n");
                self.indent_level -= 1;
                code.push_str(&self.indent());
                code.push_str("}\n");
            }
            _ => {
                code.push_str(&self.indent());
                code.push_str("// Complex branch instruction\n");
            }
        }
        
        Ok(code)
    }
    
    fn next_label(&mut self) -> String {
        let label = format!("label_{}", self.label_counter);
        self.label_counter += 1;
        label
    }

    fn generate_compare(&mut self, inst: &crate::recompiler::decoder::Instruction) -> Result<String> {
        let mut code = String::new();
        
        if inst.operands.len() < 2 {
            anyhow::bail!("Compare instruction requires at least 2 operands");
        }
        
        let bf = match &inst.operands[0] {
            Operand::Condition(c) => *c,
            _ => 0, // Default to CR0
        };
        
        let ra_reg = match &inst.operands[1] {
            Operand::Register(r) => *r,
            _ => anyhow::bail!("Second operand must be a register"),
        };
        
        // Handle different compare types (cmpwi, cmplwi, cmpw, cmplw)
        let compare_value = if inst.operands.len() > 2 {
            match &inst.operands[2] {
                Operand::Register(rb) => {
                    format!("ctx.get_register({})", rb)
                }
                Operand::Immediate(i) => {
                    let val = *i as i32;
                    format!("{}i32", val)
                }
                _ => "0i32".to_string(),
            }
        } else {
            "0i32".to_string()
        };
        
        // Determine if unsigned comparison (cmplwi, cmplw)
        let is_unsigned = inst.opcode == 10; // cmplwi
        
        code.push_str(&self.indent());
        code.push_str(&format!(
            "let ra_val = ctx.get_register({}) as {};\n",
            ra_reg,
            if is_unsigned { "u32" } else { "i32" }
        ));
        code.push_str(&self.indent());
        code.push_str(&format!(
            "let rb_val = {} as {};\n",
            compare_value,
            if is_unsigned { "u32" } else { "i32" }
        ));
        
        // Set condition register field (LT, GT, EQ bits)
        code.push_str(&self.indent());
        code.push_str("let cr_field = if ra_val < rb_val {\n");
        self.indent_level += 1;
        code.push_str(&self.indent());
        code.push_str("0x8u8 // Less than\n");
        self.indent_level -= 1;
        code.push_str(&self.indent());
        code.push_str("} else if ra_val > rb_val {\n");
        self.indent_level += 1;
        code.push_str(&self.indent());
        code.push_str("0x4u8 // Greater than\n");
        self.indent_level -= 1;
        code.push_str(&self.indent());
        code.push_str("} else {\n");
        self.indent_level += 1;
        code.push_str(&self.indent());
        code.push_str("0x2u8 // Equal\n");
        self.indent_level -= 1;
        code.push_str(&self.indent());
        code.push_str("};\n");
        code.push_str(&self.indent());
        code.push_str(&format!("ctx.set_cr_field({}, cr_field);\n", bf));
        
        Ok(code)
    }

    fn generate_move(&mut self, inst: &crate::recompiler::decoder::Instruction) -> Result<String> {
        let mut code = String::new();
        
        if inst.operands.is_empty() {
            anyhow::bail!("Move instruction requires at least one operand");
        }
        
        // Handle move from/to link register (mflr/mtlr)
        if inst.operands.len() == 1 {
            let reg = match &inst.operands[0] {
                Operand::Register(r) => *r,
                _ => anyhow::bail!("Move operand must be a register"),
            };
            
            // Check if this is mflr (move from link register) or mtlr (move to link register)
            // This would be determined by the opcode, but for now we'll handle both
            code.push_str(&self.indent());
            code.push_str(&format!(
                "ctx.set_register({}, ctx.lr); // Move from/to link register\n",
                reg
            ));
            if let Some(lr_value) = self.get_register_value(reg) {
                // If we're moving LR, track it
            }
        }
        
        Ok(code)
    }

    fn generate_floating_point(&mut self, inst: &crate::recompiler::decoder::Instruction) -> Result<String> {
        let mut code = String::new();
        
        if inst.operands.is_empty() {
            anyhow::bail!("Floating point instruction requires operands");
        }
        
        // Handle different FP instruction types based on opcode
        match inst.operands.len() {
            3 => {
                // Binary operations (fadd, fsub, fmul, fdiv)
                let frt = match &inst.operands[0] {
                    Operand::FpRegister(r) => *r,
                    _ => anyhow::bail!("First operand must be FP register"),
                };
                let fra = match &inst.operands[1] {
                    Operand::FpRegister(r) => *r,
                    _ => anyhow::bail!("Second operand must be FP register"),
                };
                let frb = match &inst.operands[2] {
                    Operand::FpRegister(r) => *r,
                    _ => anyhow::bail!("Third operand must be FP register"),
                };
                
                // Determine operation based on extended opcode
                let ext_opcode = (inst.raw >> 1) & 0x3FF;
                let op = match ext_opcode {
                    21 => "+",  // fadd
                    20 => "-",  // fsub
                    25 => "*",  // fmul
                    18 => "/",  // fdiv
                    14 => "+",  // fmadd (FRA * FRC + FRB)
                    15 => "-",  // fmsub (FRA * FRC - FRB)
                    28 => "-",  // fnmadd (-(FRA * FRC + FRB))
                    29 => "-",  // fnmsub (-(FRA * FRC - FRB))
                    _ => "+",   // Default to add
                };
                
                // Handle multiply-add/subtract operations
                if ext_opcode == 14 || ext_opcode == 15 || ext_opcode == 28 || ext_opcode == 29 {
                    // These have 4 operands: FRT, FRA, FRC, FRB
                    if inst.operands.len() >= 4 {
                        let frc = match &inst.operands[2] {
                            Operand::FpRegister(r) => *r,
                            _ => anyhow::bail!("Third operand must be FP register for multiply-add"),
                        };
                        code.push_str(&self.indent());
                        code.push_str(&format!(
                            "let mul_result = ctx.get_fpr({}) * ctx.get_fpr({});\n",
                            fra, frc
                        ));
                        code.push_str(&self.indent());
                        if ext_opcode == 28 || ext_opcode == 29 {
                            code.push_str("let mul_result = -mul_result;\n");
                        }
                        code.push_str(&self.indent());
                        code.push_str(&format!(
                            "let result = mul_result {} ctx.get_fpr({});\n",
                            if ext_opcode == 15 || ext_opcode == 29 { "-" } else { "+" },
                            frb
                        ));
                        if ext_opcode == 29 {
                            code.push_str(&self.indent());
                            code.push_str("let result = -result;\n");
                        }
                    } else {
                        anyhow::bail!("Multiply-add/subtract requires 4 operands");
                    }
                } else {
                    code.push_str(&self.indent());
                    code.push_str(&format!(
                        "let result = ctx.get_fpr({}) {} ctx.get_fpr({});\n",
                        fra, op, frb
                    ));
                }
                code.push_str(&self.indent());
                code.push_str(&format!("ctx.set_fpr({}, result);\n", frt));
            }
            2 => {
                // Load/Store operations
                let frt = match &inst.operands[0] {
                    Operand::FpRegister(r) => *r,
                    _ => anyhow::bail!("First operand must be FP register"),
                };
                let ra = match &inst.operands[1] {
                    Operand::Register(r) => *r,
                    _ => anyhow::bail!("Second operand must be register"),
                };
                
                code.push_str(&self.indent());
                code.push_str(&format!(
                    "let addr = ctx.get_register({}) as u32;\n",
                    ra
                ));
                code.push_str(&self.indent());
                code.push_str(&format!(
                    "let value = f64::from_bits(memory.read_u64(addr).unwrap_or(0));\n"
                ));
                code.push_str(&self.indent());
                code.push_str(&format!("ctx.set_fpr({}, value);\n", frt));
            }
            _ => {
                code.push_str(&self.indent());
                code.push_str("// Complex floating point instruction\n");
            }
        }
        
        Ok(code)
    }

    fn generate_condition_register(&mut self, inst: &crate::recompiler::decoder::Instruction) -> Result<String> {
        let mut code = String::new();
        
        if inst.operands.len() == 1 {
            // Move from/to condition register
            let reg = match &inst.operands[0] {
                Operand::Register(r) => *r,
                _ => anyhow::bail!("Operand must be register"),
            };
            code.push_str(&self.indent());
            code.push_str(&format!(
                "ctx.set_register({}, ctx.cr); // Move from/to condition register\n",
                reg
            ));
        } else if inst.operands.len() == 3 {
            // CR logical operations (crand, cror, etc.)
            let bt = match &inst.operands[0] {
                Operand::Condition(c) => *c,
                _ => anyhow::bail!("First operand must be condition"),
            };
            let ba = match &inst.operands[1] {
                Operand::Condition(c) => *c,
                _ => anyhow::bail!("Second operand must be condition"),
            };
            let bb = match &inst.operands[2] {
                Operand::Condition(c) => *c,
                _ => anyhow::bail!("Third operand must be condition"),
            };
            
            code.push_str(&self.indent());
            code.push_str(&format!(
                "let cr_a = ctx.get_cr_field({});\n",
                ba / 4
            ));
            code.push_str(&self.indent());
            code.push_str(&format!(
                "let cr_b = ctx.get_cr_field({});\n",
                bb / 4
            ));
            // Determine operation based on extended opcode
            let ext_opcode = (inst.raw >> 1) & 0x3FF;
            let cr_op = match ext_opcode {
                257 => "&",   // crand
                449 => "|",   // cror
                193 => "^",   // crxor
                225 => "&",   // crnand (result = !(cr_a & cr_b))
                33 => "|",    // crnor (result = !(cr_a | cr_b))
                289 => "^",   // creqv (result = !(cr_a ^ cr_b))
                129 => "&",   // crandc (result = cr_a & !cr_b)
                417 => "|",   // crorc (result = cr_a | !cr_b)
                _ => "&",     // Default to AND
            };
            
            code.push_str(&self.indent());
            if ext_opcode == 225 || ext_opcode == 33 || ext_opcode == 289 {
                // NAND, NOR, or EQV - need to negate result
                code.push_str(&format!(
                    "let cr_result = !(ctx.get_cr_field({}) {} ctx.get_cr_field({}));\n",
                    ba / 4, cr_op, bb / 4
                ));
            } else if ext_opcode == 129 {
                // AND with complement
                code.push_str(&format!(
                    "let cr_result = ctx.get_cr_field({}) & !ctx.get_cr_field({});\n",
                    ba / 4, bb / 4
                ));
            } else if ext_opcode == 417 {
                // OR with complement
                code.push_str(&format!(
                    "let cr_result = ctx.get_cr_field({}) | !ctx.get_cr_field({});\n",
                    ba / 4, bb / 4
                ));
            } else {
                code.push_str(&format!(
                    "let cr_result = ctx.get_cr_field({}) {} ctx.get_cr_field({});\n",
                    ba / 4, cr_op, bb / 4
                ));
            }
            code.push_str(&self.indent());
            code.push_str(&format!(
                "ctx.set_cr_field({}, cr_result);\n",
                bt / 4
            ));
        }
        
        Ok(code)
    }

    fn generate_shift(&mut self, inst: &crate::recompiler::decoder::Instruction) -> Result<String> {
        let mut code = String::new();
        
        if inst.operands.len() < 3 {
            anyhow::bail!("Shift instruction requires at least 3 operands");
        }
        
        let rs = match &inst.operands[0] {
            Operand::Register(r) => *r,
            _ => anyhow::bail!("First operand must be register"),
        };
        let ra = match &inst.operands[1] {
            Operand::Register(r) => *r,
            _ => anyhow::bail!("Second operand must be register"),
        };
        let sh = match &inst.operands[2] {
            Operand::ShiftAmount(s) => *s,
            Operand::Register(r) => {
                // Shift amount from register
                code.push_str(&self.indent());
                code.push_str(&format!(
                    "let sh_amount = ctx.get_register({}) & 0x1F;\n",
                    r
                ));
                0 // Will use sh_amount variable
            }
            _ => anyhow::bail!("Third operand must be shift amount or register"),
        };
        
        // Determine shift direction (would need opcode check)
        code.push_str(&self.indent());
        if sh > 0 {
            code.push_str(&format!(
                "ctx.set_register({}, ctx.get_register({}) << {});\n",
                ra, rs, sh
            ));
        } else {
            code.push_str(&format!(
                "ctx.set_register({}, ctx.get_register({}) >> sh_amount);\n",
                ra, rs
            ));
        }
        
        Ok(code)
    }

    fn generate_rotate(&mut self, inst: &crate::recompiler::decoder::Instruction) -> Result<String> {
        let mut code = String::new();
        
        if inst.operands.len() < 4 {
            anyhow::bail!("Rotate instruction requires 4 operands");
        }
        
        let rs = match &inst.operands[0] {
            Operand::Register(r) => *r,
            _ => anyhow::bail!("First operand must be register"),
        };
        let ra = match &inst.operands[1] {
            Operand::Register(r) => *r,
            _ => anyhow::bail!("Second operand must be register"),
        };
        let sh = match &inst.operands[2] {
            Operand::ShiftAmount(s) => *s,
            _ => anyhow::bail!("Third operand must be shift amount"),
        };
        let mask = match &inst.operands[3] {
            Operand::Mask(m) => *m,
            _ => anyhow::bail!("Fourth operand must be mask"),
        };
        
        code.push_str(&self.indent());
        code.push_str(&format!(
            "let rotated = ctx.get_register({}).rotate_left({} as u32);\n",
            rs, sh
        ));
        code.push_str(&self.indent());
        code.push_str(&format!(
            "let masked = rotated & 0x{:08X}u32;\n",
            mask
        ));
        code.push_str(&self.indent());
        code.push_str(&format!(
            "ctx.set_register({}, masked);\n",
            ra
        ));
        
        Ok(code)
    }

    fn generate_system(&mut self, inst: &crate::recompiler::decoder::Instruction) -> Result<String> {
        let mut code = String::new();
        
        // Handle system instructions
        if !inst.operands.is_empty() {
            if let Operand::SpecialRegister(spr) = &inst.operands[0] {
                code.push_str(&self.indent());
                code.push_str(&format!(
                    "// System register operation: SPR {}\n",
                    spr
                ));
                if inst.operands.len() > 1 {
                    if let Operand::Register(rt) = &inst.operands[1] {
                        code.push_str(&self.indent());
                        code.push_str(&format!(
                            "// Move from/to SPR {} to/from r{}\n",
                            spr, rt
                        ));
                    }
                }
            } else {
                code.push_str(&self.indent());
                code.push_str("// Cache control or memory synchronization\n");
            }
        } else {
            code.push_str(&self.indent());
            code.push_str(&format!("// System instruction: opcode 0x{:02X}\n", inst.opcode));
            code.push_str(&self.indent());
            code.push_str("// System instructions typically require special handling\n");
        }
        
        Ok(code)
    }

    fn generate_generic(&mut self, inst: &DecodedInstruction) -> Result<String> {
        let mut code = String::new();
        code.push_str(&self.indent());
        code.push_str(&format!("// Instruction type: {:?}, opcode: 0x{:02X}\n", 
            inst.instruction.instruction_type, inst.opcode));
        code.push_str(&self.indent());
        code.push_str(&format!("// Raw: 0x{:08X}\n", inst.raw));
        code.push_str(&self.indent());
        code.push_str("// TODO: Implement proper handling for this instruction\n");
        
        // Try to generate at least register operations if we can identify them
        if !inst.instruction.operands.is_empty() {
            if let Operand::Register(rt) = &inst.instruction.operands[0] {
                code.push_str(&self.indent());
                code.push_str(&format!(
                    "// First operand is register r{}\n",
                    rt
                ));
            }
        }
        
        Ok(code)
    }

    fn get_register_name(&mut self, operand: &Operand) -> Result<String> {
        match operand {
            Operand::Register(r) => Ok(format!("{}", r)),
            _ => anyhow::bail!("Expected register operand"),
        }
    }

    fn type_to_rust(&self, ty: &crate::recompiler::analysis::TypeInfo) -> String {
        match ty {
            crate::recompiler::analysis::TypeInfo::Void => "()".to_string(),
            crate::recompiler::analysis::TypeInfo::Integer { signed, size } => {
                match (*signed, *size) {
                    (true, 8) => "i8".to_string(),
                    (false, 8) => "u8".to_string(),
                    (true, 16) => "i16".to_string(),
                    (false, 16) => "u16".to_string(),
                    (true, 32) => "i32".to_string(),
                    (false, 32) => "u32".to_string(),
                    (true, 64) => "i64".to_string(),
                    (false, 64) => "u64".to_string(),
                    _ => "u32".to_string(),
                }
            }
            crate::recompiler::analysis::TypeInfo::Pointer { pointee } => {
                format!("*mut {}", self.type_to_rust(pointee))
            }
            _ => "u32".to_string(),
        }
    }

    fn sanitize_identifier(&self, name: &str) -> String {
        name.replace(" ", "_")
            .replace("-", "_")
            .replace(".", "_")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect()
    }

    fn indent(&self) -> String {
        "    ".repeat(self.indent_level)
    }
}

impl Default for CodeGenerator {
    fn default() -> Self {
        Self::new()
    }
}

