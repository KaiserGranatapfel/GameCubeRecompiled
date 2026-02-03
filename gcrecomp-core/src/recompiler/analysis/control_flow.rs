//! Control Flow Analysis
//!
//! This module provides control flow graph (CFG) construction and analysis for PowerPC instructions.
//! The CFG is essential for optimizations like dead code elimination, loop detection, and register allocation.
//!
//! # Memory Optimizations
//! - `EdgeType` uses `#[repr(u8)]` to save 3 bytes per edge
//! - `BasicBlock.successors` and `predecessors` use `SmallVec<[u32; 2]>` (most blocks have ≤2)
//! - `Loop.body` uses `BitVec` for efficient membership testing (instead of `HashSet<usize>`)
//! - Block IDs use `u32` instead of `usize` to save 4 bytes on 64-bit systems
//! - Edge indices use `u32` for consistency
//!
//! # CFG Construction Algorithm
//! 1. **Identify block boundaries**: Entry points, branch targets, and fall-through points
//! 2. **Build basic blocks**: Linear sequences of instructions with single entry/exit
//! 3. **Identify edges**: Connect blocks based on branch targets and fall-through
//!
//! # Loop Detection Algorithm
//! Uses depth-first search (DFS) to find back edges, which indicate loops.
//! A back edge is an edge from a node to an ancestor in the DFS tree.

use crate::recompiler::decoder::{DecodedInstruction, Operand};
use anyhow::Result;
use bitvec::prelude::*;
use smallvec::SmallVec;
use std::collections::HashMap;

/// Control flow graph representation.
///
/// # Memory Layout
/// - `nodes`: Vector of basic blocks (heap allocation appropriate for large graphs)
/// - `edges`: Vector of edges (heap allocation appropriate for large graphs)
/// - `entry_block`: Entry point block ID (u32 for consistency)
///
/// # Graph Properties
/// - Directed graph (edges have direction)
/// - May contain cycles (loops)
/// - Single entry point (function entry)
#[derive(Debug, Clone)]
pub struct ControlFlowGraph {
    /// Basic blocks in the control flow graph
    pub nodes: Vec<BasicBlock>,
    /// Edges between basic blocks
    pub edges: Vec<Edge>,
    /// Entry block ID (function entry point)
    pub entry_block: u32,
}

/// Basic block in the control flow graph.
///
/// # Memory Optimization
/// - `id`: Uses `u32` instead of `usize` to save 4 bytes on 64-bit systems
/// - `successors`: Uses `SmallVec<[u32; 2]>` - most blocks have ≤2 successors
/// - `predecessors`: Uses `SmallVec<[u32; 2]>` - most blocks have ≤2 predecessors
///
/// # Basic Block Properties
/// A basic block is a maximal sequence of instructions with:
/// - Single entry point (first instruction)
/// - Single exit point (last instruction is a branch/return)
/// - No internal control flow (linear execution)
#[derive(Debug, Clone)]
#[repr(C)] // Ensure C-compatible layout
pub struct BasicBlock {
    /// Basic block identifier (unique within function)
    /// Uses u32 instead of usize to save 4 bytes on 64-bit systems
    pub id: u32,
    /// Start address of this basic block in original binary
    pub start_address: u32,
    /// End address of this basic block (inclusive)
    pub end_address: u32,
    /// Instructions in this basic block (in execution order)
    pub instructions: Vec<DecodedInstruction>,
    /// Successor basic block IDs (targets of branches)
    /// Uses SmallVec with inline capacity for 2 successors (most blocks have ≤2)
    /// Typical cases: if-then-else (2 successors), loop (1-2 successors), fall-through (1 successor)
    pub successors: SmallVec<[u32; 2]>,
    /// Predecessor basic block IDs (blocks that branch to this block)
    /// Uses SmallVec with inline capacity for 2 predecessors (most blocks have ≤2)
    pub predecessors: SmallVec<[u32; 2]>,
}

/// Edge in the control flow graph.
///
/// # Memory Optimization
/// - `from` and `to`: Use `u32` instead of `usize` to save 4 bytes each on 64-bit systems
/// - `edge_type`: Uses `#[repr(u8)]` enum (1 byte instead of 4-8 bytes)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)] // Ensure C-compatible layout
pub struct Edge {
    /// Source basic block ID
    pub from: u32,
    /// Target basic block ID
    pub to: u32,
    /// Type of edge (determines control flow semantics)
    pub edge_type: EdgeType,
}

/// Type of control flow edge.
///
/// # Memory Optimization
/// Uses `#[repr(u8)]` to reduce size from default enum size (4-8 bytes) to 1 byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)] // Save 3-7 bytes per enum
pub enum EdgeType {
    /// Unconditional branch (always taken)
    Unconditional = 0,
    /// Conditional branch - true path (condition is true)
    ConditionalTrue = 1,
    /// Conditional branch - false path (condition is false)
    ConditionalFalse = 2,
    /// Function call edge (caller -> callee)
    Call = 3,
    /// Return edge (callee -> caller)
    Return = 4,
}

/// Loop information in the control flow graph.
///
/// # Memory Optimization
/// - `header`: Uses `u32` for block ID
/// - `back_edges`: Uses `SmallVec` for small loops (most loops have 1-2 back edges)
/// - `body`: Uses `BitVec` for efficient membership testing (instead of `HashSet<usize>`)
///   - Saves memory: 1 bit per block instead of 8 bytes (pointer) + overhead
///   - Faster membership tests: O(1) bit access vs O(1) hash lookup (but better cache locality)
/// - `exits`: Uses `SmallVec` for small exit lists (most loops have 1-2 exits)
#[derive(Debug, Clone)]
pub struct Loop {
    /// Loop header block ID (entry point of loop)
    pub header: u32,
    /// Back edges (edges from loop body to header)
    /// Uses SmallVec - most loops have 1-2 back edges
    pub back_edges: SmallVec<[(u32, u32); 2]>,
    /// Loop body (set of blocks in the loop)
    /// Uses BitVec for efficient membership testing and memory savings
    /// 1 bit per block instead of 8 bytes (pointer) + hash table overhead
    pub body: BitVec<u32>,
    /// Loop exit blocks (blocks that exit the loop)
    /// Uses SmallVec - most loops have 1-2 exits
    pub exits: SmallVec<[u32; 2]>,
}

/// Control flow analyzer for building and analyzing CFGs.
pub struct ControlFlowAnalyzer;

impl ControlFlowAnalyzer {
    /// Build a control flow graph from a sequence of instructions.
    ///
    /// # Algorithm
    /// 1. **Identify block boundaries**: Entry points, branch targets, and instructions after branches
    /// 2. **Build basic blocks**: Group instructions into blocks with single entry/exit
    /// 3. **Identify edges**: Connect blocks based on branch targets and fall-through
    ///
    /// # Arguments
    /// * `instructions` - Sequence of decoded PowerPC instructions
    /// * `entry_address` - Entry address of the function (first instruction address)
    ///
    /// # Returns
    /// `Result<ControlFlowGraph>` - Constructed control flow graph
    ///
    /// # Errors
    /// Returns error if CFG construction fails (invalid addresses, malformed instructions)
    ///
    /// # Examples
    /// ```rust
    /// let instructions = vec![/* decoded instructions */];
    /// let cfg = ControlFlowAnalyzer::build_cfg(&instructions, 0x80000000)?;
    /// ```
    #[inline] // May be called frequently, but function is large - let compiler decide
    pub fn build_cfg(
        instructions: &[DecodedInstruction],
        entry_address: u32,
    ) -> Result<ControlFlowGraph> {
        let mut nodes: Vec<BasicBlock> = Vec::new();
        let mut edges: Vec<Edge> = Vec::new();
        let mut address_to_block: HashMap<u32, u32> = HashMap::new();
        let mut block_id: u32 = 0u32;
        
        // First pass: identify basic block boundaries
        // Block boundaries occur at:
        // 1. Function entry point
        // 2. Branch targets
        // 3. Instructions immediately after branches (fall-through)
        let mut block_starts: std::collections::HashSet<u32> = std::collections::HashSet::new();
        block_starts.insert(entry_address);
        
        let mut current_address: u32 = entry_address;
        for inst in instructions.iter() {
            // Branch targets start new blocks
            if let Some(target) = Self::get_branch_target(inst) {
                block_starts.insert(target);
                // Next instruction after branch also starts new block (fall-through)
                if current_address.wrapping_add(4) < u32::MAX {
                    block_starts.insert(current_address.wrapping_add(4));
                }
            }
            current_address = current_address.wrapping_add(4); // PowerPC instructions are 4 bytes
        }
        
        // Second pass: build basic blocks
        let mut current_block: Option<BasicBlock> = None;
        let mut current_address: u32 = entry_address;
        
        for inst in instructions.iter() {
            if block_starts.contains(&current_address) {
                // Start new block
                if let Some(block) = current_block.take() {
                    let block_idx: u32 = nodes.len() as u32;
                    address_to_block.insert(block.start_address, block_idx);
                    nodes.push(block);
                }
                current_block = Some(BasicBlock {
                    id: block_id,
                    start_address: current_address,
                    end_address: current_address,
                    instructions: vec![inst.clone()],
                    successors: SmallVec::new(),
                    predecessors: SmallVec::new(),
                });
                block_id = block_id.wrapping_add(1);
            } else if let Some(ref mut block) = current_block {
                block.instructions.push(inst.clone());
                block.end_address = current_address;
            }
            
            current_address = current_address.wrapping_add(4); // PowerPC instructions are 4 bytes
        }
        
        // Add final block if exists
        if let Some(block) = current_block {
            let block_idx: u32 = nodes.len() as u32;
            address_to_block.insert(block.start_address, block_idx);
            nodes.push(block);
        }
        
        // Third pass: identify edges
        // Collect updates to apply after iteration (avoids borrow checker issues)
        let mut successor_updates: Vec<(usize, u32)> = Vec::new();
        let mut predecessor_updates: Vec<(usize, u32)> = Vec::new();

        for (block_idx, block) in nodes.iter().enumerate() {
            let block_idx_u32: u32 = block_idx as u32;
            if let Some(last_inst) = block.instructions.last() {
                if let Some(target) = Self::get_branch_target(last_inst) {
                    if let Some(&target_block) = address_to_block.get(&target) {
                        edges.push(Edge {
                            from: block_idx_u32,
                            to: target_block,
                            edge_type: EdgeType::Unconditional,
                        });
                        // Queue updates for successors and predecessors
                        successor_updates.push((block_idx, target_block));
                        predecessor_updates.push((target_block as usize, block_idx_u32));
                    }
                } else {
                    // Fall-through edge (no explicit branch, continue to next block)
                    if block_idx + 1 < nodes.len() {
                        let next_block_id: u32 = (block_idx + 1) as u32;
                        edges.push(Edge {
                            from: block_idx_u32,
                            to: next_block_id,
                            edge_type: EdgeType::Unconditional,
                        });
                        successor_updates.push((block_idx, next_block_id));
                        predecessor_updates.push((block_idx + 1, block_idx_u32));
                    }
                }
            }
        }

        // Apply deferred updates
        for (block_idx, successor) in successor_updates {
            if let Some(block) = nodes.get_mut(block_idx) {
                if !block.successors.contains(&successor) {
                    block.successors.push(successor);
                }
            }
        }
        for (block_idx, predecessor) in predecessor_updates {
            if let Some(block) = nodes.get_mut(block_idx) {
                if !block.predecessors.contains(&predecessor) {
                    block.predecessors.push(predecessor);
                }
            }
        }
        
        Ok(ControlFlowGraph {
            nodes,
            edges,
            entry_block: 0u32,
        })
    }
    
    /// Extract branch target address from a branch instruction.
    ///
    /// # Arguments
    /// * `inst` - Decoded instruction (should be a branch instruction)
    ///
    /// # Returns
    /// `Option<u32>` - Branch target address if instruction is a branch, None otherwise
    ///
    /// # Algorithm
    /// Checks if instruction is a branch and extracts target from operands:
    /// - Address operand: direct address
    /// - Immediate32 operand: relative offset (would need current PC to compute absolute)
    /// - Register operand: indirect branch (would need register value)
    #[inline] // Hot path - called for every branch instruction
    fn get_branch_target(inst: &DecodedInstruction) -> Option<u32> {
        // Extract branch target from instruction
        if matches!(inst.instruction.instruction_type, crate::recompiler::decoder::InstructionType::Branch) {
            if let Some(Operand::Address(addr)) = inst.instruction.operands.first() {
                return Some(*addr);
            }
            if let Some(Operand::Immediate32(imm)) = inst.instruction.operands.first() {
                // Relative branch - would need current PC to compute absolute address
                // For now, return None (caller should track PC)
                return None;
            }
            if let Some(Operand::Register(0)) = inst.instruction.operands.first() {
                // Branch to link register - would need LR value
                return None;
            }
        }
        None
    }
    
    /// Detect loops in the control flow graph using depth-first search.
    ///
    /// # Algorithm
    /// Uses DFS to find back edges, which indicate loops.
    /// A back edge is an edge from a node to an ancestor in the DFS tree.
    ///
    /// # Arguments
    /// * `cfg` - Control flow graph to analyze
    ///
    /// # Returns
    /// `Vec<Loop>` - List of detected loops with header, body, and back edges
    ///
    /// # Examples
    /// ```rust
    /// let loops = ControlFlowAnalyzer::detect_loops(&cfg);
    /// for loop_info in loops {
    ///     println!("Loop header: block {}", loop_info.header);
    /// }
    /// ```
    #[inline] // May be called frequently
    pub fn detect_loops(cfg: &ControlFlowGraph) -> Vec<Loop> {
        let mut loops: Vec<Loop> = Vec::new();
        let mut visited: BitVec<u32> = bitvec![u32, Lsb0; 0; cfg.nodes.len()];
        let mut in_stack: BitVec<u32> = bitvec![u32, Lsb0; 0; cfg.nodes.len()];
        
        // Use DFS to find back edges (indicates loops)
        Self::dfs_loops(cfg, 0u32, &mut visited, &mut in_stack, &mut loops);
        
        loops
    }
    
    /// Depth-first search helper for loop detection.
    ///
    /// # Algorithm
    /// Performs DFS and identifies back edges (edges to ancestors in DFS tree).
    /// When a back edge is found, a loop is detected.
    ///
    /// # Arguments
    /// * `cfg` - Control flow graph
    /// * `node` - Current node being visited
    /// * `visited` - Bit vector of visited nodes (for efficiency)
    /// * `in_stack` - Bit vector of nodes in current DFS path (for back edge detection)
    /// * `loops` - Output vector of detected loops
    #[inline] // Recursive function - let compiler decide inlining
    fn dfs_loops(
        cfg: &ControlFlowGraph,
        node: u32,
        visited: &mut BitVec<u32>,
        in_stack: &mut BitVec<u32>,
        loops: &mut Vec<Loop>,
    ) {
        let node_idx: usize = node as usize;
        if node_idx >= visited.len() {
            return;
        }
        
        visited.set(node_idx, true);
        in_stack.set(node_idx, true);
        
        if let Some(block) = cfg.nodes.get(node_idx) {
            for &succ in block.successors.iter() {
                let succ_idx: usize = succ as usize;
                if succ_idx >= visited.len() {
                    continue;
                }
                
                if !visited[succ_idx] {
                    Self::dfs_loops(cfg, succ, visited, in_stack, loops);
                } else if in_stack[succ_idx] {
                    // Back edge found - loop detected
                    let loop_header: u32 = succ;
                    let mut loop_body: BitVec<u32> = bitvec![u32, Lsb0; 0; cfg.nodes.len()];
                    loop_body.set(loop_header as usize, true);
                    loop_body.set(node_idx, true);
                    
                    loops.push(Loop {
                        header: loop_header,
                        back_edges: SmallVec::from_slice(&[(node, loop_header)]),
                        body: loop_body,
                        exits: SmallVec::new(),
                    });
                }
            }
        }
        
        in_stack.set(node_idx, false);
    }
    
    /// Analyze function calls in the control flow graph.
    ///
    /// # Algorithm
    /// Scans all instructions in all blocks for branch-with-link instructions (bl, bla),
    /// which indicate function calls.
    ///
    /// # Arguments
    /// * `cfg` - Control flow graph to analyze
    ///
    /// # Returns
    /// `Vec<FunctionCall>` - List of function calls with caller, callee, and location
    ///
    /// # Examples
    /// ```rust
    /// let calls = ControlFlowAnalyzer::analyze_function_calls(&cfg);
    /// for call in calls {
    ///     println!("Call from block {} to 0x{:08X}", call.caller_block, call.callee_address);
    /// }
    /// ```
    #[inline] // May be called frequently
    pub fn analyze_function_calls(cfg: &ControlFlowGraph) -> Vec<FunctionCall> {
        let mut calls: Vec<FunctionCall> = Vec::new();
        
        for block in cfg.nodes.iter() {
            let mut instruction_address: u32 = block.start_address;
            for inst in block.instructions.iter() {
                if Self::is_function_call(inst) {
                    if let Some(target) = Self::get_branch_target(inst) {
                        calls.push(FunctionCall {
                            caller_block: block.id,
                            callee_address: target,
                            instruction_address,
                        });
                    }
                }
                instruction_address = instruction_address.wrapping_add(4); // PowerPC instructions are 4 bytes
            }
        }
        
        calls
    }
    
    /// Check if an instruction is a function call.
    ///
    /// # Algorithm
    /// A function call is a branch instruction with the link bit set (bl, bla).
    /// The link bit causes the return address to be saved in the link register (LR).
    ///
    /// # Arguments
    /// * `inst` - Decoded instruction to check
    ///
    /// # Returns
    /// `bool` - True if instruction is a function call, false otherwise
    #[inline] // Hot path - called for every instruction
    fn is_function_call(inst: &DecodedInstruction) -> bool {
        // Check if instruction is a branch with link (bl, bla)
        matches!(inst.instruction.instruction_type, crate::recompiler::decoder::InstructionType::Branch)
            && (inst.raw & 1u32) != 0u32 // Link bit set
    }
}

/// Function call information.
///
/// # Memory Optimization
/// - `caller_block`: Uses `u32` instead of `usize` to save 4 bytes on 64-bit systems
/// - `callee_address` and `instruction_address`: Already `u32` (optimal)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)] // Ensure C-compatible layout
pub struct FunctionCall {
    /// Caller basic block ID
    pub caller_block: u32,
    /// Callee function address
    pub callee_address: u32,
    /// Instruction address where call occurs
    pub instruction_address: u32,
}
