//! Hook System for Function Interception
//!
//! This module provides a hook system that allows mods to intercept and
//! modify function calls at runtime.
//!
//! # Overview
//!
//! Hooks allow you to intercept and modify function calls at runtime. This is
//! the primary mechanism for mods to interact with recompiled code.
//!
//! # Creating a Hook
//!
//! Implement the `FunctionHook` trait:
//!
//! ```rust,no_run
//! use gcrecomp_runtime::mods::api::{
//!     FunctionHook, HookResult, CpuContext, MemoryManager
//! };
//!
//! pub struct MyHook;
//!
//! impl FunctionHook for MyHook {
//!     fn before_call(
//!         &self,
//!         address: u32,
//!         ctx: &mut CpuContext,
//!         memory: &mut MemoryManager,
//!     ) -> HookResult {
//!         // Called before the function executes
//!         log::info!("Function 0x{:08X} called", address);
//!         HookResult::Continue // Continue with normal execution
//!     }
//!
//!     fn after_call(
//!         &self,
//!         address: u32,
//!         ctx: &mut CpuContext,
//!         memory: &mut MemoryManager,
//!         return_value: Option<u32>,
//!     ) -> HookResult {
//!         // Called after the function executes
//!         if let Some(ret) = return_value {
//!             log::info!("Function 0x{:08X} returned: 0x{:08X}", address, ret);
//!         }
//!         HookResult::Continue
//!     }
//! }
//! ```
//!
//! # Hook Results
//!
//! - `HookResult::Continue` - Continue with normal execution
//! - `HookResult::Skip` - Skip the original function call (hook handles it)
//! - `HookResult::Replace(value)` - Replace the return value
//!
//! # Registering Hooks
//!
//! Hooks are registered through the `HookManager`, which is accessible via
//! `Runtime::hook_manager()`. Hooks can be registered by function address
//! or function name.

use anyhow::Result;
use std::collections::HashMap;

/// CPU context type (forward declaration - actual type from gcrecomp-core)
pub type CpuContext = gcrecomp_core::runtime::context::CpuContext;

/// Memory manager type (forward declaration - actual type from gcrecomp-core)
pub type MemoryManager = gcrecomp_core::runtime::memory::MemoryManager;

/// Result of a hook execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookResult {
    /// Continue with normal execution
    Continue,
    /// Skip the original function call (hook handled it)
    Skip,
    /// Replace the return value
    Replace(u32),
}

/// Trait for function hooks that can intercept function calls.
pub trait FunctionHook: Send + Sync {
    /// Called before the function is executed.
    ///
    /// # Arguments
    /// * `address` - Function address being called
    /// * `ctx` - CPU context (can be modified)
    /// * `memory` - Memory manager (can be modified)
    ///
    /// # Returns
    /// `HookResult` - How to proceed with the call
    fn before_call(
        &self,
        address: u32,
        ctx: &mut CpuContext,
        memory: &mut MemoryManager,
    ) -> HookResult;

    /// Called after the function has executed.
    ///
    /// # Arguments
    /// * `address` - Function address that was called
    /// * `ctx` - CPU context (can be modified)
    /// * `memory` - Memory manager (can be modified)
    /// * `return_value` - Return value from the function (if any)
    ///
    /// # Returns
    /// `HookResult` - How to proceed (usually Continue)
    fn after_call(
        &self,
        address: u32,
        ctx: &mut CpuContext,
        memory: &mut MemoryManager,
        return_value: Option<u32>,
    ) -> HookResult;
}

/// Manager for function hooks.
///
/// Maintains a registry of hooks for function addresses and names,
/// and executes them when functions are called.
pub struct HookManager {
    /// Hooks registered by function address
    address_hooks: HashMap<u32, Vec<Box<dyn FunctionHook>>>,
    /// Hooks registered by function name
    name_hooks: HashMap<String, Vec<Box<dyn FunctionHook>>>,
    /// Mapping from function names to addresses (for name-based hooks)
    name_to_address: HashMap<String, u32>,
}

impl HookManager {
    /// Create a new hook manager.
    pub fn new() -> Self {
        Self {
            address_hooks: HashMap::new(),
            name_hooks: HashMap::new(),
            name_to_address: HashMap::new(),
        }
    }

    /// Register a hook for a function address.
    ///
    /// # Arguments
    /// * `address` - Function address to hook
    /// * `hook` - Hook implementation
    pub fn register_address_hook(&mut self, address: u32, hook: Box<dyn FunctionHook>) {
        self.address_hooks
            .entry(address)
            .or_insert_with(Vec::new)
            .push(hook);
    }

    /// Register a hook for a function name.
    ///
    /// # Arguments
    /// * `name` - Function name to hook
    /// * `hook` - Hook implementation
    pub fn register_name_hook(&mut self, name: String, hook: Box<dyn FunctionHook>) {
        self.name_hooks
            .entry(name.clone())
            .or_insert_with(Vec::new)
            .push(hook);
    }

    /// Register a function name to address mapping.
    ///
    /// This allows name-based hooks to work.
    ///
    /// # Arguments
    /// * `name` - Function name
    /// * `address` - Function address
    pub fn register_function_name(&mut self, name: String, address: u32) {
        self.name_to_address.insert(name, address);
    }

    /// Execute before-call hooks for a function.
    ///
    /// # Arguments
    /// * `address` - Function address being called
    /// * `ctx` - CPU context
    /// * `memory` - Memory manager
    ///
    /// # Returns
    /// `HookResult` - How to proceed with the call
    pub fn execute_before_hooks(
        &self,
        address: u32,
        ctx: &mut CpuContext,
        memory: &mut MemoryManager,
    ) -> HookResult {
        // Execute address-based hooks
        if let Some(hooks) = self.address_hooks.get(&address) {
            for hook in hooks {
                let result = hook.before_call(address, ctx, memory);
                match result {
                    HookResult::Skip | HookResult::Replace(_) => {
                        return result;
                    }
                    HookResult::Continue => continue,
                }
            }
        }

        // Execute name-based hooks (if we have a name mapping)
        if let Some(name) = self.find_name_for_address(address) {
            if let Some(hooks) = self.name_hooks.get(&name) {
                for hook in hooks {
                    let result = hook.before_call(address, ctx, memory);
                    match result {
                        HookResult::Skip | HookResult::Replace(_) => {
                            return result;
                        }
                        HookResult::Continue => continue,
                    }
                }
            }
        }

        HookResult::Continue
    }

    /// Execute after-call hooks for a function.
    ///
    /// # Arguments
    /// * `address` - Function address that was called
    /// * `ctx` - CPU context
    /// * `memory` - Memory manager
    /// * `return_value` - Return value from the function
    ///
    /// # Returns
    /// `HookResult` - How to proceed (usually Continue)
    pub fn execute_after_hooks(
        &self,
        address: u32,
        ctx: &mut CpuContext,
        memory: &mut MemoryManager,
        return_value: Option<u32>,
    ) -> HookResult {
        // Execute address-based hooks
        if let Some(hooks) = self.address_hooks.get(&address) {
            for hook in hooks {
                let result = hook.after_call(address, ctx, memory, return_value);
                match result {
                    HookResult::Replace(new_value) => {
                        // Update return value in context
                        ctx.set_register(3, new_value);
                        return HookResult::Replace(new_value);
                    }
                    HookResult::Continue => continue,
                    HookResult::Skip => continue, // Skip doesn't make sense after call
                }
            }
        }

        // Execute name-based hooks
        if let Some(name) = self.find_name_for_address(address) {
            if let Some(hooks) = self.name_hooks.get(&name) {
                for hook in hooks {
                    let result = hook.after_call(address, ctx, memory, return_value);
                    match result {
                        HookResult::Replace(new_value) => {
                            ctx.set_register(3, new_value);
                            return HookResult::Replace(new_value);
                        }
                        HookResult::Continue => continue,
                        HookResult::Skip => continue,
                    }
                }
            }
        }

        HookResult::Continue
    }

    /// Find function name for an address.
    fn find_name_for_address(&self, address: u32) -> Option<String> {
        self.name_to_address
            .iter()
            .find(|(_, &addr)| addr == address)
            .map(|(name, _)| name.clone())
    }

    /// Remove all hooks for a function address.
    pub fn unregister_address_hooks(&mut self, address: u32) {
        self.address_hooks.remove(&address);
    }

    /// Remove all hooks for a function name.
    pub fn unregister_name_hooks(&mut self, name: &str) {
        self.name_hooks.remove(name);
    }

    /// Clear all hooks.
    pub fn clear(&mut self) {
        self.address_hooks.clear();
        self.name_hooks.clear();
    }
}

impl Default for HookManager {
    fn default() -> Self {
        Self::new()
    }
}
