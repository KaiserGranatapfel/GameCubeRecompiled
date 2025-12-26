// Texture upscaling
use anyhow::Result;
use image::{imageops, RgbaImage};

pub struct TextureUpscaler {
    algorithm: UpscaleAlgorithm,
}

#[derive(Debug, Clone, Copy)]
pub enum UpscaleAlgorithm {
    Nearest,
    Linear,
    Bicubic,
    Lanczos3,
}

impl TextureUpscaler {
    pub fn new() -> Self {
        Self {
            algorithm: UpscaleAlgorithm::Lanczos3,
        }
    }

    pub fn set_algorithm(&mut self, algorithm: UpscaleAlgorithm) {
        self.algorithm = algorithm;
    }

    pub fn upscale(&self, image: &RgbaImage, factor: f32) -> Result<RgbaImage> {
        let new_width = (image.width() as f32 * factor) as u32;
        let new_height = (image.height() as f32 * factor) as u32;

        let upscaled = match self.algorithm {
            UpscaleAlgorithm::Nearest => {
                imageops::resize(image, new_width, new_height, imageops::FilterType::Nearest)
            }
            UpscaleAlgorithm::Linear => {
                imageops::resize(image, new_width, new_height, imageops::FilterType::Triangle)
            }
            UpscaleAlgorithm::Bicubic => imageops::resize(
                image,
                new_width,
                new_height,
                imageops::FilterType::CatmullRom,
            ),
            UpscaleAlgorithm::Lanczos3 => {
                imageops::resize(image, new_width, new_height, imageops::FilterType::Lanczos3)
            }
        };

        Ok(upscaled)
    }
}
