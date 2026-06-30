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
