//! PowerPC Calling Convention Helpers
//!
//! This module provides utilities for handling PowerPC calling conventions.
//!
//! # PowerPC Calling Convention
//!
//! - First 8 arguments passed in registers r3-r10
//! - Additional arguments passed on the stack
//! - Return value in register r3
//! - Stack pointer in register r1
//! - Link register (LR) stores return address
//!
//! # API Reference
//!
//! ## CallingConvention
//!
//! Helper functions for PowerPC calling convention.
//!
//! ```rust,no_run
//! use gcrecomp_core::runtime::calling::CallingConvention;
//!
//! CallingConvention::setup_stack_frame(&mut ctx, frame_size);
//! let arg = CallingConvention::get_argument(&ctx, 0);
//! CallingConvention::set_return_value(&mut ctx, value);
//! ```
//!
//! ## Methods
//!
//! - `setup_stack_frame()`: Allocate stack frame
//! - `teardown_stack_frame()`: Deallocate stack frame
//! - `get_argument()`: Get function argument from register
//! - `set_return_value()`: Set function return value
//! - `get_return_value()`: Get function return value

use crate::runtime::context::CpuContext;

/// PowerPC calling convention helper
pub struct CallingConvention;

impl CallingConvention {
    /// Setup stack frame for function call
    /// PowerPC uses r1 as stack pointer
    pub fn setup_stack_frame(ctx: &mut CpuContext, frame_size: u32) {
        // Save old stack pointer
        let old_sp = ctx.get_register(1);

        // Allocate new stack frame
        let new_sp = old_sp.wrapping_sub(frame_size);
        ctx.set_register(1, new_sp);

        // Store old stack pointer in the new frame (standard PowerPC convention)
        // This would typically be done with stwu instruction
    }

    /// Teardown stack frame
    pub fn teardown_stack_frame(ctx: &mut CpuContext, frame_size: u32) {
        // Restore stack pointer
        let current_sp = ctx.get_register(1);
        let old_sp = current_sp.wrapping_add(frame_size);
        ctx.set_register(1, old_sp);
    }

    /// Get function argument from register
    /// PowerPC passes first 8 arguments in r3-r10
    pub fn get_argument(ctx: &CpuContext, arg_num: u8) -> u32 {
        if arg_num < 8 {
            ctx.get_register(3 + arg_num)
        } else {
            // Arguments beyond 8 are passed on the stack
            // This would require reading from the stack frame
            0
        }
    }

    /// Set function return value
    /// PowerPC returns values in r3
    pub fn set_return_value(ctx: &mut CpuContext, value: u32) {
        ctx.set_register(3, value);
    }

    /// Get return value
    pub fn get_return_value(ctx: &CpuContext) -> u32 {
        ctx.get_register(3)
    }
}
