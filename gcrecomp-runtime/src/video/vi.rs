/// Video Interface (VI) — manages video modes, frame buffers, and retrace callbacks.
use super::modes::VideoMode;
use super::vblank::VBlankTimer;
use log::info;

pub struct VideoInterface {
    current_mode: VideoMode,
    next_xfb_addr: u32,
    current_xfb_addr: u32,
    flush_pending: bool,
    black: bool,
    enabled: bool,
    pre_retrace_callback: Option<u32>,  // GC function address
    post_retrace_callback: Option<u32>, // GC function address
    vblank: VBlankTimer,
}

impl VideoInterface {
    pub fn new() -> Self {
        let mode = VideoMode::ntsc_480i();
        Self {
            vblank: VBlankTimer::new(mode.target_fps()),
            current_mode: mode,
            next_xfb_addr: 0,
            current_xfb_addr: 0,
            flush_pending: false,
            black: true,
            enabled: false,
            pre_retrace_callback: None,
            post_retrace_callback: None,
        }
    }

    /// VIInit
    pub fn init(&mut self) {
        info!("VIInit");
        self.enabled = true;
        self.black = true;
    }

    /// VIConfigure
    pub fn configure(&mut self, mode: VideoMode) {
        info!(
            "VIConfigure: {}x{} @ {:.1} fps",
            mode.fb_width,
            mode.efb_height,
            mode.target_fps()
        );
        self.current_mode = mode;
        self.vblank.set_target_fps(mode.target_fps());
    }

    /// VISetNextFrameBuffer
    pub fn set_next_frame_buffer(&mut self, addr: u32) {
        self.next_xfb_addr = addr;
    }

    /// VIFlush — commit settings (swap XFB on next retrace).
    pub fn flush(&mut self) {
        self.flush_pending = true;
    }

    /// VISetBlack
    pub fn set_black(&mut self, black: bool) {
        self.black = black;
    }

    /// VIWaitForRetrace — blocks until next vertical retrace.
    /// Returns the pre/post retrace callback addresses if set.
    pub fn wait_for_retrace(&mut self) -> (Option<u32>, Option<u32>) {
        let pre = self.pre_retrace_callback;

        self.vblank.wait_for_retrace();

        if self.flush_pending {
            self.current_xfb_addr = self.next_xfb_addr;
            self.flush_pending = false;
        }

        let post = self.post_retrace_callback;
        (pre, post)
    }

    /// VISetPreRetraceCallback
    pub fn set_pre_retrace_callback(&mut self, func: u32) -> Option<u32> {
        let old = self.pre_retrace_callback;
        self.pre_retrace_callback = Some(func);
        old
    }

    /// VISetPostRetraceCallback
    pub fn set_post_retrace_callback(&mut self, func: u32) -> Option<u32> {
        let old = self.post_retrace_callback;
        self.post_retrace_callback = Some(func);
        old
    }

    /// VIGetRetraceCount
    pub fn get_retrace_count(&self) -> u32 {
        self.vblank.retrace_count()
    }

    pub fn current_mode(&self) -> &VideoMode {
        &self.current_mode
    }

    pub fn current_xfb_addr(&self) -> u32 {
        self.current_xfb_addr
    }

    pub fn is_black(&self) -> bool {
        self.black
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl Default for VideoInterface {
    fn default() -> Self {
        Self::new()
    }
}
