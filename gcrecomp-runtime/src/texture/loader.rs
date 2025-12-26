// Texture loading
use crate::texture::cache::TextureCache;
use crate::texture::formats::GameCubeTextureFormat;
use anyhow::Result;
use image::RgbaImage;

pub struct TextureLoader {
    cache: TextureCache,
}

impl TextureLoader {
    pub fn new() -> Self {
        Self {
            cache: TextureCache::new(),
        }
    }

    pub fn load_texture(
        &mut self,
        data: &[u8],
        format: GameCubeTextureFormat,
        width: u32,
        height: u32,
    ) -> Result<RgbaImage> {
        // Check cache first
        let cache_key = format!("{:?}_{}_{}", format, width, height);
        if let Some(cached) = self.cache.get(&cache_key) {
            return Ok(cached.clone());
        }

        // Decode texture
        let image = format.decode(data, width, height)?;

        // Cache it
        self.cache.insert(cache_key, image.clone());

        Ok(image)
    }

    pub fn load_texture_with_mipmaps(
        &mut self,
        data: &[u8],
        format: GameCubeTextureFormat,
        width: u32,
        height: u32,
        mip_count: u32,
    ) -> Result<Vec<RgbaImage>> {
        let mut mipmaps = Vec::new();
        let mut offset = 0;
        let mut current_width = width;
        let mut current_height = height;

        for _ in 0..mip_count {
            let size = (current_width * current_height * format.bytes_per_pixel()) as usize;
            if offset + size <= data.len() {
                let mip_data = &data[offset..offset + size];
                let mip = format.decode(mip_data, current_width, current_height)?;
                mipmaps.push(mip);
                offset += size;
                current_width = (current_width / 2).max(1);
                current_height = (current_height / 2).max(1);
            }
        }

        Ok(mipmaps)
    }
}

impl GameCubeTextureFormat {
    pub fn bytes_per_pixel(&self) -> u32 {
        match self {
            Self::CMPR => 0, // Compressed, variable
            Self::I4 => 1,
            Self::I8 => 1,
            Self::IA4 => 1,
            Self::IA8 => 2,
            Self::RGB565 => 2,
            Self::RGB5A3 => 2,
            Self::RGBA8 => 4,
        }
    }
}
