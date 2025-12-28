//! Hook Integration for Generated Code
//!
//! This module provides functions that can be called from generated recompiled code
//! to integrate with the hook system.
//!
//! # Overview
//!
//! This module bridges the gap between generated recompiled code and the hook system.
//! The `call_with_hooks` function should be called from generated code instead of
//! directly calling functions, allowing hooks to intercept calls.
//!
//! # Usage
//!
//! The hook manager must be set before any recompiled code is executed:
//!
//! ```rust,no_run
//! use gcrecomp_runtime::mods::hook_integration;
//! use std::sync::{Arc, Mutex};
//!
//! let hook_manager = Arc::new(Mutex::new(HookManager::new()));
//! hook_integration::set_hook_manager(hook_manager);
//! ```
//!
//! Generated code will automatically use `call_with_hooks` when the `hooks` feature
//! is enabled, which checks hooks before and after function execution.

use crate::mods::hooks::{HookManager, HookResult};
use gcrecomp_core::runtime::context::CpuContext;
use gcrecomp_core::runtime::memory::MemoryManager;
use std::sync::{Arc, Mutex};

/// Global hook manager (thread-safe)
static HOOK_MANAGER: Mutex<Option<Arc<Mutex<HookManager>>>> = Mutex::new(None);

/// Set the global hook manager.
///
/// This should be called once during runtime initialization.
pub fn set_hook_manager(manager: Arc<Mutex<HookManager>>) {
    if let Ok(mut guard) = HOOK_MANAGER.lock() {
        *guard = Some(manager);
    } else {
        log::warn!("Hook manager mutex poisoned, cannot set hook manager");
    }
}

/// Get the global hook manager.
fn get_hook_manager() -> Option<Arc<Mutex<HookManager>>> {
    HOOK_MANAGER.lock()
        .map(|guard| guard.clone())
        .unwrap_or_else(|_| {
            log::warn!("Hook manager mutex poisoned, returning None");
            None
        })
}

/// Call a function with hook support.
///
/// This function should be called from generated code instead of directly
/// calling functions. It checks hooks before and after execution.
///
/// # Arguments
/// * `address` - Function address to call
/// * `ctx` - CPU context
/// * `memory` - Memory manager
/// * `func` - The actual function to call (if hooks allow it)
///
/// # Returns
/// `Result<Option<u32>>` - Function return value, or None if skipped
pub fn call_with_hooks<F>(
    address: u32,
    ctx: &mut CpuContext,
    memory: &mut MemoryManager,
    func: F,
) -> anyhow::Result<Option<u32>>
where
    F: FnOnce(&mut CpuContext, &mut MemoryManager) -> anyhow::Result<Option<u32>>,
{
    // Check if hook manager is available
    if let Some(manager) = get_hook_manager() {
        let mut hook_mgr = match manager.lock() {
            Ok(guard) => guard,
            Err(_) => {
                log::warn!("Hook manager mutex poisoned, skipping hook execution");
                return func(ctx, memory);
            }
        };

        // Execute before-call hooks
        match hook_mgr.execute_before_hooks(address, ctx, memory) {
            HookResult::Skip => {
                // Hook handled the call, skip original function
                return Ok(None);
            }
            HookResult::Replace(value) => {
                // Hook replaced the return value
                ctx.set_register(3, value);
                return Ok(Some(value));
            }
            HookResult::Continue => {
                // Continue with normal execution
            }
        }

        // Call the actual function
        let result = func(ctx, memory)?;

        // Execute after-call hooks
        let final_result = match hook_mgr.execute_after_hooks(address, ctx, memory, result) {
            HookResult::Replace(new_value) => Some(new_value),
            _ => result,
        };

        Ok(final_result)
    } else {
        // No hook manager, call function directly
        func(ctx, memory)
    }
}
