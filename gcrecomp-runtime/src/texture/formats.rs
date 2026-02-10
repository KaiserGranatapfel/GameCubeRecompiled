// GameCube texture format support
use anyhow::Result;
use image::RgbaImage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameCubeTextureFormat {
    Cmpr,   // Compressed (S3TC/DXT1)
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
            0x08 => Some(Self::Cmpr),
            _ => None,
        }
    }

    pub fn decode(&self, data: &[u8], width: u32, height: u32) -> Result<RgbaImage> {
        match self {
            Self::Cmpr => Self::decode_cmpr(data, width, height),
            Self::I4 => Self::decode_i4(data, width, height),
            Self::I8 => Self::decode_i8(data, width, height),
            Self::IA4 => Self::decode_ia4(data, width, height),
            Self::IA8 => Self::decode_ia8(data, width, height),
            Self::RGB565 => Self::decode_rgb565(data, width, height),
            Self::RGB5A3 => Self::decode_rgb5a3(data, width, height),
            Self::RGBA8 => Self::decode_rgba8(data, width, height),
        }
    }

    // -- Cmpr (DXT1 / S3TC) with GameCube 8x8 macro-tile layout ---------

    fn decode_cmpr(data: &[u8], width: u32, height: u32) -> Result<RgbaImage> {
        let mut image = RgbaImage::new(width, height);
        let tile_w: u32 = 8;
        let tile_h: u32 = 8;
        let tiles_x = width.div_ceil(tile_w);
        let tiles_y = height.div_ceil(tile_h);
        let mut offset = 0usize;

        for ty in 0..tiles_y {
            for tx in 0..tiles_x {
                // Each 8x8 macro-tile contains 4 DXT1 sub-blocks (4x4 each)
                // arranged in Z-order: top-left, top-right, bottom-left, bottom-right
                for sub in 0..4u32 {
                    let sub_x = (sub % 2) * 4;
                    let sub_y = (sub / 2) * 4;

                    if offset + 8 > data.len() {
                        break;
                    }

                    let block = &data[offset..offset + 8];
                    offset += 8;

                    let c0 = u16::from_be_bytes([block[0], block[1]]);
                    let c1 = u16::from_be_bytes([block[2], block[3]]);

                    let palette = Self::dxt1_palette(c0, c1);

                    for row in 0..4u32 {
                        let bits = block[4 + row as usize];
                        for col in 0..4u32 {
                            let idx = ((bits >> (6 - col * 2)) & 0x03) as usize;
                            let px = tx * tile_w + sub_x + col;
                            let py = ty * tile_h + sub_y + row;
                            if px < width && py < height {
                                image.put_pixel(px, py, image::Rgba(palette[idx]));
                            }
                        }
                    }
                }
            }
        }

        Ok(image)
    }

    /// Build the 4-color DXT1 palette from two 16-bit RGB565 endpoints.
    fn dxt1_palette(c0: u16, c1: u16) -> [[u8; 4]; 4] {
        let r0 = Self::expand5(((c0 >> 11) & 0x1F) as u8);
        let g0 = Self::expand6(((c0 >> 5) & 0x3F) as u8);
        let b0 = Self::expand5((c0 & 0x1F) as u8);
        let r1 = Self::expand5(((c1 >> 11) & 0x1F) as u8);
        let g1 = Self::expand6(((c1 >> 5) & 0x3F) as u8);
        let b1 = Self::expand5((c1 & 0x1F) as u8);

        if c0 > c1 {
            [
                [r0, g0, b0, 255],
                [r1, g1, b1, 255],
                [
                    ((2 * r0 as u16 + r1 as u16) / 3) as u8,
                    ((2 * g0 as u16 + g1 as u16) / 3) as u8,
                    ((2 * b0 as u16 + b1 as u16) / 3) as u8,
                    255,
                ],
                [
                    ((r0 as u16 + 2 * r1 as u16) / 3) as u8,
                    ((g0 as u16 + 2 * g1 as u16) / 3) as u8,
                    ((b0 as u16 + 2 * b1 as u16) / 3) as u8,
                    255,
                ],
            ]
        } else {
            [
                [r0, g0, b0, 255],
                [r1, g1, b1, 255],
                [
                    ((r0 as u16 + r1 as u16) / 2) as u8,
                    ((g0 as u16 + g1 as u16) / 2) as u8,
                    ((b0 as u16 + b1 as u16) / 2) as u8,
                    255,
                ],
                [0, 0, 0, 0], // transparent black
            ]
        }
    }

    fn expand5(v: u8) -> u8 {
        (v << 3) | (v >> 2)
    }

    fn expand6(v: u8) -> u8 {
        (v << 2) | (v >> 4)
    }

    // -- Tile-based decoders (I4 8x8) -----------------------------------

    fn decode_i4(data: &[u8], width: u32, height: u32) -> Result<RgbaImage> {
        let mut image = RgbaImage::new(width, height);
        let tile_w: u32 = 8;
        let tile_h: u32 = 8;
        let tiles_x = width.div_ceil(tile_w);
        let tiles_y = height.div_ceil(tile_h);
        let mut offset = 0usize;

        for ty in 0..tiles_y {
            for tx in 0..tiles_x {
                for row in 0..tile_h {
                    for col in (0..tile_w).step_by(2) {
                        if offset >= data.len() {
                            break;
                        }
                        let byte = data[offset];
                        offset += 1;

                        let px = tx * tile_w + col;
                        let py = ty * tile_h + row;

                        let hi = ((byte >> 4) & 0xF) * 17;
                        let lo = (byte & 0xF) * 17;

                        if px < width && py < height {
                            image.put_pixel(px, py, image::Rgba([hi, hi, hi, 255]));
                        }
                        if px + 1 < width && py < height {
                            image.put_pixel(px + 1, py, image::Rgba([lo, lo, lo, 255]));
                        }
                    }
                }
            }
        }

        Ok(image)
    }

    // -- I8 (8x4 tiles) -------------------------------------------------

    fn decode_i8(data: &[u8], width: u32, height: u32) -> Result<RgbaImage> {
        let mut image = RgbaImage::new(width, height);
        let tile_w: u32 = 8;
        let tile_h: u32 = 4;
        let tiles_x = width.div_ceil(tile_w);
        let tiles_y = height.div_ceil(tile_h);
        let mut offset = 0usize;

        for ty in 0..tiles_y {
            for tx in 0..tiles_x {
                for row in 0..tile_h {
                    for col in 0..tile_w {
                        if offset >= data.len() {
                            break;
                        }
                        let intensity = data[offset];
                        offset += 1;

                        let px = tx * tile_w + col;
                        let py = ty * tile_h + row;
                        if px < width && py < height {
                            image.put_pixel(
                                px,
                                py,
                                image::Rgba([intensity, intensity, intensity, 255]),
                            );
                        }
                    }
                }
            }
        }

        Ok(image)
    }

    // -- IA4 (8x4 tiles) ------------------------------------------------

    fn decode_ia4(data: &[u8], width: u32, height: u32) -> Result<RgbaImage> {
        let mut image = RgbaImage::new(width, height);
        let tile_w: u32 = 8;
        let tile_h: u32 = 4;
        let tiles_x = width.div_ceil(tile_w);
        let tiles_y = height.div_ceil(tile_h);
        let mut offset = 0usize;

        for ty in 0..tiles_y {
            for tx in 0..tiles_x {
                for row in 0..tile_h {
                    for col in 0..tile_w {
                        if offset >= data.len() {
                            break;
                        }
                        let byte = data[offset];
                        offset += 1;

                        let alpha = ((byte >> 4) & 0xF) * 17;
                        let intensity = (byte & 0xF) * 17;

                        let px = tx * tile_w + col;
                        let py = ty * tile_h + row;
                        if px < width && py < height {
                            image.put_pixel(
                                px,
                                py,
                                image::Rgba([intensity, intensity, intensity, alpha]),
                            );
                        }
                    }
                }
            }
        }

        Ok(image)
    }

    // -- IA8 (4x4 tiles) ------------------------------------------------

    fn decode_ia8(data: &[u8], width: u32, height: u32) -> Result<RgbaImage> {
        let mut image = RgbaImage::new(width, height);
        let tile_w: u32 = 4;
        let tile_h: u32 = 4;
        let tiles_x = width.div_ceil(tile_w);
        let tiles_y = height.div_ceil(tile_h);
        let mut offset = 0usize;

        for ty in 0..tiles_y {
            for tx in 0..tiles_x {
                for row in 0..tile_h {
                    for col in 0..tile_w {
                        if offset + 1 >= data.len() {
                            break;
                        }
                        let alpha = data[offset];
                        let intensity = data[offset + 1];
                        offset += 2;

                        let px = tx * tile_w + col;
                        let py = ty * tile_h + row;
                        if px < width && py < height {
                            image.put_pixel(
                                px,
                                py,
                                image::Rgba([intensity, intensity, intensity, alpha]),
                            );
                        }
                    }
                }
            }
        }

        Ok(image)
    }

    // -- RGB565 (4x4 tiles) ---------------------------------------------

    fn decode_rgb565(data: &[u8], width: u32, height: u32) -> Result<RgbaImage> {
        let mut image = RgbaImage::new(width, height);
        let tile_w: u32 = 4;
        let tile_h: u32 = 4;
        let tiles_x = width.div_ceil(tile_w);
        let tiles_y = height.div_ceil(tile_h);
        let mut offset = 0usize;

        for ty in 0..tiles_y {
            for tx in 0..tiles_x {
                for row in 0..tile_h {
                    for col in 0..tile_w {
                        if offset + 1 >= data.len() {
                            break;
                        }
                        let word = u16::from_be_bytes([data[offset], data[offset + 1]]);
                        offset += 2;

                        let r = Self::expand5(((word >> 11) & 0x1F) as u8);
                        let g = Self::expand6(((word >> 5) & 0x3F) as u8);
                        let b = Self::expand5((word & 0x1F) as u8);

                        let px = tx * tile_w + col;
                        let py = ty * tile_h + row;
                        if px < width && py < height {
                            image.put_pixel(px, py, image::Rgba([r, g, b, 255]));
                        }
                    }
                }
            }
        }

        Ok(image)
    }

    // -- RGB5A3 (4x4 tiles) ---------------------------------------------

    fn decode_rgb5a3(data: &[u8], width: u32, height: u32) -> Result<RgbaImage> {
        let mut image = RgbaImage::new(width, height);
        let tile_w: u32 = 4;
        let tile_h: u32 = 4;
        let tiles_x = width.div_ceil(tile_w);
        let tiles_y = height.div_ceil(tile_h);
        let mut offset = 0usize;

        for ty in 0..tiles_y {
            for tx in 0..tiles_x {
                for row in 0..tile_h {
                    for col in 0..tile_w {
                        if offset + 1 >= data.len() {
                            break;
                        }
                        let word = u16::from_be_bytes([data[offset], data[offset + 1]]);
                        offset += 2;

                        let (r, g, b, a) = if (word & 0x8000) != 0 {
                            // RGB555, opaque
                            (
                                Self::expand5(((word >> 10) & 0x1F) as u8),
                                Self::expand5(((word >> 5) & 0x1F) as u8),
                                Self::expand5((word & 0x1F) as u8),
                                255u8,
                            )
                        } else {
                            // RGB4A3
                            let a3 = ((word >> 12) & 0x7) as u8;
                            (
                                (((word >> 8) & 0xF) as u8) * 17,
                                (((word >> 4) & 0xF) as u8) * 17,
                                ((word & 0xF) as u8) * 17,
                                (a3 << 5) | (a3 << 2) | (a3 >> 1),
                            )
                        };

                        let px = tx * tile_w + col;
                        let py = ty * tile_h + row;
                        if px < width && py < height {
                            image.put_pixel(px, py, image::Rgba([r, g, b, a]));
                        }
                    }
                }
            }
        }

        Ok(image)
    }

    // -- RGBA8 (4x4 tiles, split AR/GB planes) --------------------------

    fn decode_rgba8(data: &[u8], width: u32, height: u32) -> Result<RgbaImage> {
        let mut image = RgbaImage::new(width, height);
        let tile_w: u32 = 4;
        let tile_h: u32 = 4;
        let tiles_x = width.div_ceil(tile_w);
        let tiles_y = height.div_ceil(tile_h);
        let mut offset = 0usize;

        // RGBA8 tiles store 32 bytes of AR pairs then 32 bytes of GB pairs.
        let tile_size = 64usize; // 16 pixels × 4 bytes

        for ty in 0..tiles_y {
            for tx in 0..tiles_x {
                if offset + tile_size > data.len() {
                    break;
                }

                let ar = &data[offset..offset + 32];
                let gb = &data[offset + 32..offset + 64];
                offset += tile_size;

                for row in 0..tile_h {
                    for col in 0..tile_w {
                        let i = (row * tile_w + col) as usize;
                        let a = ar[i * 2];
                        let r = ar[i * 2 + 1];
                        let g = gb[i * 2];
                        let b = gb[i * 2 + 1];

                        let px = tx * tile_w + col;
                        let py = ty * tile_h + row;
                        if px < width && py < height {
                            image.put_pixel(px, py, image::Rgba([r, g, b, a]));
                        }
                    }
                }
            }
        }

        Ok(image)
    }
}
