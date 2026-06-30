pub mod calling;
pub mod context;
pub mod memory;
pub mod sdk;

use std::sync::atomic::{AtomicBool, Ordering};

/// Wall-clock "stop" flag for recompiled code. The recompiled entry may spin on
/// hardware/SDK state we don't fully emulate; a watchdog thread sets this after a
/// deadline so a host call into recompiled code always returns in bounded time.
static STOP: AtomicBool = AtomicBool::new(false);

/// Checked (cheaply) by generated code at every function entry and inside loops.
/// Returns true once the deadline has passed, making all functions bail fast.
#[inline]
pub fn out_of_budget() -> bool {
    STOP.load(Ordering::Relaxed)
}

/// Arm the watchdog: allow recompiled code `secs` of wall-clock time, then stop it.
pub fn arm_watchdog(secs: u64) {
    STOP.store(false, Ordering::Relaxed);
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(secs));
        STOP.store(true, Ordering::Relaxed);
    });
}

// --- Optional function-call trace (for debugging where boot diverges) ---
use std::sync::atomic::AtomicU64;
static TRACE: AtomicBool = AtomicBool::new(false);
static TRACE_N: AtomicU64 = AtomicU64::new(0);

/// Enable the call trace (logs the first few thousand function entries).
pub fn enable_trace() {
    TRACE.store(true, Ordering::Relaxed);
}

/// Called at the top of every generated function. No-op unless tracing is on.
#[inline]
pub fn trace_call(addr: u32) {
    if !TRACE.load(Ordering::Relaxed) {
        return;
    }
    let n = TRACE_N.fetch_add(1, Ordering::Relaxed);
    if n < 5000 {
        log::info!("TRACE[{n}] enter 0x{addr:08X}");
    }
}
