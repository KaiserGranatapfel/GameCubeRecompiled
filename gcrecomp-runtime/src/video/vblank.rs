/// VBlank timing: tracks frame timing and fires retrace callbacks.
use std::time::Instant;

pub struct VBlankTimer {
    last_retrace: Instant,
    retrace_count: u32,
    target_frame_ns: u64,
}

impl VBlankTimer {
    pub fn new(target_fps: f64) -> Self {
        Self {
            last_retrace: Instant::now(),
            retrace_count: 0,
            target_frame_ns: (1_000_000_000.0 / target_fps) as u64,
        }
    }

    /// Set target frame rate.
    pub fn set_target_fps(&mut self, fps: f64) {
        self.target_frame_ns = (1_000_000_000.0 / fps) as u64;
    }

    /// Wait until the next retrace period. Returns true if a retrace occurred.
    pub fn wait_for_retrace(&mut self) -> bool {
        let elapsed = self.last_retrace.elapsed().as_nanos() as u64;
        if elapsed < self.target_frame_ns {
            let sleep_ns = self.target_frame_ns - elapsed;
            // Sleep in smaller increments for better precision
            if sleep_ns > 1_000_000 {
                std::thread::sleep(std::time::Duration::from_nanos(sleep_ns - 500_000));
            }
            // Spin-wait for the remaining time
            while (self.last_retrace.elapsed().as_nanos() as u64) < self.target_frame_ns {
                std::hint::spin_loop();
            }
        }
        self.last_retrace = Instant::now();
        self.retrace_count = self.retrace_count.wrapping_add(1);
        true
    }

    /// Check if a retrace period has passed without blocking.
    pub fn check_retrace(&mut self) -> bool {
        let elapsed = self.last_retrace.elapsed().as_nanos() as u64;
        if elapsed >= self.target_frame_ns {
            self.last_retrace = Instant::now();
            self.retrace_count = self.retrace_count.wrapping_add(1);
            true
        } else {
            false
        }
    }

    pub fn retrace_count(&self) -> u32 {
        self.retrace_count
    }
}
