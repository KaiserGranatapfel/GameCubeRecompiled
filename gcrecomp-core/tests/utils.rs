//! Test Utilities
//!
//! This module provides utilities for testing recompiled code, including
//! mock contexts, assertion helpers, and test data generators.

use gcrecomp_core::runtime::context::CpuContext;
use gcrecomp_core::runtime::memory::MemoryManager;

/// Create a mock CPU context for testing.
pub fn mock_cpu_context() -> CpuContext {
    let mut ctx = CpuContext::new();
    // Set some default values
    ctx.pc = 0x80000000;
    ctx.lr = 0;
    ctx.ctr = 0;
    ctx.cr = 0;
    ctx
}

/// Create a mock CPU context with specific register values.
pub fn mock_cpu_context_with_registers(registers: &[(u8, u32)]) -> CpuContext {
    let mut ctx = mock_cpu_context();
    for (reg, value) in registers {
        ctx.set_register(*reg, *value);
    }
    ctx
}

/// Create a mock memory manager for testing.
pub fn mock_memory_manager() -> MemoryManager {
    MemoryManager::new()
}

/// Create a mock memory manager with initial data.
pub fn mock_memory_manager_with_data(data: &[(u32, &[u8])]) -> MemoryManager {
    let mut memory = MemoryManager::new();
    for (address, bytes) in data {
        memory.write_bytes(*address, bytes).unwrap();
    }
    memory
}

/// Assert that two CPU contexts have the same register values.
pub fn assert_registers_equal(expected: &CpuContext, actual: &CpuContext, message: &str) {
    for i in 0..32 {
        let expected_val = expected.get_register(i);
        let actual_val = actual.get_register(i);
        assert_eq!(
            expected_val, actual_val,
            "{}: Register r{} differs: expected 0x{:08X}, got 0x{:08X}",
            message, i, expected_val, actual_val
        );
    }
}

/// Assert that two CPU contexts have the same state (registers + special registers).
pub fn assert_context_equal(expected: &CpuContext, actual: &CpuContext, message: &str) {
    assert_registers_equal(expected, actual, message);
    assert_eq!(
        expected.pc, actual.pc,
        "{}: PC differs: expected 0x{:08X}, got 0x{:08X}",
        message, expected.pc, actual.pc
    );
    assert_eq!(
        expected.lr, actual.lr,
        "{}: LR differs: expected 0x{:08X}, got 0x{:08X}",
        message, expected.lr, actual.lr
    );
    assert_eq!(
        expected.ctr, actual.ctr,
        "{}: CTR differs: expected 0x{:08X}, got 0x{:08X}",
        message, expected.ctr, actual.ctr
    );
}

/// Assert that memory regions are equal.
pub fn assert_memory_equal(
    expected: &MemoryManager,
    actual: &MemoryManager,
    address: u32,
    size: usize,
    message: &str,
) {
    let expected_data = expected.read_bytes(address, size).unwrap();
    let actual_data = actual.read_bytes(address, size).unwrap();
    assert_eq!(
        expected_data, actual_data,
        "{}: Memory at 0x{:08X} differs",
        message, address
    );
}

/// Generate test data for a memory region.
pub fn generate_test_data(size: usize) -> Vec<u8> {
    (0..size).map(|i| (i % 256) as u8).collect()
}

/// Generate random test data.
pub fn generate_random_data(size: usize) -> Vec<u8> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut data = Vec::with_capacity(size);
    let mut hasher = DefaultHasher::new();
    for i in 0..size {
        (i as u64).hash(&mut hasher);
        data.push((hasher.finish() % 256) as u8);
    }
    data
}

/// Helper for parameterized tests.
pub struct TestCase<T> {
    pub name: String,
    pub input: T,
    pub expected_output: Option<T>,
}

impl<T> TestCase<T> {
    pub fn new(name: impl Into<String>, input: T) -> Self {
        Self {
            name: name.into(),
            input,
            expected_output: None,
        }
    }

    pub fn with_expected(mut self, expected: T) -> Self {
        self.expected_output = Some(expected);
        self
    }
}
