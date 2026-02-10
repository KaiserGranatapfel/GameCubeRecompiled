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
        let edges = Vec::new();
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
        // This would need to analyze instructions to find call sites
        // For now, placeholder

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
