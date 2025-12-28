//! VI (Video Interface) implementation
//!
//! The VI handles display output and video timing for the GameCube.
//! It manages display modes (NTSC/PAL), resolution, and frame timing.

use anyhow::Result;

/// VI display mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VIMode {
    /// NTSC mode (480i, 60Hz)
    NTSC480i,
    /// NTSC progressive (480p, 60Hz)
    NTSC480p,
    /// PAL mode (576i, 50Hz)
    PAL576i,
    /// PAL progressive (576p, 50Hz)
    PAL576p,
    /// Progressive scan (240p)
    Progressive240p,
    /// Unknown/unsupported mode
    Unknown(u32),
}

impl VIMode {
    /// Get resolution for this mode
    pub fn resolution(&self) -> (u32, u32) {
        match self {
            Self::NTSC480i | Self::NTSC480p => (640, 480),
            Self::PAL576i | Self::PAL576p => (640, 576),
            Self::Progressive240p => (640, 240),
            Self::Unknown(_) => (640, 480), // Default
        }
    }

    /// Get refresh rate (Hz)
    pub fn refresh_rate(&self) -> f32 {
        match self {
            Self::NTSC480i | Self::NTSC480p => 60.0,
            Self::PAL576i | Self::PAL576p => 50.0,
            Self::Progressive240p => 60.0,
            Self::Unknown(_) => 60.0,
        }
    }
}

/// VI state
#[derive(Debug)]
pub struct VI {
    /// Current display mode
    mode: VIMode,
    /// Black screen flag
    black: bool,
    /// Horizontal resolution
    width: u32,
    /// Vertical resolution
    height: u32,
    /// Frame buffer address
    framebuffer_addr: u32,
}

impl VI {
    /// Create a new VI instance
    pub fn new() -> Self {
        Self {
            mode: VIMode::NTSC480i,
            black: false,
            width: 640,
            height: 480,
            framebuffer_addr: 0,
        }
    }

    /// Set display mode
    pub fn set_mode(&mut self, mode_value: u32) -> Result<()> {
        self.mode = match mode_value {
            0 => VIMode::NTSC480i,
            1 => VIMode::NTSC480p,
            2 => VIMode::PAL576i,
            3 => VIMode::PAL576p,
            4 => VIMode::Progressive240p,
            _ => VIMode::Unknown(mode_value),
        };

        let (w, h) = self.mode.resolution();
        self.width = w;
        self.height = h;

        log::info!("VI mode set to {:?} ({}x{})", self.mode, w, h);
        Ok(())
    }

    /// Set black screen
    pub fn set_black(&mut self, black: bool) {
        self.black = black;
        if black {
            log::debug!("VI: Screen blacked out");
        } else {
            log::debug!("VI: Screen enabled");
        }
    }

    /// Get current mode
    pub fn mode(&self) -> VIMode {
        self.mode
    }

    /// Get resolution
    pub fn resolution(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Check if screen is blacked out
    pub fn is_black(&self) -> bool {
        self.black
    }

    /// Set frame buffer address
    pub fn set_framebuffer(&mut self, addr: u32) {
        self.framebuffer_addr = addr;
        log::debug!("VI framebuffer set to 0x{:08X}", addr);
    }

    /// Get frame buffer address
    pub fn framebuffer_addr(&self) -> u32 {
        self.framebuffer_addr
    }
}

impl Default for VI {
    fn default() -> Self {
        Self::new()
    }
}

