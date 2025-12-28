// GameCube texture format support
use anyhow::Result;
use image::{DynamicImage, RgbaImage};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameCubeTextureFormat {
    CMPR,   // Compressed (S3TC/DXT1)
    I4,     // 4-bit intensity
    I8,     // 8-bit intensity
    IA4,    // 4-bit intensity + alpha
    IA8,    // 8-bit intensity + alpha
    RGB565, // 16-bit RGB
    RGB5A3, // 16-bit RGB + alpha
    RGBA8,  // 32-bit RGBA
}

impl GameCubeTextureFormat {
    pub fn from_gx_format(format: u8) -> Option<Self> {
        match format {
            0x00 => Some(Self::I4),
            0x01 => Some(Self::I8),
            0x02 => Some(Self::IA4),
            0x03 => Some(Self::IA8),
            0x04 => Some(Self::RGB565),
            0x05 => Some(Self::RGB5A3),
            0x06 => Some(Self::RGBA8),
            0x08 => Some(Self::CMPR),
            _ => None,
        }
    }

    pub fn decode(&self, data: &[u8], width: u32, height: u32) -> Result<RgbaImage> {
        match self {
            Self::CMPR => Self::decode_cmpr(data, width, height),
            Self::I4 => Self::decode_i4(data, width, height),
            Self::I8 => Self::decode_i8(data, width, height),
            Self::IA4 => Self::decode_ia4(data, width, height),
            Self::IA8 => Self::decode_ia8(data, width, height),
            Self::RGB565 => Self::decode_rgb565(data, width, height),
            Self::RGB5A3 => Self::decode_rgb5a3(data, width, height),
            Self::RGBA8 => Self::decode_rgba8(data, width, height),
        }
    }

    fn decode_cmpr(data: &[u8], width: u32, height: u32) -> Result<RgbaImage> {
        // CMPR is DXT1/S3TC compression (4x4 blocks, 8 bytes per block)
        let mut image = RgbaImage::new(width, height);
        
        let block_width = (width + 3) / 4;
        let block_height = (height + 3) / 4;
        
        for by in 0..block_height {
            for bx in 0..block_width {
                let block_offset = ((by * block_width + bx) * 8) as usize;
                if block_offset + 8 > data.len() {
                    continue;
                }
                
                // Read color endpoints (RGB565)
                let c0 = u16::from_le_bytes([data[block_offset], data[block_offset + 1]]);
                let c1 = u16::from_le_bytes([data[block_offset + 2], data[block_offset + 3]]);
                
                // Decode RGB565 to RGB888
                let r0 = ((c0 >> 11) & 0x1F) as u8;
                let g0 = ((c0 >> 5) & 0x3F) as u8;
                let b0 = (c0 & 0x1F) as u8;
                let rgb0 = [
                    (r0 << 3) | (r0 >> 2),
                    (g0 << 2) | (g0 >> 4),
                    (b0 << 3) | (b0 >> 2),
                ];
                
                let r1 = ((c1 >> 11) & 0x1F) as u8;
                let g1 = ((c1 >> 5) & 0x3F) as u8;
                let b1 = (c1 & 0x1F) as u8;
                let rgb1 = [
                    (r1 << 3) | (r1 >> 2),
                    (g1 << 2) | (g1 >> 4),
                    (b1 << 3) | (b1 >> 2),
                ];
                
                // Interpolate colors
                let rgb2 = [
                    ((rgb0[0] as u16 * 2 + rgb1[0] as u16) / 3) as u8,
                    ((rgb0[1] as u16 * 2 + rgb1[1] as u16) / 3) as u8,
                    ((rgb0[2] as u16 * 2 + rgb1[2] as u16) / 3) as u8,
                ];
                let rgb3 = [
                    ((rgb0[0] as u16 + rgb1[0] as u16 * 2) / 3) as u8,
                    ((rgb0[1] as u16 + rgb1[1] as u16 * 2) / 3) as u8,
                    ((rgb0[2] as u16 + rgb1[2] as u16 * 2) / 3) as u8,
                ];
                
                // Read pixel indices (4x4 = 16 pixels, 2 bits each = 4 bytes)
                let indices = &data[block_offset + 4..block_offset + 8];
                
                // Decode 4x4 block
                for py in 0..4 {
                    for px in 0..4 {
                        let x = bx * 4 + px;
                        let y = by * 4 + py;
                        
                        if x >= width || y >= height {
                            continue;
                        }
                        
                        let index_byte = indices[py as usize];
                        let index = ((index_byte >> (px * 2)) & 0x3) as usize;
                        
                        let color = match index {
                            0 => rgb0,
                            1 => rgb1,
                            2 => rgb2,
                            3 => rgb3,
                            _ => [0, 0, 0],
                        };
                        
                        image.put_pixel(x, y, image::Rgba([color[0], color[1], color[2], 255]));
                    }
                }
            }
        }
        
        Ok(image)
    }

    fn decode_i4(data: &[u8], width: u32, height: u32) -> Result<RgbaImage> {
        let mut image = RgbaImage::new(width, height);
        let pixels_per_byte = 2;
        let mut data_idx = 0;

        for y in 0..height {
            for x in 0..width {
                let byte_idx = (y * width + x) / pixels_per_byte;
                if byte_idx < data.len() {
                    let byte = data[byte_idx as usize];
                    let pixel_idx = (x % pixels_per_byte) as usize;
                    let intensity = if pixel_idx == 0 {
                        ((byte >> 4) & 0xF) * 17
                    } else {
                        (byte & 0xF) * 17
                    };

                    image.put_pixel(x, y, image::Rgba([intensity, intensity, intensity, 255]));
                }
                data_idx += 1;
            }
        }

        Ok(image)
    }

    fn decode_i8(data: &[u8], width: u32, height: u32) -> Result<RgbaImage> {
        let mut image = RgbaImage::new(width, height);
        let mut data_idx = 0;

        for y in 0..height {
            for x in 0..width {
                if data_idx < data.len() {
                    let intensity = data[data_idx];
                    image.put_pixel(x, y, image::Rgba([intensity, intensity, intensity, 255]));
                    data_idx += 1;
                }
            }
        }

        Ok(image)
    }

    fn decode_ia4(data: &[u8], width: u32, height: u32) -> Result<RgbaImage> {
        let mut image = RgbaImage::new(width, height);
        let pixels_per_byte = 2;

        for y in 0..height {
            for x in 0..width {
                let byte_idx = ((y * width + x) / pixels_per_byte) as usize;
                if byte_idx < data.len() {
                    let byte = data[byte_idx];
                    let pixel_idx = (x % pixels_per_byte) as usize;
                    let (intensity, alpha) = if pixel_idx == 0 {
                        (((byte >> 4) & 0xF) * 17, ((byte >> 7) & 0x1) * 255)
                    } else {
                        ((byte & 0xF) * 17, ((byte >> 3) & 0x1) * 255)
                    };

                    image.put_pixel(x, y, image::Rgba([intensity, intensity, intensity, alpha]));
                }
            }
        }

        Ok(image)
    }

    fn decode_ia8(data: &[u8], width: u32, height: u32) -> Result<RgbaImage> {
        let mut image = RgbaImage::new(width, height);
        let mut data_idx = 0;

        for y in 0..height {
            for x in 0..width {
                if data_idx + 1 < data.len() {
                    let intensity = data[data_idx];
                    let alpha = data[data_idx + 1];
                    image.put_pixel(x, y, image::Rgba([intensity, intensity, intensity, alpha]));
                    data_idx += 2;
                }
            }
        }

        Ok(image)
    }

    fn decode_rgb565(data: &[u8], width: u32, height: u32) -> Result<RgbaImage> {
        let mut image = RgbaImage::new(width, height);
        let mut data_idx = 0;

        for y in 0..height {
            for x in 0..width {
                if data_idx + 1 < data.len() {
                    let word = u16::from_be_bytes([data[data_idx], data[data_idx + 1]]);
                    let r = ((word >> 11) & 0x1F) as u8 * 8;
                    let g = ((word >> 5) & 0x3F) as u8 * 4;
                    let b = (word & 0x1F) as u8 * 8;
                    image.put_pixel(x, y, image::Rgba([r, g, b, 255]));
                    data_idx += 2;
                }
            }
        }

        Ok(image)
    }

    fn decode_rgb5a3(data: &[u8], width: u32, height: u32) -> Result<RgbaImage> {
        let mut image = RgbaImage::new(width, height);
        let mut data_idx = 0;

        for y in 0..height {
            for x in 0..width {
                if data_idx + 1 < data.len() {
                    let word = u16::from_be_bytes([data[data_idx], data[data_idx + 1]]);
                    if (word & 0x8000) != 0 {
                        // RGB5 mode
                        let r = ((word >> 10) & 0x1F) as u8 * 8;
                        let g = ((word >> 5) & 0x1F) as u8 * 8;
                        let b = (word & 0x1F) as u8 * 8;
                        image.put_pixel(x, y, image::Rgba([r, g, b, 255]));
                    } else {
                        // RGB4A3 mode
                        let a = ((word >> 12) & 0x7) as u8 * 32;
                        let r = ((word >> 8) & 0xF) as u8 * 16;
                        let g = ((word >> 4) & 0xF) as u8 * 16;
                        let b = (word & 0xF) as u8 * 16;
                        image.put_pixel(x, y, image::Rgba([r, g, b, a]));
                    }
                    data_idx += 2;
                }
            }
        }

        Ok(image)
    }

    fn decode_rgba8(data: &[u8], width: u32, height: u32) -> Result<RgbaImage> {
        let mut image = RgbaImage::new(width, height);
        let mut data_idx = 0;

        for y in 0..height {
            for x in 0..width {
                if data_idx + 3 < data.len() {
                    let r = data[data_idx];
                    let g = data[data_idx + 1];
                    let b = data[data_idx + 2];
                    let a = data[data_idx + 3];
                    image.put_pixel(x, y, image::Rgba([r, g, b, a]));
                    data_idx += 4;
                }
            }
        }

        Ok(image)
    }
}
