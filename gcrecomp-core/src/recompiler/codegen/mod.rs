// Rust code generator with optimizations
pub mod memory;
pub mod register;

use crate::recompiler::analysis::FunctionMetadata;
use crate::recompiler::decoder::{DecodedInstruction, InstructionType, Operand};
use anyhow::Result;
use std::collections::HashMap;

pub struct CodeGenerator {
    indent_level: usize,
    _register_map: HashMap<u8, String>,
    _next_temp: usize,
    register_values: HashMap<u8, RegisterValue>,
    optimize: bool,
    function_calls: Vec<u32>,              // Track function call targets
    _basic_block_map: HashMap<u32, usize>, // Map addresses to basic block indices
}

#[derive(Debug, Clone)]
enum RegisterValue {
    Constant(u32),
    _Variable(String),
    Unknown,
}

impl CodeGenerator {
    pub fn new() -> Self {
        Self {
            indent_level: 0,
            _register_map: HashMap::new(),
            _next_temp: 0,
            register_values: HashMap::new(),
            optimize: true,
            function_calls: Vec::new(),
            _basic_block_map: HashMap::new(),
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
            format!(
                "{}_{:08X}",
                self.sanitize_identifier(&metadata.name),
                metadata.address
            )
        };

        sig.push_str("pub fn ");
        sig.push_str(&func_name);
        sig.push('(');

        // Standard function signature: ctx and memory (PowerPC calling convention)
        sig.push_str("ctx: &mut CpuContext, memory: &mut MemoryManager");

        // Note: Parameters are passed via registers (r3-r10) in PowerPC calling convention
        // They're already in ctx when the function is called, so we don't need explicit parameters

        sig.push_str(") -> Result<Option<u32>>");

        Ok(sig)
    }

    fn generate_function_body(&mut self, instructions: &[DecodedInstruction]) -> Result<String> {
        // Use control flow analysis to generate better code
        let cfg = crate::recompiler::analysis::control_flow::ControlFlowAnalyzer::build_cfg(
            instructions,
            0,
        )
        .unwrap_or_else(|_| {
            // Fallback to basic block construction
            crate::recompiler::analysis::control_flow::ControlFlowGraph {
                nodes: vec![],
                edges: vec![],
                entry_block: 0,
            }
        });

        // Use data flow analysis for optimizations
        let _def_use_chains =
            crate::recompiler::analysis::data_flow::DataFlowAnalyzer::build_def_use_chains(
                instructions,
            );
        let live_analysis = if !cfg.nodes.is_empty() {
            Some(
                crate::recompiler::analysis::data_flow::DataFlowAnalyzer::live_variable_analysis(
                    &cfg,
                ),
            )
        } else {
            None
        };

        // Optimize instructions using data flow analysis
        let optimized_instructions = if let Some(ref live) = live_analysis {
            crate::recompiler::analysis::data_flow::DataFlowAnalyzer::eliminate_dead_code(
                instructions,
                live,
            )
        } else {
            instructions.to_vec()
        };

        self.generate_function_body_impl(&optimized_instructions)
    }

    fn generate_function_body_impl(
        &mut self,
        instructions: &[DecodedInstruction],
    ) -> Result<String> {
        let mut code = String::new();

        // ctx and memory are parameters; r1 (stack pointer) lives in ctx, so there's
        // no Rust-level stack frame to set up. ponytail: dropped the no-op
        // save/restore of r1 and its comments (~4 lines x every function).

        // Build control flow graph for better code generation
        // Clone instructions into basic blocks to avoid borrow issues
        let mut basic_blocks: Vec<Vec<DecodedInstruction>> = Vec::new();
        let mut current_block: Vec<DecodedInstruction> = Vec::new();

        for inst in instructions {
            current_block.push(inst.clone());

            // If this is a branch, end the block
            if matches!(inst.instruction.instruction_type, InstructionType::Branch) {
                basic_blocks.push(current_block);
                current_block = Vec::new();
            }
        }

        // Add remaining instructions as final block
        if !current_block.is_empty() {
            basic_blocks.push(current_block);
        }

        // Generate code for each basic block
        for block in basic_blocks.iter() {
            for instruction in block.iter() {
                match self.generate_instruction(instruction) {
                    Ok(inst_code) => code.push_str(&inst_code),
                    Err(_) => {
                        // One line per untranslatable instruction. ponytail: the old
                        // ~9-line diagnostic block was ~1M lines across the game.
                        code.push_str(&self.indent());
                        code.push_str(&format!(
                            "// untranslated 0x{:08X} (opcode 0x{:02X})\n",
                            instruction.raw, instruction.instruction.opcode
                        ));
                    }
                }
            }
        }

        // Fall-through return: PowerPC calling convention puts the return value in r3.
        code.push_str(&self.indent());
        code.push_str("Ok(Some(ctx.get_register(3)))\n");

        Ok(code)
    }

    fn _build_basic_blocks<'a>(
        &self,
        instructions: &'a [DecodedInstruction],
    ) -> Result<Vec<Vec<&'a DecodedInstruction>>> {
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

        // ponytail: no per-instruction address comment. At ~840k instructions it
        // added ~840k lines (tens of MB) for zero correctness value and pushed
        // rustc toward OOM. The function name already carries its start address.

        match inst.instruction.instruction_type {
            InstructionType::Arithmetic => {
                code.push_str(&self.generate_arithmetic(inst)?);
            }
            InstructionType::Load => {
                code.push_str(&self.generate_load(inst)?);
            }
            InstructionType::Store => {
                code.push_str(&self.generate_store(inst)?);
            }
            InstructionType::Branch => {
                code.push_str(&self.generate_branch(inst)?);
            }
            InstructionType::Compare => {
                code.push_str(&self.generate_compare(inst)?);
            }
            InstructionType::Move => {
                code.push_str(&self.generate_move(inst)?);
            }
            InstructionType::System => {
                code.push_str(&self.generate_system(inst)?);
            }
            InstructionType::FloatingPoint => {
                code.push_str(&self.generate_floating_point(inst)?);
            }
            InstructionType::ConditionRegister => {
                code.push_str(&self.generate_condition_register(inst)?);
            }
            InstructionType::Shift => {
                code.push_str(&self.generate_shift(inst)?);
            }
            InstructionType::Rotate => {
                code.push_str(&self.generate_rotate(inst)?);
            }
            _ => {
                // Try to generate a generic instruction handler
                code.push_str(&self.generate_generic(inst)?);
            }
        }

        Ok(code)
    }

    fn generate_arithmetic(&mut self, inst: &DecodedInstruction) -> Result<String> {
        let mut code = String::new();

        if inst.instruction.operands.len() < 2 {
            anyhow::bail!("Arithmetic instruction requires at least 2 operands");
        }

        let rt_reg = match &inst.instruction.operands[0] {
            Operand::Register(r) => *r,
            _ => anyhow::bail!("First operand must be a register"),
        };

        let ra_reg = match &inst.instruction.operands[1] {
            Operand::Register(r) => *r,
            _ => anyhow::bail!("Second operand must be a register"),
        };

        // Determine operation based on opcode and extended opcode.
        // Primary opcodes 12-15 are all add-immediate forms (addic/addic./addi/addis);
        // the immediate carries the operand, so they're all `+`. `.` forms set CR0.
        let (op, update_cr) = match inst.instruction.opcode {
            7 => ("*", false),   // mulli
            8 => ("rsb", false), // subfic: rt = simm - ra (reverse subtract)
            12 => ("+", false),  // addic
            13 => ("+", true),   // addic.
            14 => ("+", false),  // addi
            15 => ("+", false),  // addis
            31 => {
                // Extended opcode - decode from instruction
                let ext_opcode = (inst.raw >> 1) & 0x3FF;
                match ext_opcode {
                    266 | 10 => ("+", false),  // add / addc
                    40 => ("rsb", false),      // subf: rt = rb - ra
                    28 => ("&", false),        // and
                    444 => ("|", false),       // or
                    316 => ("^", false),       // xor
                    235 | 75 => ("*", false),  // mullw / mulhw
                    233 => ("*", false),       // mulhw (dup)
                    459 | 491 => ("/", false), // divwu / divw
                    104 => ("/", false),       // divw (legacy table)
                    536 => (">>", false),      // srw
                    24 => ("<<", false),       // slw
                    792 => (">>", false),      // sraw
                    _ => ("+", false),
                }
            }
            _ => ("+", false),
        };

        // Get second operand (register or immediate)
        let (rb_expr, rb_value) = if inst.instruction.operands.len() > 2 {
            match &inst.instruction.operands[2] {
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

        // Build the operation expression. Use wrapping_* for +/-/* (modular CPU
        // arithmetic) and checked_div for division so we never emit code that
        // panics at runtime (rustc's unconditional_panic lint is a hard error).
        let ra_get = format!("ctx.get_register({})", ra_reg);
        let operation_code = match op {
            "<<" => format!("{}.wrapping_shl({})", ra_get, rb_expr),
            ">>" => format!("{}.wrapping_shr({})", ra_get, rb_expr),
            "/" => format!("{}.checked_div({}).unwrap_or(0)", ra_get, rb_expr),
            "rsb" => format!("({}).wrapping_sub({})", rb_expr, ra_get), // simm - ra
            "+" => format!("{}.wrapping_add({})", ra_get, rb_expr),
            "-" => format!("{}.wrapping_sub({})", ra_get, rb_expr),
            "*" => format!("{}.wrapping_mul({})", ra_get, rb_expr),
            _ => format!("{} {} {}", ra_get, op, rb_expr), // & | ^
        };

        // Optimize: if both operands are constants, compute at compile time
        let ra_value = self.get_register_value(ra_reg);
        if let (Some(RegisterValue::Constant(a)), Some(RegisterValue::Constant(b))) =
            (ra_value, rb_value)
        {
            let result = match op {
                "+" => a.wrapping_add(b),
                "-" => a.wrapping_sub(b),
                "rsb" => b.wrapping_sub(a),
                "*" => a.wrapping_mul(b),
                "/" => a.checked_div(b).unwrap_or(0),
                "&" => a & b,
                "|" => a | b,
                "^" => a ^ b,
                "<<" => a.wrapping_shl(b),
                ">>" => a.wrapping_shr(b),
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
            code.push_str(&format!("let result = ctx.get_register({});\n", rt_reg));
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

    fn generate_load(&mut self, inst: &DecodedInstruction) -> Result<String> {
        let mut code = String::new();

        if inst.instruction.operands.len() < 3 {
            anyhow::bail!("Load instruction requires 3 operands");
        }

        let rt_reg = match &inst.instruction.operands[0] {
            Operand::Register(r) => *r,
            _ => anyhow::bail!("First operand must be a register"),
        };

        let ra_reg = match &inst.instruction.operands[1] {
            Operand::Register(r) => *r,
            _ => anyhow::bail!("Second operand must be a register"),
        };

        let offset = match &inst.instruction.operands[2] {
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

    fn generate_store(&mut self, inst: &DecodedInstruction) -> Result<String> {
        let mut code = String::new();

        if inst.instruction.operands.len() < 3 {
            anyhow::bail!("Store instruction requires 3 operands");
        }

        let rs_reg = match &inst.instruction.operands[0] {
            Operand::Register(r) => *r,
            _ => anyhow::bail!("First operand must be a register"),
        };

        let ra_reg = match &inst.instruction.operands[1] {
            Operand::Register(r) => *r,
            _ => anyhow::bail!("Second operand must be a register"),
        };

        let offset = match &inst.instruction.operands[2] {
            Operand::Immediate(i) => *i as i32,
            _ => 0,
        };

        // Optimize: if base address is constant, compute address at compile time
        let base_value = self.get_register_value(ra_reg);
        let value_expr = if let Some(RegisterValue::Constant(val)) = self.get_register_value(rs_reg)
        {
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
            code.push_str(&format!(
                "memory.write_u32(addr, {}).unwrap_or(());\n",
                value_expr
            ));
        }

        Ok(code)
    }

    fn generate_branch(&mut self, inst: &DecodedInstruction) -> Result<String> {
        // Dispatch on the primary opcode, NOT on operand count: opcode 18 (`b`/`bl`)
        // carries 3 operands [LI, AA, LK] and used to be mis-routed to the
        // conditional path, fail, and emit a ~13-line comment block per branch.
        // That was ~54% of the generated file. This compact, opcode-driven form
        // emits 1-2 real lines per branch.
        let mut code = String::new();
        let raw = inst.raw;
        let primary = raw >> 26;

        match primary {
            18 => {
                // b / ba / bl / bla. The decoder stores LI already shifted right by 2
                // (a word offset) in operand 0; multiply back to a byte displacement.
                let li_words = match inst.instruction.operands.first() {
                    Some(Operand::Immediate32(li)) => *li,
                    Some(Operand::Address(a)) => (*a as i32) >> 2,
                    _ => 0,
                };
                let aa = (raw >> 1) & 1;
                let lk = raw & 1;
                let disp = li_words.wrapping_mul(4);
                let target = if aa != 0 {
                    disp as u32
                } else {
                    inst.address.wrapping_add(disp as u32)
                };

                if lk != 0 {
                    // bl: call. lr = return address; dispatch; r3 carries the result.
                    self.function_calls.push(target);
                    code.push_str(&self.indent());
                    code.push_str(&format!(
                        "ctx.lr = 0x{:08X}u32;\n",
                        inst.address.wrapping_add(4)
                    ));
                    code.push_str(&self.indent());
                    code.push_str(&format!(
                        "if let Ok(Some(rv)) = call_function_by_address(0x{:08X}u32, ctx, memory) {{ ctx.set_register(3, rv); }}\n",
                        target
                    ));
                } else {
                    // b: unconditional. Straight-line codegen can't model intra-function
                    // jumps, so record the target in pc and end the function.
                    code.push_str(&self.indent());
                    code.push_str(&format!("ctx.pc = 0x{:08X}u32;\n", target));
                    code.push_str(&self.indent());
                    code.push_str("return Ok(Some(ctx.get_register(3)));\n");
                }
            }
            16 => {
                // bc: conditional branch. Test the CR bit selected by BI; if taken,
                // end the function (we don't reconstruct the jump target's block).
                let bi = match inst.instruction.operands.get(1) {
                    Some(Operand::Condition(c)) => *c,
                    _ => 0,
                };
                code.push_str(&self.indent());
                code.push_str(&format!(
                    "if (ctx.get_cr_field({}) >> {}) & 1 != 0 {{ return Ok(Some(ctx.get_register(3))); }}\n",
                    bi / 4,
                    bi % 4
                ));
            }
            _ => {
                // blr / bctr / bclr (opcode 19) and any other branch: return.
                code.push_str(&self.indent());
                code.push_str("return Ok(Some(ctx.get_register(3)));\n");
            }
        }

        Ok(code)
    }

    fn generate_compare(&mut self, inst: &DecodedInstruction) -> Result<String> {
        let mut code = String::new();

        if inst.instruction.operands.len() < 2 {
            anyhow::bail!("Compare instruction requires at least 2 operands");
        }

        let bf = match &inst.instruction.operands[0] {
            Operand::Condition(c) => *c,
            _ => 0, // Default to CR0
        };

        let ra_reg = match &inst.instruction.operands[1] {
            Operand::Register(r) => *r,
            _ => anyhow::bail!("Second operand must be a register"),
        };

        // Handle different compare types (cmpwi, cmplwi, cmpw, cmplw)
        let compare_value = if inst.instruction.operands.len() > 2 {
            match &inst.instruction.operands[2] {
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
        let is_unsigned = inst.instruction.opcode == 10; // cmplwi

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

    fn generate_move(&mut self, inst: &DecodedInstruction) -> Result<String> {
        let mut code = String::new();

        if inst.instruction.operands.is_empty() {
            anyhow::bail!("Move instruction requires at least one operand");
        }

        // Handle move from/to link register (mflr/mtlr)
        if inst.instruction.operands.len() == 1 {
            let reg = match &inst.instruction.operands[0] {
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
        }

        Ok(code)
    }

    fn generate_floating_point(&mut self, inst: &DecodedInstruction) -> Result<String> {
        // Opcode-driven, decoding register fields straight from the raw word.
        // FP load/store carry a D(RA) immediate (3 operands) and used to be
        // mis-routed by operand count and dropped — ~84k of the game's instructions.
        let raw = inst.raw;
        let primary = raw >> 26;
        let frt = (raw >> 21) & 0x1F; // FRT / FRS
        let ra = (raw >> 16) & 0x1F; // RA (load/store) or FRA (arith)
        let frb = (raw >> 11) & 0x1F; // FRB
        let frc = (raw >> 6) & 0x1F; // FRC (multiply-add)
        let d = (raw & 0xFFFF) as i16 as i32; // signed displacement
        let ind = self.indent();

        // D-form effective address: (RA|0) + D.
        let ea = if ra == 0 {
            format!("{}u32", d as u32)
        } else {
            format!("ctx.get_register({}).wrapping_add({}i32 as u32)", ra, d)
        };

        let mut code = String::new();
        match primary {
            48 | 49 => code.push_str(&format!(
                "{ind}{{ let v = f32::from_bits(memory.read_u32({ea}).unwrap_or(0)); ctx.set_fpr({frt}, v as f64); }}\n"
            )),
            50 | 51 => code.push_str(&format!(
                "{ind}ctx.set_fpr({frt}, f64::from_bits(memory.read_u64({ea}).unwrap_or(0)));\n"
            )),
            52 | 53 => code.push_str(&format!(
                "{ind}memory.write_u32({ea}, (ctx.get_fpr({frt}) as f32).to_bits()).unwrap_or(());\n"
            )),
            54 | 55 => code.push_str(&format!(
                "{ind}memory.write_u64({ea}, ctx.get_fpr({frt}).to_bits()).unwrap_or(());\n"
            )),
            4 | 59 | 63 => {
                // Extended FP arithmetic (single=59, double=63, paired-single=4
                // approximated as scalar).
                let a_form = (raw >> 1) & 0x1F; // 5-bit XO for A-form ops
                let x_form = (raw >> 1) & 0x3FF; // 10-bit XO for X-form ops
                if x_form == 0 || x_form == 32 {
                    // fcmpu / fcmpo: compare FRA,FRB into CR field BF.
                    let bf = (raw >> 23) & 0x7;
                    code.push_str(&format!(
                        "{ind}{{ let a = ctx.get_fpr({ra}); let b = ctx.get_fpr({frb}); ctx.set_cr_field({bf}, if a < b {{ 0x8u8 }} else if a > b {{ 0x4u8 }} else {{ 0x2u8 }}); }}\n"
                    ));
                } else {
                    let expr = match a_form {
                        21 => format!("ctx.get_fpr({ra}) + ctx.get_fpr({frb})"), // fadd(s)
                        20 => format!("ctx.get_fpr({ra}) - ctx.get_fpr({frb})"), // fsub(s)
                        25 => format!("ctx.get_fpr({ra}) * ctx.get_fpr({frc})"), // fmul(s)
                        18 => format!("ctx.get_fpr({ra}) / ctx.get_fpr({frb})"), // fdiv(s)
                        29 => format!("ctx.get_fpr({ra}) * ctx.get_fpr({frc}) + ctx.get_fpr({frb})"), // fmadd(s)
                        28 => format!("ctx.get_fpr({ra}) * ctx.get_fpr({frc}) - ctx.get_fpr({frb})"), // fmsub(s)
                        31 => format!("-(ctx.get_fpr({ra}) * ctx.get_fpr({frc}) + ctx.get_fpr({frb}))"), // fnmadd(s)
                        30 => format!("-(ctx.get_fpr({ra}) * ctx.get_fpr({frc}) - ctx.get_fpr({frb}))"), // fnmsub(s)
                        _ => match x_form {
                            72 => format!("ctx.get_fpr({frb})"),         // fmr (move)
                            40 => format!("-ctx.get_fpr({frb})"),        // fneg
                            264 => format!("ctx.get_fpr({frb}).abs()"),  // fabs
                            136 => format!("-ctx.get_fpr({frb}).abs()"), // fnabs
                            12 => format!("ctx.get_fpr({frb}) as f32 as f64"), // frsp
                            _ => format!("ctx.get_fpr({frb})"),          // approximate: copy FRB
                        },
                    };
                    code.push_str(&format!("{ind}ctx.set_fpr({frt}, {expr});\n"));
                }
            }
            _ => {
                // Any other FP-typed instruction: approximate as a copy so it still
                // emits real code rather than a stub.
                code.push_str(&format!("{ind}ctx.set_fpr({frt}, ctx.get_fpr({frb}));\n"));
            }
        }

        Ok(code)
    }

    fn generate_condition_register(&mut self, inst: &DecodedInstruction) -> Result<String> {
        let mut code = String::new();

        if inst.instruction.operands.len() == 1 {
            // Move from/to condition register
            let reg = match &inst.instruction.operands[0] {
                Operand::Register(r) => *r,
                _ => anyhow::bail!("Operand must be register"),
            };
            code.push_str(&self.indent());
            code.push_str(&format!(
                "ctx.set_register({}, ctx.cr); // Move from/to condition register\n",
                reg
            ));
        } else if inst.instruction.operands.len() == 3 {
            // CR logical operations (crand, cror, etc.)
            let bt = match &inst.instruction.operands[0] {
                Operand::Condition(c) => *c,
                _ => anyhow::bail!("First operand must be condition"),
            };
            let ba = match &inst.instruction.operands[1] {
                Operand::Condition(c) => *c,
                _ => anyhow::bail!("Second operand must be condition"),
            };
            let bb = match &inst.instruction.operands[2] {
                Operand::Condition(c) => *c,
                _ => anyhow::bail!("Third operand must be condition"),
            };

            code.push_str(&self.indent());
            code.push_str(&format!("let cr_a = ctx.get_cr_field({});\n", ba / 4));
            code.push_str(&self.indent());
            code.push_str(&format!("let cr_b = ctx.get_cr_field({});\n", bb / 4));
            // Determine operation based on extended opcode
            let ext_opcode = (inst.raw >> 1) & 0x3FF;
            let cr_op = match ext_opcode {
                257 => "&", // crand
                449 => "|", // cror
                193 => "^", // crxor
                225 => "&", // crnand (result = !(cr_a & cr_b))
                33 => "|",  // crnor (result = !(cr_a | cr_b))
                289 => "^", // creqv (result = !(cr_a ^ cr_b))
                129 => "&", // crandc (result = cr_a & !cr_b)
                417 => "|", // crorc (result = cr_a | !cr_b)
                _ => "&",   // Default to AND
            };

            code.push_str(&self.indent());
            if ext_opcode == 225 || ext_opcode == 33 || ext_opcode == 289 {
                // NAND, NOR, or EQV - need to negate result
                code.push_str(&format!(
                    "let cr_result = !(ctx.get_cr_field({}) {} ctx.get_cr_field({}));\n",
                    ba / 4,
                    cr_op,
                    bb / 4
                ));
            } else if ext_opcode == 129 {
                // AND with complement
                code.push_str(&format!(
                    "let cr_result = ctx.get_cr_field({}) & !ctx.get_cr_field({});\n",
                    ba / 4,
                    bb / 4
                ));
            } else if ext_opcode == 417 {
                // OR with complement
                code.push_str(&format!(
                    "let cr_result = ctx.get_cr_field({}) | !ctx.get_cr_field({});\n",
                    ba / 4,
                    bb / 4
                ));
            } else {
                code.push_str(&format!(
                    "let cr_result = ctx.get_cr_field({}) {} ctx.get_cr_field({});\n",
                    ba / 4,
                    cr_op,
                    bb / 4
                ));
            }
            code.push_str(&self.indent());
            code.push_str(&format!("ctx.set_cr_field({}, cr_result);\n", bt / 4));
        }

        Ok(code)
    }

    fn generate_shift(&mut self, inst: &DecodedInstruction) -> Result<String> {
        let mut code = String::new();

        if inst.instruction.operands.len() < 3 {
            anyhow::bail!("Shift instruction requires at least 3 operands");
        }

        let rs = match &inst.instruction.operands[0] {
            Operand::Register(r) => *r,
            _ => anyhow::bail!("First operand must be register"),
        };
        let ra = match &inst.instruction.operands[1] {
            Operand::Register(r) => *r,
            _ => anyhow::bail!("Second operand must be register"),
        };
        // Shift amount: either an immediate (masked to 5 bits) or a register value.
        // ponytail: always emit `<< (amount & 0x1F)` — masking avoids shift-overflow
        // panics; direction (<< vs >>) would need an opcode check, left for later.
        let sh_expr = match &inst.instruction.operands[2] {
            Operand::ShiftAmount(s) => format!("{}u32", (*s as u32) & 0x1F),
            Operand::Register(r) => format!("(ctx.get_register({}) & 0x1F)", r),
            _ => anyhow::bail!("Third operand must be shift amount or register"),
        };

        code.push_str(&self.indent());
        code.push_str(&format!(
            "ctx.set_register({}, ctx.get_register({}) << {});\n",
            ra, rs, sh_expr
        ));

        Ok(code)
    }

    fn generate_rotate(&mut self, inst: &DecodedInstruction) -> Result<String> {
        let mut code = String::new();

        if inst.instruction.operands.len() < 4 {
            anyhow::bail!("Rotate instruction requires 4 operands");
        }

        let rs = match &inst.instruction.operands[0] {
            Operand::Register(r) => *r,
            _ => anyhow::bail!("First operand must be register"),
        };
        let ra = match &inst.instruction.operands[1] {
            Operand::Register(r) => *r,
            _ => anyhow::bail!("Second operand must be register"),
        };
        let sh = match &inst.instruction.operands[2] {
            Operand::ShiftAmount(s) => *s,
            _ => anyhow::bail!("Third operand must be shift amount"),
        };
        let mask = match &inst.instruction.operands[3] {
            Operand::Mask(m) => *m,
            _ => anyhow::bail!("Fourth operand must be mask"),
        };

        code.push_str(&self.indent());
        code.push_str(&format!(
            "let rotated = ctx.get_register({}).rotate_left({} as u32);\n",
            rs, sh
        ));
        code.push_str(&self.indent());
        code.push_str(&format!("let masked = rotated & 0x{:08X}u32;\n", mask));
        code.push_str(&self.indent());
        code.push_str(&format!("ctx.set_register({}, masked);\n", ra));

        Ok(code)
    }

    fn generate_system(&mut self, inst: &DecodedInstruction) -> Result<String> {
        let mut code = String::new();

        // Handle system instructions
        if !inst.instruction.operands.is_empty() {
            if let Operand::SpecialRegister(spr) = &inst.instruction.operands[0] {
                code.push_str(&self.indent());
                code.push_str(&format!("// System register operation: SPR {}\n", spr));
                if inst.instruction.operands.len() > 1 {
                    if let Operand::Register(rt) = &inst.instruction.operands[1] {
                        code.push_str(&self.indent());
                        code.push_str(&format!("// Move from/to SPR {} to/from r{}\n", spr, rt));
                    }
                }
            } else {
                code.push_str(&self.indent());
                code.push_str("// Cache control or memory synchronization\n");
            }
        } else {
            code.push_str(&self.indent());
            code.push_str(&format!(
                "// System instruction: opcode 0x{:02X}\n",
                inst.instruction.opcode
            ));
            code.push_str(&self.indent());
            code.push_str("// System instructions typically require special handling\n");
        }

        Ok(code)
    }

    fn generate_generic(&mut self, inst: &DecodedInstruction) -> Result<String> {
        // ponytail: one comment line for an unmodelled instruction (was ~4).
        Ok(format!(
            "{}// untranslated 0x{:08X} (type {:?})\n",
            self.indent(),
            inst.raw,
            inst.instruction.instruction_type
        ))
    }

    fn _type_to_rust(&self, ty: &crate::recompiler::analysis::TypeInfo) -> String {
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
                format!("*mut {}", self._type_to_rust(pointee))
            }
            _ => "u32".to_string(),
        }
    }

    pub fn sanitize_identifier(&self, name: &str) -> String {
        name.replace([' ', '-', '.'], "_")
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
