use log::{info, warn};

use super::heap::ArenaAllocator;
use super::interrupt::InterruptSystem;
use super::timer::OsTimer;
use crate::runtime::context::CpuContext;
use crate::runtime::memory::MemoryManager;

/// Full OS state for the recompiled GameCube runtime.
pub struct OsState {
    pub arena: ArenaAllocator,
    pub timer: OsTimer,
    pub interrupts: InterruptSystem,
    pub console_type: u32,
    pub initialized: bool,
}

impl OsState {
    pub fn new() -> Self {
        Self {
            arena: ArenaAllocator::new(),
            timer: OsTimer::new(),
            interrupts: InterruptSystem::new(),
            console_type: 0x10000006, // Retail GameCube (HW2)
            initialized: false,
        }
    }
}

impl Default for OsState {
    fn default() -> Self {
        Self::new()
    }
}

/// OSInit - Operating system initialization.
/// Sets up the arena allocator, timer, and interrupt system.
pub fn os_init(os: &mut OsState, memory: &mut MemoryManager) {
    info!("OSInit called");
    os.timer.reset();
    os.interrupts.disable_all();
    os.arena.reset();
    os.initialized = true;

    // Write OS globals into low memory (matching real GameCube OS)
    // 0x800000F8: Bus clock speed (162 MHz)
    let _ = memory.write_u32(0x800000F8, 162_000_000);
    // 0x800000FC: CPU clock speed (486 MHz)
    let _ = memory.write_u32(0x800000FC, 486_000_000);
    // 0x80000028: Memory size (24 MB)
    let _ = memory.write_u32(0x80000028, 24 * 1024 * 1024);
    // 0x800000CC: Console type
    let _ = memory.write_u32(0x800000CC, os.console_type);

    info!("OSInit complete: arena ready, timer started");
}

/// OSReport - Debug output function.
pub fn os_report(message: &str) {
    info!("OSReport: {}", message);
}

/// OSFatal - Fatal error handler.
pub fn os_fatal(message: &str) {
    warn!("OSFatal: {}", message);
    std::process::exit(1);
}

/// OSGetConsoleType - Returns console hardware revision.
pub fn os_get_console_type(os: &OsState) -> u32 {
    os.console_type
}

/// OSDisableInterrupts - Disable all maskable interrupts, return previous state.
pub fn os_disable_interrupts(os: &mut OsState) -> u32 {
    let prev = if os.interrupts.enabled() { 1 } else { 0 };
    os.interrupts.set_master_enable(false);
    prev
}

/// OSRestoreInterrupts - Restore interrupt enable state.
pub fn os_restore_interrupts(os: &mut OsState, prev: u32) {
    os.interrupts.set_master_enable(prev != 0);
}

/// OSAllocFromArenaLo - Allocate memory from low end of arena.
pub fn os_alloc_from_arena_lo(os: &mut OsState, size: u32, align: u32) -> u32 {
    os.arena.alloc_lo(size, align)
}

/// OSAllocFromArenaHi - Allocate memory from high end of arena.
pub fn os_alloc_from_arena_hi(os: &mut OsState, size: u32, align: u32) -> u32 {
    os.arena.alloc_hi(size, align)
}

/// OSGetArenaLo - Get current low arena pointer.
pub fn os_get_arena_lo(os: &OsState) -> u32 {
    os.arena.lo_cursor()
}

/// OSGetArenaHi - Get current high arena pointer.
pub fn os_get_arena_hi(os: &OsState) -> u32 {
    os.arena.hi_cursor()
}

/// OSSetArenaLo - Set the low arena pointer directly.
pub fn os_set_arena_lo(os: &mut OsState, addr: u32) {
    os.arena.set_lo_cursor(addr);
}

/// OSSetArenaHi - Set the high arena pointer directly.
pub fn os_set_arena_hi(os: &mut OsState, addr: u32) {
    os.arena.set_hi_cursor(addr);
}

/// Dispatch an SDK call by symbol name. Returns true if handled.
pub fn dispatch_sdk_call(
    name: &str,
    ctx: &mut CpuContext,
    memory: &mut MemoryManager,
    os: &mut OsState,
) -> bool {
    match name {
        "OSInit" => {
            os_init(os, memory);
            true
        }
        "OSReport" => {
            let addr = ctx.get_register(3);
            let msg = read_c_string(memory, addr);
            os_report(&msg);
            true
        }
        "OSFatal" => {
            let addr = ctx.get_register(3);
            let msg = read_c_string(memory, addr);
            os_fatal(&msg);
            true
        }
        "OSGetConsoleType" => {
            let val = os_get_console_type(os);
            ctx.set_register(3, val);
            true
        }
        "OSDisableInterrupts" => {
            let prev = os_disable_interrupts(os);
            ctx.set_register(3, prev);
            true
        }
        "OSRestoreInterrupts" => {
            let prev = ctx.get_register(3);
            os_restore_interrupts(os, prev);
            true
        }
        "OSAllocFromArenaLo" => {
            let size = ctx.get_register(3);
            let align = ctx.get_register(4);
            let addr = os_alloc_from_arena_lo(os, size, align);
            ctx.set_register(3, addr);
            true
        }
        "OSAllocFromArenaHi" => {
            let size = ctx.get_register(3);
            let align = ctx.get_register(4);
            let addr = os_alloc_from_arena_hi(os, size, align);
            ctx.set_register(3, addr);
            true
        }
        "OSGetArenaLo" => {
            ctx.set_register(3, os_get_arena_lo(os));
            true
        }
        "OSGetArenaHi" => {
            ctx.set_register(3, os_get_arena_hi(os));
            true
        }
        "OSSetArenaLo" => {
            let addr = ctx.get_register(3);
            os_set_arena_lo(os, addr);
            true
        }
        "OSSetArenaHi" => {
            let addr = ctx.get_register(3);
            os_set_arena_hi(os, addr);
            true
        }
        "OSGetTick" => {
            ctx.set_register(3, os.timer.get_tick());
            true
        }
        "OSGetTime" => {
            let time = os.timer.get_time();
            ctx.set_register(3, (time >> 32) as u32);
            ctx.set_register(4, time as u32);
            true
        }
        _ => false,
    }
}

/// Read a null-terminated C string from memory at the given GC address.
pub fn read_c_string(memory: &MemoryManager, addr: u32) -> String {
    let mut result = Vec::new();
    let mut offset = addr;
    loop {
        match memory.read_u8(offset) {
            Ok(0) | Err(_) => break,
            Ok(b) => {
                result.push(b);
                offset = offset.wrapping_add(1);
                if result.len() > 4096 {
                    break; // Safety limit
                }
            }
        }
    }
    String::from_utf8_lossy(&result).into_owned()
}
