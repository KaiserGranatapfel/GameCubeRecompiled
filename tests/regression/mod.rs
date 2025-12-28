//! Regression Test Suite
//!
//! This module provides a regression test runner for recompiled code.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Test case for regression testing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionTestCase {
    /// Test case name
    pub name: String,
    /// Function address to test
    pub function_address: u32,
    /// Initial CPU context state
    pub initial_context: TestContext,
    /// Initial memory state (address -> data)
    pub initial_memory: HashMap<u32, Vec<u8>>,
    /// Expected final CPU context state
    pub expected_context: TestContext,
    /// Expected final memory state
    pub expected_memory: HashMap<u32, Vec<u8>>,
}

/// Simplified CPU context for test cases.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestContext {
    /// GPR registers (r0-r31)
    pub gpr: [u32; 32],
    /// Program counter
    pub pc: u32,
    /// Link register
    pub lr: u32,
    /// Count register
    pub ctr: u32,
    /// Condition register
    pub cr: u32,
}

impl From<&gcrecomp_core::runtime::context::CpuContext> for TestContext {
    fn from(ctx: &gcrecomp_core::runtime::context::CpuContext) -> Self {
        Self {
            gpr: ctx.gpr,
            pc: ctx.pc,
            lr: ctx.lr,
            ctr: ctx.ctr,
            cr: ctx.cr,
        }
    }
}

impl From<TestContext> for gcrecomp_core::runtime::context::CpuContext {
    fn from(tc: TestContext) -> Self {
        let mut ctx = gcrecomp_core::runtime::context::CpuContext::new();
        ctx.gpr = tc.gpr;
        ctx.pc = tc.pc;
        ctx.lr = tc.lr;
        ctx.ctr = tc.ctr;
        ctx.cr = tc.cr;
        ctx
    }
}

/// Regression test runner.
pub struct RegressionTestRunner {
    /// Test cases
    test_cases: Vec<RegressionTestCase>,
    /// Test results cache
    results_cache: HashMap<String, bool>,
}

impl RegressionTestRunner {
    /// Create a new regression test runner.
    pub fn new() -> Self {
        Self {
            test_cases: Vec::new(),
            results_cache: HashMap::new(),
        }
    }

    /// Load test cases from a file.
    pub fn load_from_file(&mut self, path: &Path) -> anyhow::Result<()> {
        let content = std::fs::read_to_string(path)?;
        let test_cases: Vec<RegressionTestCase> = serde_json::from_str(&content)?;
        self.test_cases.extend(test_cases);
        Ok(())
    }

    /// Load test cases from a directory.
    pub fn load_from_directory(&mut self, dir: &Path) -> anyhow::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                self.load_from_file(&path)?;
            }
        }
        Ok(())
    }

    /// Add a test case.
    pub fn add_test_case(&mut self, test_case: RegressionTestCase) {
        self.test_cases.push(test_case);
    }

    /// Run all test cases.
    pub fn run_all<F>(&mut self, executor: F) -> RegressionTestResults
    where
        F: Fn(u32, &mut gcrecomp_core::runtime::context::CpuContext, &mut gcrecomp_core::runtime::memory::MemoryManager) -> anyhow::Result<Option<u32>>,
    {
        let mut results = RegressionTestResults::new();

        for test_case in &self.test_cases {
            // Check cache
            if let Some(&cached_result) = self.results_cache.get(&test_case.name) {
                if cached_result {
                    results.passed += 1;
                } else {
                    results.failed += 1;
                }
                continue;
            }

            // Run test case
            let result = self.run_test_case(test_case, &executor);
            if result.passed {
                results.passed += 1;
                self.results_cache.insert(test_case.name.clone(), true);
            } else {
                results.failed += 1;
                results.failures.push(TestFailure {
                    test_name: test_case.name.clone(),
                    differences: result.differences,
                });
                self.results_cache.insert(test_case.name.clone(), false);
            }
        }

        results
    }

    /// Run a single test case.
    fn run_test_case<F>(
        &self,
        test_case: &RegressionTestCase,
        executor: F,
    ) -> gcrecomp_core::tests::comparison::ComparisonResult
    where
        F: Fn(u32, &mut gcrecomp_core::runtime::context::CpuContext, &mut gcrecomp_core::runtime::memory::MemoryManager) -> anyhow::Result<Option<u32>>,
    {
        use gcrecomp_core::runtime::context::CpuContext;
        use gcrecomp_core::runtime::memory::MemoryManager;
        use gcrecomp_core::tests::comparison::compare_execution_results;

        // Setup initial state
        let mut ctx: CpuContext = test_case.initial_context.clone().into();
        let mut memory = MemoryManager::new();

        // Initialize memory
        for (address, data) in &test_case.initial_memory {
            memory.write_bytes(*address, data).unwrap();
        }

        // Execute function
        let _result = executor(test_case.function_address, &mut ctx, &mut memory).unwrap();

        // Compare results
        let expected_ctx: CpuContext = test_case.expected_context.clone().into();
        let expected_memory = MemoryManager::new();
        // Note: Would need to set expected_memory state

        compare_execution_results(
            &expected_ctx,
            &ctx,
            &expected_memory,
            &memory,
            None,
            0.0001, // Float tolerance
        )
    }

    /// Save test results to a file.
    pub fn save_results(&self, path: &Path, results: &RegressionTestResults) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(results)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

impl Default for RegressionTestRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Results of regression test run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionTestResults {
    /// Number of passed tests
    pub passed: usize,
    /// Number of failed tests
    pub failed: usize,
    /// List of failures
    pub failures: Vec<TestFailure>,
}

impl RegressionTestResults {
    pub fn new() -> Self {
        Self {
            passed: 0,
            failed: 0,
            failures: Vec::new(),
        }
    }

    pub fn total(&self) -> usize {
        self.passed + self.failed
    }

    pub fn success_rate(&self) -> f64 {
        if self.total() == 0 {
            return 0.0;
        }
        self.passed as f64 / self.total() as f64
    }
}

/// Test failure information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFailure {
    /// Test name
    pub test_name: String,
    /// Differences found
    pub differences: Vec<String>,
}

