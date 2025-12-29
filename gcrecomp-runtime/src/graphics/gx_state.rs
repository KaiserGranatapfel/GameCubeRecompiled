//! GX processor state management
//!
//! Handles blending modes, fog, alpha testing, depth testing, and other rendering state

use glam::{Mat4, Vec3, Vec4};

/// Blending mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendingMode {
    None,
    Alpha,
    Additive,
    Subtractive,
    Multiply,
}

/// Alpha test function
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlphaTestFunc {
    Never,
    Less,
    Equal,
    LessEqual,
    Greater,
    NotEqual,
    GreaterEqual,
    Always,
}

/// Depth test function
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepthTestFunc {
    Never,
    Less,
    Equal,
    LessEqual,
    Greater,
    NotEqual,
    GreaterEqual,
    Always,
}

/// Fog mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FogMode {
    None,
    Linear,
    Exponential,
    Exponential2,
}

/// GX rendering state
#[derive(Debug, Clone)]
pub struct GXRenderingState {
    /// Blending mode
    pub blending: BlendingMode,
    /// Alpha test enabled
    pub alpha_test_enabled: bool,
    /// Alpha test function
    pub alpha_test_func: AlphaTestFunc,
    /// Alpha test reference value
    pub alpha_test_ref: f32,
    /// Depth test enabled
    pub depth_test_enabled: bool,
    /// Depth test function
    pub depth_test_func: DepthTestFunc,
    /// Depth write enabled
    pub depth_write_enabled: bool,
    /// Fog enabled
    pub fog_enabled: bool,
    /// Fog mode
    pub fog_mode: FogMode,
    /// Fog start distance
    pub fog_start: f32,
    /// Fog end distance
    pub fog_end: f32,
    /// Fog color
    pub fog_color: Vec4,
    /// Model-view matrix
    pub model_view: Mat4,
    /// Normal matrix (for lighting)
    pub normal_matrix: Mat4,
    /// Scissor box (x, y, width, height)
    pub scissor: Option<(f32, f32, f32, f32)>,
}

impl Default for GXRenderingState {
    fn default() -> Self {
        Self {
            blending: BlendingMode::None,
            alpha_test_enabled: false,
            alpha_test_func: AlphaTestFunc::Always,
            alpha_test_ref: 0.0,
            depth_test_enabled: true,
            depth_test_func: DepthTestFunc::Less,
            depth_write_enabled: true,
            fog_enabled: false,
            fog_mode: FogMode::None,
            fog_start: 0.0,
            fog_end: 1.0,
            fog_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            model_view: Mat4::IDENTITY,
            normal_matrix: Mat4::IDENTITY,
            scissor: None,
        }
    }
}

impl GXRenderingState {
    /// Create new rendering state
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply fog to a vertex color based on distance
    pub fn apply_fog(&self, vertex_color: Vec4, distance: f32) -> Vec4 {
        if !self.fog_enabled {
            return vertex_color;
        }

        let fog_factor = match self.fog_mode {
            FogMode::None => 1.0,
            FogMode::Linear => {
                if distance <= self.fog_start {
                    1.0
                } else if distance >= self.fog_end {
                    0.0
                } else {
                    1.0 - (distance - self.fog_start) / (self.fog_end - self.fog_start)
                }
            }
            FogMode::Exponential => {
                let density = 1.0 / (self.fog_end - self.fog_start);
                (-density * distance).exp()
            }
            FogMode::Exponential2 => {
                let density = 1.0 / (self.fog_end - self.fog_start);
                let exp = (-density * distance).exp();
                exp * exp
            }
        };

        // Interpolate between vertex color and fog color
        vertex_color.lerp(self.fog_color, 1.0 - fog_factor)
    }

    /// Test alpha value
    pub fn test_alpha(&self, alpha: f32) -> bool {
        if !self.alpha_test_enabled {
            return true;
        }

        match self.alpha_test_func {
            AlphaTestFunc::Never => false,
            AlphaTestFunc::Less => alpha < self.alpha_test_ref,
            AlphaTestFunc::Equal => (alpha - self.alpha_test_ref).abs() < f32::EPSILON,
            AlphaTestFunc::LessEqual => alpha <= self.alpha_test_ref,
            AlphaTestFunc::Greater => alpha > self.alpha_test_ref,
            AlphaTestFunc::NotEqual => (alpha - self.alpha_test_ref).abs() >= f32::EPSILON,
            AlphaTestFunc::GreaterEqual => alpha >= self.alpha_test_ref,
            AlphaTestFunc::Always => true,
        }
    }

    /// Test depth value
    pub fn test_depth(&self, depth: f32, stored_depth: f32) -> bool {
        if !self.depth_test_enabled {
            return true;
        }

        match self.depth_test_func {
            DepthTestFunc::Never => false,
            DepthTestFunc::Less => depth < stored_depth,
            DepthTestFunc::Equal => (depth - stored_depth).abs() < f32::EPSILON,
            DepthTestFunc::LessEqual => depth <= stored_depth,
            DepthTestFunc::Greater => depth > stored_depth,
            DepthTestFunc::NotEqual => (depth - stored_depth).abs() >= f32::EPSILON,
            DepthTestFunc::GreaterEqual => depth >= stored_depth,
            DepthTestFunc::Always => true,
        }
    }

    /// Apply blending to colors
    pub fn blend_colors(&self, src: Vec4, dst: Vec4) -> Vec4 {
        match self.blending {
            BlendingMode::None => src,
            BlendingMode::Alpha => {
                let alpha = src.w;
                src * alpha + dst * (1.0 - alpha)
            }
            BlendingMode::Additive => {
                Vec4::new(
                    (src.x + dst.x).min(1.0),
                    (src.y + dst.y).min(1.0),
                    (src.z + dst.z).min(1.0),
                    (src.w + dst.w).min(1.0),
                )
            }
            BlendingMode::Subtractive => {
                Vec4::new(
                    (src.x - dst.x).max(0.0),
                    (src.y - dst.y).max(0.0),
                    (src.z - dst.z).max(0.0),
                    (src.w - dst.w).max(0.0),
                )
            }
            BlendingMode::Multiply => Vec4::new(
                src.x * dst.x,
                src.y * dst.y,
                src.z * dst.z,
                src.w * dst.w,
            ),
        }
    }
}

/// Transform a vertex position using model-view-projection matrix
pub fn transform_vertex(
    position: Vec3,
    model_view: Mat4,
    projection: Mat4,
    viewport: (f32, f32, f32, f32), // x, y, width, height
) -> (Vec3, f32) {
    // Transform to clip space
    let clip_pos = projection * model_view * position.extend(1.0);
    
    // Perspective divide
    let ndc = clip_pos.xyz() / clip_pos.w;
    
    // Transform to viewport space
    let viewport_pos = Vec3::new(
        viewport.0 + (ndc.x + 1.0) * 0.5 * viewport.2,
        viewport.1 + (1.0 - ndc.y) * 0.5 * viewport.3, // Flip Y
        (ndc.z + 1.0) * 0.5, // Depth in [0, 1]
    );
    
    (viewport_pos, clip_pos.w)
}

