// Calling convention helpers
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

