//! Functional Test Infrastructure
//!
//! This module provides a test harness for testing recompiled functions.

use crate::tests::utils::*;
use gcrecomp_core::runtime::context::CpuContext;
use gcrecomp_core::runtime::memory::MemoryManager;

/// Test harness for recompiled functions.
pub struct FunctionTestHarness {
    /// CPU context
    pub ctx: CpuContext,
    /// Memory manager
    pub memory: MemoryManager,
}

impl FunctionTestHarness {
    /// Create a new test harness.
    pub fn new() -> Self {
        Self {
            ctx: mock_cpu_context(),
            memory: mock_memory_manager(),
        }
    }

    /// Create a test harness with initial state.
    pub fn with_state(
        ctx: CpuContext,
        memory: MemoryManager,
    ) -> Self {
        Self { ctx, memory }
    }

    /// Execute a recompiled function.
    ///
    /// # Arguments
    /// * `func` - Function to execute (takes ctx and memory, returns Result<Option<u32>>)
    pub fn execute<F>(&mut self, func: F) -> anyhow::Result<Option<u32>>
    where
        F: FnOnce(&mut CpuContext, &mut MemoryManager) -> anyhow::Result<Option<u32>>,
    {
        func(&mut self.ctx, &mut self.memory)
    }

    /// Assert register value.
    pub fn assert_register(&self, reg: u8, expected: u32, message: &str) {
        let actual = self.ctx.get_register(reg);
        assert_eq!(
            expected, actual,
            "{}: Register r{}: expected 0x{:08X}, got 0x{:08X}",
            message, reg, expected, actual
        );
    }

    /// Assert memory value.
    pub fn assert_memory(&self, address: u32, expected: &[u8], message: &str) {
        let actual = self.memory.read_bytes(address, expected.len()).unwrap();
        assert_eq!(
            expected, &actual,
            "{}: Memory at 0x{:08X} differs",
            message, address
        );
    }

    /// Assert return value.
    pub fn assert_return_value(&self, expected: Option<u32>, message: &str) {
        let actual = self.ctx.get_register(3); // r3 is return value
        if let Some(expected_val) = expected {
            assert_eq!(
                expected_val, actual,
                "{}: Return value: expected 0x{:08X}, got 0x{:08X}",
                message, expected_val, actual
            );
        }
    }
}

impl Default for FunctionTestHarness {
    fn default() -> Self {
        Self::new()
    }
}

/// Parameterized test runner.
pub fn run_parameterized_test<F, I, O>(
    test_cases: Vec<TestCase<I>>,
    test_func: F,
) where
    F: Fn(&TestCase<I>) -> anyhow::Result<O>,
    I: std::fmt::Debug,
    O: std::fmt::Debug + PartialEq,
{
    for test_case in test_cases {
        match test_func(&test_case) {
            Ok(result) => {
                if let Some(expected) = &test_case.expected_output {
                    assert_eq!(
                        &result, expected,
                        "Test case '{}' failed: expected {:?}, got {:?}",
                        test_case.name, expected, result
                    );
                }
            }
            Err(e) => {
                panic!("Test case '{}' failed with error: {}", test_case.name, e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harness_creation() {
        let harness = FunctionTestHarness::new();
        assert_eq!(harness.ctx.pc, 0x80000000);
    }

    #[test]
    fn test_register_assertion() {
        let mut harness = FunctionTestHarness::new();
        harness.ctx.set_register(3, 0x12345678);
        harness.assert_register(3, 0x12345678, "test");
    }
}

