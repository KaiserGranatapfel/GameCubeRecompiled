// GameCube SDK stubs
use crate::runtime::memory::MemoryManager;
use log::{info, warn};
use std::sync::{Arc, Mutex};

// Note: Runtime implementations are in gcrecomp-runtime
// This module provides SDK stubs that can be registered with runtime implementations

/// Global memory manager instance (thread-safe)
static MEMORY_MANAGER: Mutex<Option<Arc<Mutex<MemoryManager>>>> = Mutex::new(None);

/// Set the global memory manager.
///
/// This should be called once during runtime initialization.
pub fn set_memory_manager(manager: Arc<Mutex<MemoryManager>>>) {
    if let Ok(mut mgr) = MEMORY_MANAGER.lock() {
        *mgr = Some(manager);
    } else {
        log::error!("Failed to lock memory manager mutex");
    }
}

/// Get the global memory manager.
fn get_memory_manager() -> Option<Arc<Mutex<MemoryManager>>> {
    MEMORY_MANAGER.lock().ok()?.clone()
}

/// VI callback function type (set by runtime)
type VISetModeCallback = Box<dyn Fn(u32) + Send + Sync>;
type VISetBlackCallback = Box<dyn Fn(bool) + Send + Sync>;

static VI_SET_MODE_CB: Mutex<Option<VISetModeCallback>> = Mutex::new(None);
static VI_SET_BLACK_CB: Mutex<Option<VISetBlackCallback>> = Mutex::new(None);

/// Register VI callbacks (called by runtime)
pub fn register_vi_callbacks(set_mode: VISetModeCallback, set_black: VISetBlackCallback) {
    if let Ok(mut cb) = VI_SET_MODE_CB.lock() {
        *cb = Some(set_mode);
    }
    if let Ok(mut cb) = VI_SET_BLACK_CB.lock() {
        *cb = Some(set_black);
    }
}

/// GX callback function type (set by runtime)
type GXSetViewportCallback = Box<dyn Fn(f32, f32, f32, f32, f32, f32) + Send + Sync>;

static GX_SET_VIEWPORT_CB: Mutex<Option<GXSetViewportCallback>> = Mutex::new(None);

/// Register GX callbacks (called by runtime)
pub fn register_gx_callbacks(set_viewport: GXSetViewportCallback) {
    if let Ok(mut cb) = GX_SET_VIEWPORT_CB.lock() {
        *cb = Some(set_viewport);
    }
}

/// OSReport - Debug output function
pub fn os_report(message: &str) {
    info!("OSReport: {}", message);
}

/// Memory initialization
pub fn init_memory() {
    info!("Initializing memory...");
    // Memory manager should be set before calling this
    if get_memory_manager().is_none() {
        warn!("Memory manager not set, creating default instance");
        let manager = Arc::new(Mutex::new(MemoryManager::new()));
        set_memory_manager(manager);
    }
}

/// GX (Graphics) initialization
pub fn init_gx() {
    info!("Initializing GX graphics system...");
    // GX initialization is handled by runtime registration
}

/// VI (Video Interface) initialization
pub fn init_vi() {
    info!("Initializing VI video interface...");
    // VI initialization is handled by runtime registration
}

/// AI (Audio Interface) initialization
pub fn init_ai() {
    info!("Initializing AI audio interface...");
    // AI initialization is handled by runtime registration
}

/// DSP initialization
pub fn init_dsp() {
    info!("Initializing DSP...");
    // DSP initialization is handled by runtime registration
}

/// OSInit - Operating system initialization
pub fn os_init() {
    info!("OSInit called");
    init_memory();
}

/// OSFatal - Fatal error handler
pub fn os_fatal(message: &str) {
    warn!("OSFatal: {}", message);
    // In a real implementation, this would terminate the program
}

/// OSAllocFromArenaLo - Allocate memory from low arena
pub fn os_alloc_from_arena_lo(size: u32) -> *mut u8 {
    if let Some(manager) = get_memory_manager() {
        let mut mem = match manager.lock() {
            Ok(m) => m,
            Err(_) => {
                warn!("OSAllocFromArenaLo({}) - failed to lock memory manager", size);
                return std::ptr::null_mut();
            }
        };
        if let Some(addr) = mem.arena_mut().alloc_low(size) {
            // Convert address to pointer (unsafe but necessary for C compatibility)
            addr as *mut u8
        } else {
            warn!("OSAllocFromArenaLo({}) - out of memory", size);
            std::ptr::null_mut()
        }
    } else {
        warn!("OSAllocFromArenaLo({}) - memory manager not initialized", size);
        std::ptr::null_mut()
    }
}

/// OSAllocFromArenaHi - Allocate memory from high arena
pub fn os_alloc_from_arena_hi(size: u32) -> *mut u8 {
    if let Some(manager) = get_memory_manager() {
        let mut mem = match manager.lock() {
            Ok(m) => m,
            Err(_) => {
                warn!("OSAllocFromArenaLo({}) - failed to lock memory manager", size);
                return std::ptr::null_mut();
            }
        };
        if let Some(addr) = mem.arena_mut().alloc_high(size) {
            addr as *mut u8
        } else {
            warn!("OSAllocFromArenaHi({}) - out of memory", size);
            std::ptr::null_mut()
        }
    } else {
        warn!("OSAllocFromArenaHi({}) - memory manager not initialized", size);
        std::ptr::null_mut()
    }
}

/// OSFreeToArenaLo - Free memory to low arena
pub fn os_free_to_arena_lo(ptr: *mut u8, size: u32) {
    if ptr.is_null() {
        return;
    }
    if let Some(manager) = get_memory_manager() {
        if let Ok(mut mem) = manager.lock() {
            let addr = ptr as u32;
            mem.arena_mut().free_low(addr, size);
        }
    } else {
        warn!("OSFreeToArenaLo({:p}, {}) - memory manager not initialized", ptr, size);
    }
}

/// OSFreeToArenaHi - Free memory to high arena
pub fn os_free_to_arena_hi(ptr: *mut u8, size: u32) {
    if ptr.is_null() {
        return;
    }
    if let Some(manager) = get_memory_manager() {
        let mut mem = match manager.lock() {
            Ok(m) => m,
            Err(_) => {
                warn!("OSAllocFromArenaLo({}) - failed to lock memory manager", size);
                return std::ptr::null_mut();
            }
        };
        let addr = ptr as u32;
        mem.arena_mut().free_high(addr, size);
    } else {
        warn!("OSFreeToArenaHi({:p}, {}) - memory manager not initialized", ptr, size);
    }
}

// GX Graphics API stubs
pub fn gx_init() {
    info!("GX_Init called");
    init_gx();
}

pub fn gx_set_viewport(x: f32, y: f32, w: f32, h: f32, near: f32, far: f32) {
    if let Ok(cb_guard) = GX_SET_VIEWPORT_CB.lock() {
        if let Some(cb) = cb_guard.as_ref() {
            cb(x, y, w, h, near, far);
            return;
        }
    }
    log::info!(
        "GX_SetViewport({}, {}, {}, {}, {}, {}) - callback not registered",
        x, y, w, h, near, far
    );
}

pub fn gx_clear_color(r: u8, g: u8, b: u8, a: u8) {
    info!("GX_ClearColor({}, {}, {}, {})", r, g, b, a);
}

// VI Video Interface stubs
pub fn vi_set_mode(mode: u32) {
    if let Ok(cb_guard) = VI_SET_MODE_CB.lock() {
        if let Some(cb) = cb_guard.as_ref() {
            cb(mode);
            return;
        }
    }
    info!("VI_SetMode({}) - callback not registered", mode);
}

pub fn vi_set_black(black: bool) {
    if let Ok(cb_guard) = VI_SET_BLACK_CB.lock() {
        if let Some(cb) = cb_guard.as_ref() {
            cb(black);
            return;
        }
    }
    info!("VI_SetBlack({}) - callback not registered", black);
}

// AI Audio Interface stubs
pub fn ai_init() {
    info!("AI_Init called");
    init_ai();
}

pub fn ai_set_stream_sample_rate(rate: u32) {
    info!("AI_SetStreamSampleRate({})", rate);
    // AI initialization is handled by runtime
}

// DSP stubs
pub fn dsp_init() {
    info!("DSP_Init called");
    init_dsp();
}

/// Handle unimplemented instruction
///
/// This function is called from generated code when an unimplemented
/// instruction is encountered. It logs the instruction and optionally
/// provides fallback behavior.
pub fn handle_unimplemented_instruction(
    address: u32,
    raw_instruction: u32,
    _ctx: &mut crate::runtime::context::CpuContext,
    _memory: &mut MemoryManager,
) -> anyhow::Result<()> {
    warn!(
        "Unimplemented instruction at 0x{:08X}: 0x{:08X}",
        address, raw_instruction
    );
    // For now, just log and continue
    // In the future, this could provide emulation or fallback behavior
    Ok(())
}

