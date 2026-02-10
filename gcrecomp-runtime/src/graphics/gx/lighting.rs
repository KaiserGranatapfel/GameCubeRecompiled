/// GX lighting / color channel configuration.
/// A single color channel configuration (material + ambient + light enable).
#[derive(Debug, Clone, Copy)]
pub struct ColorChannel {
    pub mat_src: ColorSrc,
    pub amb_src: ColorSrc,
    pub light_mask: u8,
    pub diff_fn: DiffuseFunction,
    pub attn_fn: AttenuationFunction,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSrc {
    Register = 0,
    Vertex = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffuseFunction {
    None = 0,
    Sign = 1,
    Clamp = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttenuationFunction {
    Off = 0,
    Spec = 1,
    Spot = 2,
}

impl Default for ColorChannel {
    fn default() -> Self {
        Self {
            mat_src: ColorSrc::Register,
            amb_src: ColorSrc::Register,
            light_mask: 0,
            diff_fn: DiffuseFunction::None,
            attn_fn: AttenuationFunction::Off,
            enabled: false,
        }
    }
}

/// Light channel state for the GX processor.
#[derive(Debug, Clone)]
pub struct LightingState {
    pub channels: [ColorChannel; 4], // 2 color + 2 alpha channels
    pub num_channels: u8,
    pub material_colors: [[f32; 4]; 2],
    pub ambient_colors: [[f32; 4]; 2],
}

impl LightingState {
    pub fn new() -> Self {
        Self {
            channels: [ColorChannel::default(); 4],
            num_channels: 0,
            material_colors: [[1.0, 1.0, 1.0, 1.0]; 2],
            ambient_colors: [[0.0, 0.0, 0.0, 1.0]; 2],
        }
    }

    pub fn set_num_channels(&mut self, n: u8) {
        self.num_channels = n.min(2);
    }

    pub fn set_chan_ctrl(
        &mut self,
        channel: u8,
        enable: bool,
        amb_src: u8,
        mat_src: u8,
        light_mask: u8,
        diff_fn: u8,
        attn_fn: u8,
    ) {
        if (channel as usize) < 4 {
            let ch = &mut self.channels[channel as usize];
            ch.enabled = enable;
            ch.amb_src = if amb_src == 0 {
                ColorSrc::Register
            } else {
                ColorSrc::Vertex
            };
            ch.mat_src = if mat_src == 0 {
                ColorSrc::Register
            } else {
                ColorSrc::Vertex
            };
            ch.light_mask = light_mask;
            ch.diff_fn = match diff_fn {
                1 => DiffuseFunction::Sign,
                2 => DiffuseFunction::Clamp,
                _ => DiffuseFunction::None,
            };
            ch.attn_fn = match attn_fn {
                1 => AttenuationFunction::Spec,
                2 => AttenuationFunction::Spot,
                _ => AttenuationFunction::Off,
            };
        }
    }

    pub fn set_mat_color(&mut self, channel: u8, r: u8, g: u8, b: u8, a: u8) {
        if (channel as usize) < 2 {
            self.material_colors[channel as usize] = [
                r as f32 / 255.0,
                g as f32 / 255.0,
                b as f32 / 255.0,
                a as f32 / 255.0,
            ];
        }
    }

    pub fn set_amb_color(&mut self, channel: u8, r: u8, g: u8, b: u8, a: u8) {
        if (channel as usize) < 2 {
            self.ambient_colors[channel as usize] = [
                r as f32 / 255.0,
                g as f32 / 255.0,
                b as f32 / 255.0,
                a as f32 / 255.0,
            ];
        }
    }
}

impl Default for LightingState {
    fn default() -> Self {
        Self::new()
    }
}
