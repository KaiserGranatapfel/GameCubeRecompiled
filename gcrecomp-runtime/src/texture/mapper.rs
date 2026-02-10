// Texture mapping

pub struct TextureMapper {
    // Handles texture coordinate mapping and UV transformations
}

impl Default for TextureMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl TextureMapper {
    pub fn new() -> Self {
        Self {}
    }

    pub fn map_coordinates(&self, u: f32, v: f32, wrap_mode: WrapMode) -> (f32, f32) {
        match wrap_mode {
            WrapMode::Clamp => (u.clamp(0.0, 1.0), v.clamp(0.0, 1.0)),
            WrapMode::Repeat => (u.fract(), v.fract()),
            WrapMode::Mirror => {
                let u_mirror = if (u as i32) % 2 == 0 {
                    u.fract()
                } else {
                    1.0 - u.fract()
                };
                let v_mirror = if (v as i32) % 2 == 0 {
                    v.fract()
                } else {
                    1.0 - v.fract()
                };
                (u_mirror, v_mirror)
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum WrapMode {
    Clamp,
    Repeat,
    Mirror,
}
