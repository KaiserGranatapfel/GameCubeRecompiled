// Inter-Procedural Analysis
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct CallGraph {
    pub nodes: Vec<FunctionNode>,
    pub edges: Vec<CallEdge>,
}

#[derive(Debug, Clone)]
pub struct FunctionNode {
    pub address: u32,
    pub name: String,
    pub is_entry_point: bool,
    pub callers: Vec<usize>,
    pub callees: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct CallEdge {
    pub caller: usize,
    pub callee: usize,
    pub call_sites: Vec<u32>,
}

pub struct InterProceduralAnalyzer;

impl InterProceduralAnalyzer {
    pub fn build_call_graph(functions: &[crate::recompiler::ghidra::FunctionInfo]) -> CallGraph {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut address_to_index: HashMap<u32, usize> = HashMap::new();

        // Create nodes for all functions
        for (idx, func) in functions.iter().enumerate() {
            address_to_index.insert(func.address, idx);
            nodes.push(FunctionNode {
                address: func.address,
                name: func.name.clone(),
                is_entry_point: idx == 0, // First function is typically entry point
                callers: vec![],
                callees: vec![],
            });
        }

        // Build edges from function calls
        // We need to analyze each function's instructions to find call sites
        // For now, we'll use a simplified approach: check if function addresses match
        // In a full implementation, we'd decode and analyze instructions for each function
        
        let mut edge_map: HashMap<(usize, usize), Vec<u32>> = HashMap::new();
        
        // For each function, try to find calls to other functions
        for (caller_idx, caller_func) in functions.iter().enumerate() {
            // In a full implementation, we would:
            // 1. Decode instructions for this function
            // 2. Use ControlFlowAnalyzer::analyze_function_calls() to find call sites
            // 3. Match call targets to function addresses
            
            // Simplified: check if function's basic blocks contain references to other functions
            // This is a placeholder - full implementation would analyze instructions
            for callee_func in functions.iter() {
                if caller_func.address != callee_func.address {
                    // Check if caller might call callee (simplified heuristic)
                    // In real implementation, would check actual call instructions
                    let caller_end = caller_func.address + caller_func.size;
                    if callee_func.address >= caller_func.address 
                        && callee_func.address < caller_end {
                        // Potential call (simplified - would need actual instruction analysis)
                        let callee_idx = match address_to_index.get(&callee_func.address) {
                            Some(idx) => *idx,
                            None => {
                                log::warn!("Callee function 0x{:08X} not found in function list", callee_func.address);
                                continue;
                            }
                        };
                        edge_map.entry((caller_idx, callee_idx))
                            .or_insert_with(Vec::new)
                            .push(caller_func.address); // Call site (simplified)
                    }
                }
            }
        }
        
        // Create edges and update node relationships
        for ((caller_idx, callee_idx), call_sites) in edge_map {
            edges.push(CallEdge {
                caller: caller_idx,
                callee: callee_idx,
                call_sites,
            });
            
            // Update node relationships
            nodes[caller_idx].callees.push(callee_idx);
            nodes[callee_idx].callers.push(caller_idx);
        }

        CallGraph { nodes, edges }
    }

    pub fn find_unreachable_functions(call_graph: &CallGraph) -> Vec<usize> {
        let mut reachable = HashSet::new();
        let mut queue = Vec::new();

        // Start from entry points
        for (idx, node) in call_graph.nodes.iter().enumerate() {
            if node.is_entry_point {
                queue.push(idx);
                reachable.insert(idx);
            }
        }

        // BFS to find all reachable functions
        while let Some(node_idx) = queue.pop() {
            for &callee in &call_graph.nodes[node_idx].callees {
                if !reachable.contains(&callee) {
                    reachable.insert(callee);
                    queue.push(callee);
                }
            }
        }

        // Return unreachable function indices
        (0..call_graph.nodes.len())
            .filter(|idx| !reachable.contains(idx))
            .collect()
    }
}
