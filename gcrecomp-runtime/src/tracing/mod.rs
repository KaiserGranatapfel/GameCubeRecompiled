//! Runtime Tracing System
//!
//! This module provides comprehensive runtime tracing for recompiled code,
//! including instruction tracing, function call logging, and memory access tracking.
//!
//! # Overview
//!
//! The runtime tracing system provides comprehensive execution tracing for recompiled code:
//! - **Instruction Tracing**: Log every instruction execution with full register state
//! - **Function Call Logging**: Track all function calls with arguments and return values
//! - **Memory Access Tracking**: Monitor all memory reads and writes
//!
//! # Usage
//!
//! ## Basic Setup
//!
//! ```rust,no_run
//! use gcrecomp_runtime::Runtime;
//! use gcrecomp_runtime::tracing::{RuntimeTracer, TracingConfig};
//!
//! let mut runtime = Runtime::new()?;
//!
//! // Configure tracing
//! let config = TracingConfig {
//!     instruction_tracing: true,
//!     call_logging: true,
//!     memory_tracking: true,
//! };
//! runtime.tracer().set_config(config);
//! ```
//!
//! ## Exporting Traces
//!
//! ```rust,no_run
//! // Export all traces to a single JSON file
//! let json = runtime.tracer().export_all_json()?;
//!
//! // Or export to separate files
//! runtime.tracer().export_to_files(std::path::Path::new("./traces"))?;
//! ```
//!
//! This creates:
//! - `instruction_traces.json` - All instruction traces
//! - `function_calls.json` - All function calls
//! - `call_graph.json` - Call graph structure
//! - `memory_accesses.json` - All memory accesses
//! - `memory_analysis.json` - Memory access pattern analysis
//!
//! ## Performance Considerations
//!
//! Tracing adds overhead to execution:
//! - **Instruction tracing**: ~10-50% overhead depending on filter
//! - **Function call logging**: ~5-10% overhead
//! - **Memory tracking**: ~20-40% overhead
//!
//! For production use, disable tracing or use aggressive filtering.
//!
//! # Use Cases
//!
//! ## Debugging
//!
//! Use tracing to debug recompiled code by enabling all tracing and exporting
//! traces for analysis.
//!
//! ## Validation
//!
//! Compare traces between original and recompiled code to validate correctness.
//!
//! ## Performance Analysis
//!
//! Analyze function call patterns and memory access patterns to identify
//! optimization opportunities.
//!
//! ## Memory Analysis
//!
//! Detect uninitialized memory access and memory access patterns.
//!
//! # Best Practices
//!
//! 1. **Use Filtering**: Don't trace everything - use filters to focus on relevant code
//! 2. **Limit Trace Count**: Set `max_traces` to prevent memory exhaustion
//! 3. **Export Regularly**: Export traces periodically for long-running code
//! 4. **Disable in Production**: Tracing adds overhead - disable for production builds
//! 5. **Analyze Patterns**: Use analysis functions to understand behavior

pub mod calls;
pub mod instruction;
pub mod memory;

use anyhow::Result;
use calls::FunctionCallLogger;
use instruction::InstructionTracer;
use memory::MemoryAccessTracker;

/// Configuration for runtime tracing.
#[derive(Debug, Clone)]
pub struct TracingConfig {
    /// Enable instruction tracing
    pub instruction_tracing: bool,
    /// Enable function call logging
    pub call_logging: bool,
    /// Enable memory access tracking
    pub memory_tracking: bool,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            instruction_tracing: false,
            call_logging: false,
            memory_tracking: false,
        }
    }
}

/// Runtime tracer that coordinates all tracing subsystems.
pub struct RuntimeTracer {
    /// Instruction tracer
    instruction_tracer: InstructionTracer,
    /// Function call logger
    call_logger: FunctionCallLogger,
    /// Memory access tracker
    memory_tracker: MemoryAccessTracker,
    /// Tracing configuration
    config: TracingConfig,
}

impl RuntimeTracer {
    /// Create a new runtime tracer.
    pub fn new() -> Self {
        Self {
            instruction_tracer: InstructionTracer::new(),
            call_logger: FunctionCallLogger::new(),
            memory_tracker: MemoryAccessTracker::new(),
            config: TracingConfig::default(),
        }
    }

    /// Create a runtime tracer with configuration.
    pub fn with_config(config: TracingConfig) -> Self {
        let mut tracer = Self::new();
        tracer.set_config(config);
        tracer
    }

    /// Set tracing configuration.
    pub fn set_config(&mut self, config: TracingConfig) {
        self.config = config.clone();
        self.instruction_tracer
            .set_enabled(config.instruction_tracing);
        self.call_logger.set_enabled(config.call_logging);
        self.memory_tracker.set_enabled(config.memory_tracking);
    }

    /// Get tracing configuration.
    pub fn config(&self) -> &TracingConfig {
        &self.config
    }

    /// Get instruction tracer.
    pub fn instruction_tracer(&mut self) -> &mut InstructionTracer {
        &mut self.instruction_tracer
    }

    /// Get function call logger.
    pub fn call_logger(&mut self) -> &mut FunctionCallLogger {
        &mut self.call_logger
    }

    /// Get memory access tracker.
    pub fn memory_tracker(&mut self) -> &mut MemoryAccessTracker {
        &mut self.memory_tracker
    }

    /// Export all traces to JSON.
    pub fn export_all_json(&self) -> Result<String> {
        use serde_json::json;

        let mut output = json!({
            "instruction_traces": self.instruction_tracer.traces(),
            "function_calls": self.call_logger.calls(),
            "memory_accesses": self.memory_tracker.accesses(),
        });

        Ok(serde_json::to_string_pretty(&output)?)
    }

    /// Export traces to separate files.
    ///
    /// # Arguments
    /// * `base_path` - Base path for output files
    pub fn export_to_files(&self, base_path: &std::path::Path) -> Result<()> {
        use std::fs;

        if self.config.instruction_tracing {
            let path = base_path.join("instruction_traces.json");
            fs::write(&path, self.instruction_tracer.export_json()?)?;
        }

        if self.config.call_logging {
            let path = base_path.join("function_calls.json");
            fs::write(&path, self.call_logger.export_json()?)?;

            let graph_path = base_path.join("call_graph.json");
            fs::write(&graph_path, self.call_logger.export_call_graph_json()?)?;
        }

        if self.config.memory_tracking {
            let path = base_path.join("memory_accesses.json");
            fs::write(&path, self.memory_tracker.export_json()?)?;

            let analysis_path = base_path.join("memory_analysis.json");
            let analysis = self.memory_tracker.analyze_patterns();
            fs::write(&analysis_path, serde_json::to_string_pretty(&analysis)?)?;
        }

        Ok(())
    }

    /// Clear all traces.
    pub fn clear(&mut self) {
        self.instruction_tracer.clear();
        self.call_logger.clear();
        self.memory_tracker.clear();
    }

    /// Get trace statistics.
    pub fn statistics(&self) -> TraceStatistics {
        TraceStatistics {
            instruction_trace_count: self.instruction_tracer.trace_count(),
            function_call_count: self.call_logger.call_count(),
            memory_access_count: self.memory_tracker.access_count(),
            call_stack_depth: self.call_logger.call_stack_depth(),
            uninitialized_access_count: self.memory_tracker.uninitialized_accesses().len(),
        }
    }
}

impl Default for RuntimeTracer {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about collected traces.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TraceStatistics {
    /// Number of instruction traces
    pub instruction_trace_count: usize,
    /// Number of function calls logged
    pub function_call_count: usize,
    /// Number of memory accesses tracked
    pub memory_access_count: usize,
    /// Current call stack depth
    pub call_stack_depth: usize,
    /// Number of uninitialized memory accesses
    pub uninitialized_access_count: usize,
}
