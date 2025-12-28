//! Memory Access Tracking
//!
//! This module provides memory access tracking for debugging and analysis.
//!
//! # Usage
//!
//! ```rust,no_run
//! use gcrecomp_runtime::tracing::memory::MemoryAccessTracker;
//!
//! let mut tracker = MemoryAccessTracker::new();
//!
//! // Enable tracking
//! tracker.set_enabled(true);
//!
//! // Mark initialized memory regions
//! tracker.mark_initialized(0x80000000, 1024 * 1024);
//!
//! // Track accesses (typically done automatically)
//! tracker.track_read(address, size, value, function_address);
//! tracker.track_write(address, size, value, function_address);
//!
//! // Analyze patterns
//! let analysis = tracker.analyze_patterns();
//!
//! // Get uninitialized accesses
//! let uninitialized = tracker.uninitialized_accesses();
//!
//! // Export accesses
//! let json = tracker.export_json()?;
//! ```
//!
//! # Memory Analysis
//!
//! Detect uninitialized memory access:
//!
//! ```rust,no_run
//! let uninitialized = tracker.uninitialized_accesses();
//! if !uninitialized.is_empty() {
//!     println!("Warning: {} uninitialized memory accesses", uninitialized.len());
//! }
//! ```

use gcrecomp_core::runtime::memory::MemoryManager;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

/// Type of memory access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryAccessType {
    /// Read access
    Read,
    /// Write access
    Write,
}

/// Logged memory access information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryAccess {
    /// Memory address
    pub address: u32,
    /// Access size in bytes
    pub size: usize,
    /// Access type (read or write)
    pub access_type: MemoryAccessType,
    /// Value read or written
    pub value: Vec<u8>,
    /// Timestamp (nanoseconds since epoch)
    pub timestamp: u64,
    /// Function address that performed the access
    pub function_address: Option<u32>,
    /// Whether this access was to uninitialized memory
    pub uninitialized: bool,
}

/// Memory access tracker.
pub struct MemoryAccessTracker {
    /// Logged memory accesses
    accesses: Vec<MemoryAccess>,
    /// Initialized memory regions (address -> size)
    initialized_regions: HashMap<u32, usize>,
    /// Uninitialized access addresses
    uninitialized_accesses: HashSet<u32>,
    /// Access patterns (address -> access count)
    access_patterns: HashMap<u32, usize>,
    /// Whether tracking is enabled
    enabled: bool,
}

impl MemoryAccessTracker {
    /// Create a new memory access tracker.
    pub fn new() -> Self {
        Self {
            accesses: Vec::new(),
            initialized_regions: HashMap::new(),
            uninitialized_accesses: HashSet::new(),
            access_patterns: HashMap::new(),
            enabled: false,
        }
    }

    /// Enable or disable tracking.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if tracking is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Mark a memory region as initialized.
    ///
    /// # Arguments
    /// * `address` - Start address
    /// * `size` - Size in bytes
    pub fn mark_initialized(&mut self, address: u32, size: usize) {
        self.initialized_regions.insert(address, size);
    }

    /// Check if a memory address is initialized.
    fn is_initialized(&self, address: u32, size: usize) -> bool {
        // Check if address falls within any initialized region
        for (&init_addr, &init_size) in &self.initialized_regions {
            if address >= init_addr && address < init_addr + init_size as u32 {
                return true;
            }
            // Also check if initialized region overlaps with access
            if init_addr >= address && init_addr < address + size as u32 {
                return true;
            }
        }
        false
    }

    /// Track a memory read access.
    ///
    /// # Arguments
    /// * `address` - Memory address
    /// * `size` - Access size in bytes
    /// * `value` - Value read
    /// * `function_address` - Function that performed the access
    pub fn track_read(
        &mut self,
        address: u32,
        size: usize,
        value: Vec<u8>,
        function_address: Option<u32>,
    ) {
        if !self.enabled {
            return;
        }

        let uninitialized = !self.is_initialized(address, size);
        if uninitialized {
            self.uninitialized_accesses.insert(address);
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default() // Should never fail, but handle gracefully
            .as_nanos() as u64;

        // Update access patterns
        *self.access_patterns.entry(address).or_insert(0) += 1;

        self.accesses.push(MemoryAccess {
            address,
            size,
            access_type: MemoryAccessType::Read,
            value,
            timestamp,
            function_address,
            uninitialized,
        });
    }

    /// Track a memory write access.
    ///
    /// # Arguments
    /// * `address` - Memory address
    /// * `size` - Access size in bytes
    /// * `value` - Value written
    /// * `function_address` - Function that performed the access
    pub fn track_write(
        &mut self,
        address: u32,
        size: usize,
        value: Vec<u8>,
        function_address: Option<u32>,
    ) {
        if !self.enabled {
            return;
        }

        // Mark as initialized after write
        self.mark_initialized(address, size);

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default() // Should never fail, but handle gracefully
            .as_nanos() as u64;

        // Update access patterns
        *self.access_patterns.entry(address).or_insert(0) += 1;

        self.accesses.push(MemoryAccess {
            address,
            size,
            access_type: MemoryAccessType::Write,
            value,
            timestamp,
            function_address,
            uninitialized: false, // Writes initialize memory
        });
    }

    /// Get all logged accesses.
    pub fn accesses(&self) -> &[MemoryAccess] {
        &self.accesses
    }

    /// Get uninitialized memory access addresses.
    pub fn uninitialized_accesses(&self) -> &HashSet<u32> {
        &self.uninitialized_accesses
    }

    /// Get access patterns (address -> count).
    pub fn access_patterns(&self) -> &HashMap<u32, usize> {
        &self.access_patterns
    }

    /// Analyze memory access patterns.
    ///
    /// Returns statistics about memory access patterns.
    pub fn analyze_patterns(&self) -> MemoryAccessPatternAnalysis {
        let mut read_count = 0;
        let mut write_count = 0;
        let mut total_bytes_read = 0;
        let mut total_bytes_written = 0;
        let mut function_accesses: HashMap<Option<u32>, usize> = HashMap::new();

        for access in &self.accesses {
            match access.access_type {
                MemoryAccessType::Read => {
                    read_count += 1;
                    total_bytes_read += access.size;
                }
                MemoryAccessType::Write => {
                    write_count += 1;
                    total_bytes_written += access.size;
                }
            }
            *function_accesses
                .entry(access.function_address)
                .or_insert(0) += 1;
        }

        MemoryAccessPatternAnalysis {
            total_accesses: self.accesses.len(),
            read_count,
            write_count,
            total_bytes_read,
            total_bytes_written,
            uninitialized_access_count: self.uninitialized_accesses.len(),
            function_accesses,
            most_accessed_addresses: self
                .access_patterns
                .iter()
                .map(|(addr, count)| (*addr, *count))
                .collect(),
        }
    }

    /// Clear all tracked accesses.
    pub fn clear(&mut self) {
        self.accesses.clear();
        self.uninitialized_accesses.clear();
        self.access_patterns.clear();
    }

    /// Export accesses to JSON.
    pub fn export_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.accesses)
    }

    /// Get access count.
    pub fn access_count(&self) -> usize {
        self.accesses.len()
    }
}

impl Default for MemoryAccessTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Analysis of memory access patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryAccessPatternAnalysis {
    /// Total number of accesses
    pub total_accesses: usize,
    /// Number of read accesses
    pub read_count: usize,
    /// Number of write accesses
    pub write_count: usize,
    /// Total bytes read
    pub total_bytes_read: usize,
    /// Total bytes written
    pub total_bytes_written: usize,
    /// Number of uninitialized accesses
    pub uninitialized_access_count: usize,
    /// Access counts by function
    pub function_accesses: HashMap<Option<u32>, usize>,
    /// Most accessed addresses (address -> count)
    pub most_accessed_addresses: Vec<(u32, usize)>,
}
