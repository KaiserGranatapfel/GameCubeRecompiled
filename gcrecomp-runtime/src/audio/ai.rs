/// Audio Interface (AI) — manages sample rate, DMA, and streaming.
use log::info;

pub struct AudioInterface {
    sample_rate: u32,
    dma_address: u32,
    dma_length: u32,
    dma_active: bool,
    _streaming: bool,
    volume_left: u8,
    volume_right: u8,
    dma_callback: Option<u32>, // GC function address for AI DMA interrupt
    initialized: bool,
}

impl AudioInterface {
    pub fn new() -> Self {
        Self {
            sample_rate: 32000,
            dma_address: 0,
            dma_length: 0,
            dma_active: false,
            _streaming: false,
            volume_left: 255,
            volume_right: 255,
            dma_callback: None,
            initialized: false,
        }
    }

    /// AIInit
    pub fn init(&mut self) {
        info!("AIInit: sample_rate={}", self.sample_rate);
        self.initialized = true;
        self.volume_left = 255;
        self.volume_right = 255;
    }

    /// AIInitDMA
    pub fn init_dma(&mut self, address: u32, length: u32) {
        self.dma_address = address;
        self.dma_length = length;
        info!("AIInitDMA: addr=0x{:08X} len={}", address, length);
    }

    /// AIStartDMA
    pub fn start_dma(&mut self) {
        self.dma_active = true;
        info!("AIStartDMA");
    }

    /// AIStopDMA
    pub fn stop_dma(&mut self) {
        self.dma_active = false;
        info!("AIStopDMA");
    }

    /// AISetStreamSampleRate
    pub fn set_stream_sample_rate(&mut self, rate: u32) {
        self.sample_rate = if rate == 0 { 32000 } else { 48000 };
        info!("AISetStreamSampleRate: {}", self.sample_rate);
    }

    /// AIRegisterDMACallback
    pub fn register_dma_callback(&mut self, callback: u32) -> Option<u32> {
        let old = self.dma_callback;
        self.dma_callback = Some(callback);
        old
    }

    /// AISetStreamVolLeft
    pub fn set_volume_left(&mut self, vol: u8) {
        self.volume_left = vol;
    }

    /// AISetStreamVolRight
    pub fn set_volume_right(&mut self, vol: u8) {
        self.volume_right = vol;
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn dma_address(&self) -> u32 {
        self.dma_address
    }

    pub fn dma_length(&self) -> u32 {
        self.dma_length
    }

    pub fn is_dma_active(&self) -> bool {
        self.dma_active
    }

    pub fn dma_callback(&self) -> Option<u32> {
        self.dma_callback
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

impl Default for AudioInterface {
    fn default() -> Self {
        Self::new()
    }
}
