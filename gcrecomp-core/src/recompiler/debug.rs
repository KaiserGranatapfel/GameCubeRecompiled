// Debugging Support
use std::collections::HashMap;

pub struct DebugInfo {
    pub source_mapping: HashMap<u32, SourceLocation>,
    pub breakpoints: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub original_address: u32,
    pub rust_file: String,
    pub rust_line: usize,
    pub rust_column: usize,
}

impl DebugInfo {
    pub fn new() -> Self {
        Self {
            source_mapping: HashMap::new(),
            breakpoints: Vec::new(),
        }
    }
    
    pub fn add_mapping(&mut self, original_addr: u32, rust_file: String, line: usize, col: usize) {
        self.source_mapping.insert(original_addr, SourceLocation {
            original_address: original_addr,
            rust_file,
            rust_line: line,
            rust_column: col,
        });
    }
    
    pub fn get_rust_location(&self, original_addr: u32) -> Option<&SourceLocation> {
        self.source_mapping.get(&original_addr)
    }
}

pub struct ExecutionTracer {
    pub trace_log: Vec<TraceEntry>,
}

#[derive(Debug, Clone)]
pub struct TraceEntry {
    pub address: u32,
    pub instruction: String,
    pub register_state: HashMap<u8, u32>,
    pub memory_accesses: Vec<MemoryAccess>,
}

#[derive(Debug, Clone)]
pub struct MemoryAccess {
    pub address: u32,
    pub value: u32,
    pub is_write: bool,
}

impl ExecutionTracer {
    pub fn new() -> Self {
        Self {
            trace_log: Vec::new(),
        }
    }
    
    pub fn trace_instruction(&mut self, addr: u32, inst: &str) {
        self.trace_log.push(TraceEntry {
            address: addr,
            instruction: inst.to_string(),
            register_state: HashMap::new(),
            memory_accesses: Vec::new(),
        });
    }
}

