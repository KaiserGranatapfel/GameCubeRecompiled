//! Function Call Logging
//!
//! This module provides function call logging and call graph building.
//!
//! # Usage
//!
//! ```rust,no_run
//! use gcrecomp_runtime::tracing::calls::FunctionCallLogger;
//!
//! let mut logger = FunctionCallLogger::new();
//!
//! // Enable logging
//! logger.set_enabled(true);
//!
//! // Set maximum call depth
//! logger.set_max_depth(100);
//!
//! // Log a function call (typically done automatically)
//! logger.log_call(function_address, &cpu_context);
//!
//! // Log return (typically done automatically)
//! logger.log_return(Some(return_value));
//!
//! // Get call graph
//! let call_graph = logger.call_graph();
//!
//! // Export calls
//! let json = logger.export_json()?;
//! ```
//!
//! # Performance Analysis
//!
//! Analyze function call patterns:
//!
//! ```rust,no_run
//! let call_graph = logger.call_graph();
//! let call_counts = logger.call_counts();
//!
//! // Find most called functions
//! for (addr, count) in call_counts.iter() {
//!     if *count > 100 {
//!         println!("Function 0x{:08X} called {} times", addr, count);
//!     }
//! }
//! ```

use gcrecomp_core::runtime::context::CpuContext;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};

/// Logged function call information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// Caller function address
    pub caller: Option<u32>,
    /// Callee function address
    pub callee: u32,
    /// Function arguments (from registers r3-r10)
    pub arguments: [u32; 8],
    /// Return value (from register r3)
    pub return_value: Option<u32>,
    /// Timestamp (nanoseconds since epoch)
    pub timestamp: u64,
    /// Call depth in call stack
    pub depth: usize,
}

/// Call stack entry.
#[derive(Debug, Clone)]
struct CallStackEntry {
    /// Function address
    address: u32,
    /// Call timestamp
    timestamp: u64,
}

/// Function call logger.
pub struct FunctionCallLogger {
    /// Logged function calls
    calls: Vec<FunctionCall>,
    /// Current call stack
    call_stack: VecDeque<CallStackEntry>,
    /// Call graph (caller -> callees)
    call_graph: HashMap<u32, Vec<u32>>,
    /// Function call counts
    call_counts: HashMap<u32, usize>,
    /// Maximum call depth to track
    max_depth: usize,
    /// Whether logging is enabled
    enabled: bool,
}

impl FunctionCallLogger {
    /// Create a new function call logger.
    pub fn new() -> Self {
        Self {
            calls: Vec::new(),
            call_stack: VecDeque::new(),
            call_graph: HashMap::new(),
            call_counts: HashMap::new(),
            max_depth: 1000,
            enabled: false,
        }
    }

    /// Enable or disable logging.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if logging is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Set maximum call depth to track.
    pub fn set_max_depth(&mut self, depth: usize) {
        self.max_depth = depth;
    }

    /// Log a function call entry.
    ///
    /// # Arguments
    /// * `callee` - Function address being called
    /// * `ctx` - CPU context (contains arguments in registers)
    pub fn log_call(&mut self, callee: u32, ctx: &CpuContext) {
        if !self.enabled {
            return;
        }

        let caller = self.call_stack.back().map(|e| e.address);
        let depth = self.call_stack.len();

        // Extract arguments from registers r3-r10
        let arguments = [
            ctx.get_register(3),
            ctx.get_register(4),
            ctx.get_register(5),
            ctx.get_register(6),
            ctx.get_register(7),
            ctx.get_register(8),
            ctx.get_register(9),
            ctx.get_register(10),
        ];

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default() // Should never fail, but handle gracefully
            .as_nanos() as u64;

        // Push to call stack
        if depth < self.max_depth {
            self.call_stack.push_back(CallStackEntry {
                address: callee,
                timestamp,
            });
        }

        // Update call graph
        if let Some(caller_addr) = caller {
            self.call_graph
                .entry(caller_addr)
                .or_insert_with(Vec::new)
                .push(callee);
        }

        // Update call count
        *self.call_counts.entry(callee).or_insert(0) += 1;

        // Log the call
        self.calls.push(FunctionCall {
            caller,
            callee,
            arguments,
            return_value: None, // Will be set on return
            timestamp,
            depth,
        });
    }

    /// Log a function return.
    ///
    /// # Arguments
    /// * `return_value` - Return value from register r3
    pub fn log_return(&mut self, return_value: Option<u32>) {
        if !self.enabled {
            return;
        }

        // Update the most recent call with return value
        if let Some(last_call) = self.calls.last_mut() {
            last_call.return_value = return_value;
        }

        // Pop from call stack
        self.call_stack.pop_back();
    }

    /// Get all logged calls.
    pub fn calls(&self) -> &[FunctionCall] {
        &self.calls
    }

    /// Get the call graph.
    pub fn call_graph(&self) -> &HashMap<u32, Vec<u32>> {
        &self.call_graph
    }

    /// Get call counts for all functions.
    pub fn call_counts(&self) -> &HashMap<u32, usize> {
        &self.call_counts
    }

    /// Get current call stack depth.
    pub fn call_stack_depth(&self) -> usize {
        self.call_stack.len()
    }

    /// Get current call stack (function addresses from caller to callee).
    pub fn call_stack(&self) -> Vec<u32> {
        self.call_stack.iter().map(|e| e.address).collect()
    }

    /// Clear all logged calls.
    pub fn clear(&mut self) {
        self.calls.clear();
        self.call_stack.clear();
        self.call_graph.clear();
        self.call_counts.clear();
    }

    /// Export calls to JSON.
    pub fn export_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.calls)
    }

    /// Export call graph to JSON.
    pub fn export_call_graph_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.call_graph)
    }

    /// Get call count.
    pub fn call_count(&self) -> usize {
        self.calls.len()
    }
}

impl Default for FunctionCallLogger {
    fn default() -> Self {
        Self::new()
    }
}
