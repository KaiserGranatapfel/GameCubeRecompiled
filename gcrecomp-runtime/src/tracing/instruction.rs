//! Instruction-Level Tracing
//!
//! This module provides instruction-level execution tracing for debugging
//! and analysis of recompiled code.
//!
//! # Usage
//!
//! ```rust,no_run
//! use gcrecomp_runtime::tracing::instruction::InstructionTracer;
//!
//! let mut tracer = InstructionTracer::new();
//!
//! // Enable tracing
//! tracer.set_enabled(true);
//!
//! // Add filters (optional)
//! tracer.filter_mut().add_address_range(0x80000000, 0x80100000);
//! tracer.filter_mut().set_max_traces(10000);
//!
//! // Traces are automatically collected during execution
//! // Export traces
//! let json = tracer.export_json()?;
//! ```
//!
//! # Filtering
//!
//! Filter which instructions are traced:
//!
//! ```rust,no_run
//! let mut filter = tracer.filter_mut();
//!
//! // Include specific address ranges
//! filter.add_address_range(0x80000000, 0x80100000);
//!
//! // Include specific addresses
//! filter.add_address(0x80001000);
//!
//! // Exclude addresses
//! filter.exclude_address(0x80002000);
//!
//! // Include entire functions
//! filter.add_function(0x80003000);
//!
//! // Limit number of traces
//! filter.set_max_traces(10000);
//! ```

use gcrecomp_core::runtime::context::CpuContext;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

/// Trace of a single instruction execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionTrace {
    /// Instruction address
    pub address: u32,
    /// Instruction mnemonic/description
    pub instruction: String,
    /// Register state (GPR r0-r31)
    pub registers: [u32; 32],
    /// Program counter
    pub pc: u32,
    /// Link register
    pub lr: u32,
    /// Count register
    pub ctr: u32,
    /// Condition register
    pub cr: u32,
    /// Timestamp (nanoseconds since epoch)
    pub timestamp: u64,
}

impl InstructionTrace {
    /// Create a new instruction trace from CPU context.
    pub fn new(address: u32, instruction: String, ctx: &CpuContext) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default() // Should never fail, but handle gracefully
            .as_nanos() as u64;

        Self {
            address,
            instruction,
            registers: ctx.gpr,
            pc: ctx.pc,
            lr: ctx.lr,
            ctr: ctx.ctr,
            cr: ctx.cr,
            timestamp,
        }
    }
}

/// Filter for instruction traces.
#[derive(Debug, Clone)]
pub struct InstructionTraceFilter {
    /// Address ranges to include (start, end)
    address_ranges: Vec<(u32, u32)>,
    /// Addresses to include
    addresses: HashSet<u32>,
    /// Addresses to exclude
    exclude_addresses: HashSet<u32>,
    /// Function addresses to include
    function_addresses: HashSet<u32>,
    /// Maximum number of traces to keep
    max_traces: Option<usize>,
}

impl InstructionTraceFilter {
    /// Create a new filter that accepts all traces.
    pub fn new() -> Self {
        Self {
            address_ranges: Vec::new(),
            addresses: HashSet::new(),
            exclude_addresses: HashSet::new(),
            function_addresses: HashSet::new(),
            max_traces: None,
        }
    }

    /// Add an address range to include.
    pub fn add_address_range(&mut self, start: u32, end: u32) {
        self.address_ranges.push((start, end));
    }

    /// Add a specific address to include.
    pub fn add_address(&mut self, address: u32) {
        self.addresses.insert(address);
    }

    /// Add an address to exclude.
    pub fn exclude_address(&mut self, address: u32) {
        self.exclude_addresses.insert(address);
    }

    /// Add a function address to include (includes all instructions in function).
    pub fn add_function(&mut self, function_address: u32) {
        self.function_addresses.insert(function_address);
    }

    /// Set maximum number of traces to keep.
    pub fn set_max_traces(&mut self, max: usize) {
        self.max_traces = Some(max);
    }

    /// Check if a trace should be included.
    pub fn should_trace(&self, address: u32) -> bool {
        // Check exclusions first
        if self.exclude_addresses.contains(&address) {
            return false;
        }

        // Check specific addresses
        if self.addresses.contains(&address) {
            return true;
        }

        // Check address ranges
        for (start, end) in &self.address_ranges {
            if address >= *start && address <= *end {
                return true;
            }
        }

        // Check function addresses (would need function boundaries)
        // For now, just check if address matches
        if self.function_addresses.contains(&address) {
            return true;
        }

        // If no filters set, include all
        if self.addresses.is_empty()
            && self.address_ranges.is_empty()
            && self.function_addresses.is_empty()
        {
            return true;
        }

        false
    }
}

impl Default for InstructionTraceFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Instruction tracer that collects instruction execution traces.
pub struct InstructionTracer {
    /// Collected traces
    traces: Vec<InstructionTrace>,
    /// Filter for traces
    filter: InstructionTraceFilter,
    /// Whether tracing is enabled
    enabled: bool,
}

impl InstructionTracer {
    /// Create a new instruction tracer.
    pub fn new() -> Self {
        Self {
            traces: Vec::new(),
            filter: InstructionTraceFilter::new(),
            enabled: false,
        }
    }

    /// Enable or disable tracing.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if tracing is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get the trace filter.
    pub fn filter_mut(&mut self) -> &mut InstructionTraceFilter {
        &mut self.filter
    }

    /// Trace an instruction execution.
    ///
    /// # Arguments
    /// * `address` - Instruction address
    /// * `instruction` - Instruction description
    /// * `ctx` - CPU context
    pub fn trace(&mut self, address: u32, instruction: String, ctx: &CpuContext) {
        if !self.enabled {
            return;
        }

        if !self.filter.should_trace(address) {
            return;
        }

        let trace = InstructionTrace::new(address, instruction, ctx);

        // Check max traces limit
        if let Some(max) = self.filter.max_traces {
            if self.traces.len() >= max {
                // Remove oldest trace (FIFO)
                self.traces.remove(0);
            }
        }

        self.traces.push(trace);
    }

    /// Get all collected traces.
    pub fn traces(&self) -> &[InstructionTrace] {
        &self.traces
    }

    /// Clear all traces.
    pub fn clear(&mut self) {
        self.traces.clear();
    }

    /// Export traces to JSON.
    pub fn export_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.traces)
    }

    /// Export traces to a compressed format (using zstd).
    #[cfg(feature = "compression")]
    pub fn export_compressed(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let json = self.export_json()?;
        let compressed = zstd::encode_all(json.as_bytes(), 3)?;
        Ok(compressed)
    }

    /// Get trace count.
    pub fn trace_count(&self) -> usize {
        self.traces.len()
    }
}

impl Default for InstructionTracer {
    fn default() -> Self {
        Self::new()
    }
}
