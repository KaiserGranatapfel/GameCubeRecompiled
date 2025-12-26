//! Register Allocation
//!
//! This module provides register allocation for code generation, mapping PowerPC registers
//! to Rust variables. In a full implementation, this would use graph coloring or linear scan
//! algorithms for optimal register allocation.
//!
//! # Memory Optimizations
//! - Uses `HashMap` for register mapping (efficient lookup)
//! - String interning could be added for repeated register names
//!
//! # Allocation Strategy
//! Currently uses a simple mapping strategy. A full implementation would:
//! - Use graph coloring to minimize register pressure
//! - Implement register spilling when registers are exhausted
//! - Track register liveness to optimize allocation

use std::collections::HashMap;

/// Register allocator for mapping PowerPC registers to Rust variables.
///
/// Maps PowerPC general-purpose registers (r0-r31) to Rust variable names.
/// In a full implementation, would use graph coloring or linear scan algorithms.
pub struct RegisterAllocator {
    /// Map from PowerPC register number to Rust variable name
    register_map: HashMap<u8, String>,
    /// Next temporary variable number (for unique variable names)
    next_temp: usize,
    /// List of spilled registers (registers moved to stack)
    spilled_registers: Vec<u8>,
}

impl RegisterAllocator {
    /// Create a new register allocator.
    ///
    /// # Returns
    /// `RegisterAllocator` - New register allocator instance
    ///
    /// # Examples
    /// ```rust
    /// let mut allocator = RegisterAllocator::new();
    /// ```
    #[inline] // Constructor - simple, may be inlined
    pub fn new() -> Self {
        Self {
            register_map: HashMap::new(),
            next_temp: 0usize,
            spilled_registers: Vec::new(),
        }
    }
    
    /// Allocate a Rust variable name for a PowerPC register.
    ///
    /// # Algorithm
    /// Maps PowerPC register to Rust variable name. If register hasn't been allocated,
    /// creates a new variable name. Otherwise, returns existing variable name.
    ///
    /// # Arguments
    /// * `ppc_reg` - PowerPC register number (0-31)
    ///
    /// # Returns
    /// `String` - Rust variable name for the register
    ///
    /// # Examples
    /// ```rust
    /// let var_name = allocator.allocate_register(3); // Returns "r3"
    /// ```
    #[inline] // Hot path - may be inlined
    pub fn allocate_register(&mut self, ppc_reg: u8) -> String {
        // Map PowerPC register to Rust variable
        // In a full implementation, would use graph coloring or linear scan
        self.register_map
            .entry(ppc_reg)
            .or_insert_with(|| {
                let name: String = format!("r{}", ppc_reg);
                self.next_temp = self.next_temp.wrapping_add(1);
                name
            })
            .clone()
    }
    
    /// Spill a register to the stack.
    ///
    /// # Algorithm
    /// When registers are exhausted, spill a register to stack memory.
    /// Returns a stack variable name for the spilled register.
    ///
    /// # Arguments
    /// * `reg` - PowerPC register number to spill
    ///
    /// # Returns
    /// `String` - Stack variable name for the spilled register
    ///
    /// # Examples
    /// ```rust
    /// let stack_var = allocator.spill_register(3); // Returns "spilled_r3"
    /// ```
    #[inline] // May be called frequently
    pub fn spill_register(&mut self, reg: u8) -> String {
        // Spill register to stack
        let stack_var: String = format!("spilled_r{}", reg);
        self.spilled_registers.push(reg);
        stack_var
    }
}

impl Default for RegisterAllocator {
    #[inline] // Simple default implementation
    fn default() -> Self {
        Self::new()
    }
}
