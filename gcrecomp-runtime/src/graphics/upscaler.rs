// Resolution upscaling
use anyhow::Result;
use wgpu::*;

pub struct Upscaler {
    upscale_factor: f32,
    maintain_aspect: bool,
}

impl Upscaler {
    pub fn new(_device: &Device, _config: &SurfaceConfiguration) -> Result<Self> {
        Ok(Self {
            upscale_factor: 1.0,
            maintain_aspect: true,
        })
    }
    
    pub fn set_factor(&mut self, factor: f32) {
        self.upscale_factor = factor;
    }
    
    pub fn set_maintain_aspect(&mut self, maintain: bool) {
        self.maintain_aspect = maintain;
    }
    
    pub fn calculate_target_resolution(&self, native: (u32, u32)) -> (u32, u32) {
        if self.maintain_aspect {
            let aspect = native.0 as f32 / native.1 as f32;
            let target_w = (native.0 as f32 * self.upscale_factor) as u32;
            let target_h = (target_w as f32 / aspect) as u32;
            (target_w, target_h)
        } else {
            (
                (native.0 as f32 * self.upscale_factor) as u32,
                (native.1 as f32 * self.upscale_factor) as u32,
            )
        }
    }
    
    pub fn integer_upscale(&self, native: (u32, u32), factor: u32) -> (u32, u32) {
        (native.0 * factor, native.1 * factor)
    }
    
    pub fn fractional_upscale(&self, native: (u32, u32), factor: f32) -> (u32, u32) {
        (
            (native.0 as f32 * factor) as u32,
            (native.1 as f32 * factor) as u32,
        )
    }
}

