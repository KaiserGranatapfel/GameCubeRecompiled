/// GameCube video mode definitions.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VideoMode {
    pub fb_width: u16,
    pub efb_height: u16,
    pub xfb_height: u16,
    pub vi_x_origin: u16,
    pub vi_y_origin: u16,
    pub vi_width: u16,
    pub vi_height: u16,
    pub xfb_mode: XfbMode,
    pub field_rendering: bool,
    pub anti_aliasing: bool,
    pub timing: VideoTiming,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XfbMode {
    Single,
    Double,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoTiming {
    Ntsc,
    Pal,
    Mpal,
}

impl VideoMode {
    /// NTSC 480i (standard interlaced, used by most US/JP games).
    pub fn ntsc_480i() -> Self {
        Self {
            fb_width: 640,
            efb_height: 480,
            xfb_height: 480,
            vi_x_origin: 0,
            vi_y_origin: 0,
            vi_width: 640,
            vi_height: 480,
            xfb_mode: XfbMode::Double,
            field_rendering: true,
            anti_aliasing: false,
            timing: VideoTiming::Ntsc,
        }
    }

    /// NTSC 480p (progressive scan).
    pub fn ntsc_480p() -> Self {
        Self {
            fb_width: 640,
            efb_height: 480,
            xfb_height: 480,
            vi_x_origin: 0,
            vi_y_origin: 0,
            vi_width: 640,
            vi_height: 480,
            xfb_mode: XfbMode::Double,
            field_rendering: false,
            anti_aliasing: false,
            timing: VideoTiming::Ntsc,
        }
    }

    /// PAL 576i (standard PAL interlaced).
    pub fn pal_576i() -> Self {
        Self {
            fb_width: 640,
            efb_height: 576,
            xfb_height: 576,
            vi_x_origin: 0,
            vi_y_origin: 0,
            vi_width: 640,
            vi_height: 576,
            xfb_mode: XfbMode::Double,
            field_rendering: true,
            anti_aliasing: false,
            timing: VideoTiming::Pal,
        }
    }

    /// Target frame rate based on timing standard.
    pub fn target_fps(&self) -> f64 {
        match self.timing {
            VideoTiming::Ntsc | VideoTiming::Mpal => 59.94,
            VideoTiming::Pal => 50.0,
        }
    }

    /// Frame duration in nanoseconds.
    pub fn frame_duration_ns(&self) -> u64 {
        (1_000_000_000.0 / self.target_fps()) as u64
    }
}
