//! Public Modding API
//!
//! This module provides the public API for mod developers to interact with
//! the GCRecomp runtime system.
//!
//! # Overview
//!
//! This module re-exports all the types and traits needed to create mods for GCRecomp.
//! Mod developers should import from this module:
//!
//! ```rust,no_run
//! use gcrecomp_runtime::mods::api::*;
//! ```
//!
//! # Quick Start
//!
//! 1. Implement the `Mod` trait for your mod
//! 2. Export `mod_metadata()` and `mod_init()` functions
//! 3. Build as a `cdylib` library
//! 4. Create a metadata file (`.json` or `.toml`)
//! 5. Place in the mods directory and load via `Runtime::load_mods()`
//!
//! See the parent module documentation for detailed examples and best practices.

pub use crate::mods::hooks::{FunctionHook, HookManager, HookResult};
pub use crate::mods::loader::ModLoader;
pub use crate::mods::r#mod::{Mod, ModMetadata, ModRegistry};

/// Re-export runtime types for mod convenience
pub use gcrecomp_core::runtime::context::CpuContext;
pub use gcrecomp_core::runtime::memory::MemoryManager;
