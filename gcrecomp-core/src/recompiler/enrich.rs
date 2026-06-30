//! Function enrichment: derive useful facts about each discovered function from
//! its decoded instructions.
//!
//! Naive discovery (see `pipeline::naive_function_discovery`) only knows a
//! function's address and byte size. This pass walks the instructions inside each
//! function and adds information the recompiler can act on:
//!
//! - **call graph**: which addresses the function calls (`bl`), and whether it is
//!   a *leaf* (calls nothing — so codegen can skip link-register save/restore).
//! - **shape**: whether it ends in a return (`blr`) and whether it contains a
//!   backward branch (a loop).
//! - **coverage**: how many of its instructions the decoder actually understood
//!   (non-`Unknown`), i.e. how much of it becomes real translated code vs. a
//!   commented-out stub. This is the honest "how much did we recompile" number.

use crate::recompiler::decoder::{DecodedInstruction, InstructionType};
use crate::recompiler::ghidra::FunctionInfo;

/// Derived facts about one function. This is the "info added to a function".
#[derive(Debug, Clone, Default)]
pub struct FunctionFacts {
    pub address: u32,
    pub name: String,
    pub instruction_count: usize,
    pub byte_size: u32,
    /// Addresses called via `bl` (deduplicated, in first-seen order).
    pub call_targets: Vec<u32>,
    /// True if the function never calls another function.
    pub is_leaf: bool,
    /// True if it ends in a `blr` (proper return).
    pub returns: bool,
    /// True if it contains a backward branch (a loop).
    pub has_loop: bool,
    /// Instructions the decoder understood (non-`Unknown`).
    pub translated: usize,
    /// `translated / instruction_count` in `[0.0, 1.0]`.
    pub coverage: f32,
}

const BLR: u32 = 0x4E80_0020;

/// Sign-extend a 26-bit branch displacement (`b`/`bl`, the `LI` field).
fn sign_extend_li(raw: u32) -> i32 {
    let li = (raw & 0x03FF_FFFC) as i32;
    (li << 6) >> 6 // shift the sign bit (bit 25) up and back down
}

/// Compute facts for a single function from the slice of instructions that fall
/// within `[func.address, func.address + func.size)`.
pub fn analyze_function(func: &FunctionInfo, instructions: &[DecodedInstruction]) -> FunctionFacts {
    let end = func.address.wrapping_add(func.size.max(4));
    let body: Vec<&DecodedInstruction> = instructions
        .iter()
        .filter(|i| func.address <= i.address && i.address < end)
        .collect();

    let mut facts = FunctionFacts {
        address: func.address,
        name: func.name.clone(),
        byte_size: func.size,
        instruction_count: body.len(),
        is_leaf: true,
        ..Default::default()
    };

    for inst in &body {
        let raw = inst.raw;
        let primary = raw >> 26;

        if inst.instruction.instruction_type != InstructionType::Unknown {
            facts.translated += 1;
        }

        // `bl`: primary opcode 18 with the LK (link) bit set, AA = 0 (relative).
        if primary == 18 && (raw & 1) == 1 {
            facts.is_leaf = false;
            if (raw & 2) == 0 {
                let target = inst.address.wrapping_add(sign_extend_li(raw) as u32);
                if !facts.call_targets.contains(&target) {
                    facts.call_targets.push(target);
                }
            }
        }

        // Backward relative branch (`b`/`bc`) => a loop.
        if (primary == 18 || primary == 16) && (raw & 2) == 0 {
            let disp = if primary == 18 {
                sign_extend_li(raw)
            } else {
                let bd = (raw & 0x0000_FFFC) as i32;
                (bd << 16) >> 16
            };
            if disp < 0 {
                facts.has_loop = true;
            }
        }

        if raw == BLR {
            facts.returns = true;
        }
    }

    facts.coverage = if facts.instruction_count == 0 {
        0.0
    } else {
        facts.translated as f32 / facts.instruction_count as f32
    };
    facts
}

/// Enrich every discovered function. This is the "function that adds info to the
/// functions": one `FunctionFacts` per `FunctionInfo`.
pub fn enrich_functions(
    functions: &[FunctionInfo],
    instructions: &[DecodedInstruction],
) -> Vec<FunctionFacts> {
    functions
        .iter()
        .map(|f| analyze_function(f, instructions))
        .collect()
}

/// Whole-program coverage rollup over the enriched functions.
#[derive(Debug, Clone, Default)]
pub struct CoverageReport {
    pub functions: usize,
    pub leaf_functions: usize,
    pub functions_with_loops: usize,
    pub total_instructions: usize,
    pub translated_instructions: usize,
}

impl CoverageReport {
    pub fn from_facts(facts: &[FunctionFacts]) -> Self {
        let mut r = CoverageReport {
            functions: facts.len(),
            ..Default::default()
        };
        for f in facts {
            if f.is_leaf {
                r.leaf_functions += 1;
            }
            if f.has_loop {
                r.functions_with_loops += 1;
            }
            r.total_instructions += f.instruction_count;
            r.translated_instructions += f.translated;
        }
        r
    }

    /// Fraction of instructions translated to real code, in `[0.0, 1.0]`.
    pub fn instruction_coverage(&self) -> f32 {
        if self.total_instructions == 0 {
            0.0
        } else {
            self.translated_instructions as f32 / self.total_instructions as f32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recompiler::decoder::Instruction;
    use crate::recompiler::ghidra::FunctionInfo;

    fn func(addr: u32, size: u32) -> FunctionInfo {
        FunctionInfo {
            address: addr,
            name: format!("sub_{:08x}", addr),
            size,
            calling_convention: "default".into(),
            parameters: vec![],
            return_type: None,
            local_variables: vec![],
            basic_blocks: vec![],
        }
    }

    #[test]
    fn detects_leaf_return_and_coverage() {
        // addi (known), bl (call -> not leaf), blr (return)
        let words = [0x3800_0001u32, 0x4800_0011u32 /* bl +0x10 */, BLR];
        let is: Vec<_> = words
            .iter()
            .enumerate()
            .map(|(i, &w)| Instruction::decode(w, 0x100 + (i as u32) * 4).unwrap())
            .collect();
        let f = analyze_function(&func(0x100, 12), &is);
        assert!(!f.is_leaf, "has a bl -> not a leaf");
        assert!(f.returns, "ends in blr");
        assert_eq!(f.call_targets, vec![0x100 + 4 + 0x10], "bl target resolved");
        assert_eq!(f.instruction_count, 3);
        assert!(f.coverage > 0.0 && f.coverage <= 1.0);
    }

    #[test]
    fn leaf_function_has_no_calls() {
        let is = vec![Instruction::decode(BLR, 0x200).unwrap()];
        let f = analyze_function(&func(0x200, 4), &is);
        assert!(f.is_leaf);
        assert!(f.call_targets.is_empty());
    }
}
