use std::time::Instant;

/// GameCube timer emulation.
///
/// The GameCube timebase runs at 1/4 of the bus clock:
/// - Bus clock: 162 MHz
/// - Timebase: 40.5 MHz (162 / 4)
///
/// `OSGetTick()` returns the lower 32 bits of the timebase counter.
/// `OSGetTime()` returns the full 64-bit timebase counter.
pub struct OsTimer {
    start: Instant,
}

impl OsTimer {
    /// Timebase frequency: 40.5 MHz (bus clock / 4).
    const TIMEBASE_FREQ: u64 = 40_500_000;
    /// Bus clock frequency: 162 MHz.
    pub const BUS_CLOCK: u64 = 162_000_000;

    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    pub fn reset(&mut self) {
        self.start = Instant::now();
    }

    /// Get the lower 32 bits of the timebase counter (OSGetTick).
    pub fn get_tick(&self) -> u32 {
        self.get_time() as u32
    }

    /// Get the full 64-bit timebase counter (OSGetTime).
    pub fn get_time(&self) -> u64 {
        let elapsed = self.start.elapsed();
        let nanos = elapsed.as_nanos() as u64;
        // Convert nanoseconds to timebase ticks: ticks = nanos * freq / 1_000_000_000
        nanos.wrapping_mul(Self::TIMEBASE_FREQ) / 1_000_000_000
    }

    /// Compute tick difference (handles 32-bit wrap).
    pub fn diff_tick(tick1: u32, tick0: u32) -> u32 {
        tick1.wrapping_sub(tick0)
    }

    /// Convert ticks to milliseconds.
    pub fn ticks_to_millis(ticks: u64) -> u64 {
        ticks * 1000 / Self::TIMEBASE_FREQ
    }

    /// Convert milliseconds to ticks.
    pub fn millis_to_ticks(ms: u64) -> u64 {
        ms * Self::TIMEBASE_FREQ / 1000
    }
}

impl Default for OsTimer {
    fn default() -> Self {
        Self::new()
    }
}
