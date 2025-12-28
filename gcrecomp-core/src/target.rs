//! Target Specification System
//!
//! This module provides target specification for code generation.

use serde::{Deserialize, Serialize};

/// Target architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetArch {
    /// x86_64 (default)
    X86_64,
    /// ARM64
    Arm64,
    /// ARM
    Arm,
    /// Original GameCube PowerPC
    PowerPC,
}

/// Target specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetSpec {
    /// Target architecture
    pub arch: TargetArch,
    /// OS (if applicable)
    pub os: Option<String>,
    /// Enable SIMD
    pub enable_simd: bool,
    /// Enable hardware-specific features
    pub hardware_features: Vec<String>,
}

impl Default for TargetSpec {
    fn default() -> Self {
        Self {
            arch: TargetArch::X86_64,
            os: Some("linux".to_string()),
            enable_simd: true,
            hardware_features: Vec::new(),
        }
    }
}

/// Validate target specification.
pub fn validate_target(target: &TargetSpec) -> Result<(), String> {
    // Validate target configuration
    match target.arch {
        TargetArch::X86_64 => {
            if target.enable_simd && target.os.is_none() {
                return Err("SIMD requires OS specification".to_string());
            }
        }
        TargetArch::PowerPC => {
            // Original hardware target
        }
        _ => {
            // Other architectures
        }
    }
    Ok(())
}
