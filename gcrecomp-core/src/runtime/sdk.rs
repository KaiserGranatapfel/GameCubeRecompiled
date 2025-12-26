// GameCube SDK stubs
use log::{info, warn};

/// OSReport - Debug output function
pub fn os_report(message: &str) {
    info!("OSReport: {}", message);
}

/// Memory initialization
pub fn init_memory() {
    info!("Initializing memory...");
    // TODO: Implement memory initialization
}

/// GX (Graphics) initialization
pub fn init_gx() {
    info!("Initializing GX graphics system...");
    // TODO: Implement GX initialization
}

/// VI (Video Interface) initialization
pub fn init_vi() {
    info!("Initializing VI video interface...");
    // TODO: Implement VI initialization
}

/// AI (Audio Interface) initialization
pub fn init_ai() {
    info!("Initializing AI audio interface...");
    // TODO: Implement AI initialization
}

/// DSP initialization
pub fn init_dsp() {
    info!("Initializing DSP...");
    // TODO: Implement DSP initialization
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
    warn!("OSAllocFromArenaLo({}) - not implemented", size);
    std::ptr::null_mut()
}

/// OSAllocFromArenaHi - Allocate memory from high arena
pub fn os_alloc_from_arena_hi(size: u32) -> *mut u8 {
    warn!("OSAllocFromArenaHi({}) - not implemented", size);
    std::ptr::null_mut()
}

/// OSFreeToArenaLo - Free memory to low arena
pub fn os_free_to_arena_lo(ptr: *mut u8, size: u32) {
    warn!("OSFreeToArenaLo({:p}, {}) - not implemented", ptr, size);
}

/// OSFreeToArenaHi - Free memory to high arena
pub fn os_free_to_arena_hi(ptr: *mut u8, size: u32) {
    warn!("OSFreeToArenaHi({:p}, {}) - not implemented", ptr, size);
}

// GX Graphics API stubs
pub fn gx_init() {
    info!("GX_Init called");
}

pub fn gx_set_viewport(x: f32, y: f32, w: f32, h: f32, near: f32, far: f32) {
    info!("GX_SetViewport({}, {}, {}, {}, {}, {})", x, y, w, h, near, far);
}

pub fn gx_clear_color(r: u8, g: u8, b: u8, a: u8) {
    info!("GX_ClearColor({}, {}, {}, {})", r, g, b, a);
}

// VI Video Interface stubs
pub fn vi_set_mode(mode: u32) {
    info!("VI_SetMode({})", mode);
}

pub fn vi_set_black(black: bool) {
    info!("VI_SetBlack({})", black);
}

// AI Audio Interface stubs
pub fn ai_init() {
    info!("AI_Init called");
}

pub fn ai_set_stream_sample_rate(rate: u32) {
    info!("AI_SetStreamSampleRate({})", rate);
}

// DSP stubs
pub fn dsp_init() {
    info!("DSP_Init called");
}

