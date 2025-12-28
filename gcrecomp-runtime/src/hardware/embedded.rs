//! Embedded Target Support
//!
//! This module provides support for embedded targets with no_std.

#[cfg(not(feature = "std"))]
pub mod no_std_runtime {
    // Minimal runtime for embedded systems
    // Would include basic memory management and execution context
}

/// Embedded target configuration.
pub struct EmbeddedConfig {
    /// Target architecture
    pub arch: String,
    /// Memory size
    pub memory_size: usize,
    /// Stack size
    pub stack_size: usize,
}

impl Default for EmbeddedConfig {
    fn default() -> Self {
        Self {
            arch: "arm".to_string(),
            memory_size: 1024 * 1024, // 1MB
            stack_size: 4096,
        }
    }
}
