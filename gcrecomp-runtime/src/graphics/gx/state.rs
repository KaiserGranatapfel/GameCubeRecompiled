// GX (Graphics eXecutor) state machine for GameCube GPU emulation.
//
// The GameCube's GPU ("Flipper") contains the GX graphics pipeline, which
// includes a programmable TEV (Texture Environment) unit with up to 16 stages,
// flexible vertex attribute loading, matrix stacks, and fixed-function blend
// and depth testing. This module models the full mutable state of the GX
// pipeline as a single coherent struct, suitable for driving a wgpu backend.

// ---------------------------------------------------------------------------
// Vertex attribute types
// ---------------------------------------------------------------------------

/// Identifies one of the 21 vertex attribute slots defined by GX.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum VtxAttr {
    PositionMatrixIdx = 0,
    Tex0MatrixIdx = 1,
    Tex1MatrixIdx = 2,
    Tex2MatrixIdx = 3,
    Tex3MatrixIdx = 4,
    Tex4MatrixIdx = 5,
    Tex5MatrixIdx = 6,
    Tex6MatrixIdx = 7,
    Tex7MatrixIdx = 8,
    Position = 9,
    Normal = 10,
    Color0 = 11,
    Color1 = 12,
    Tex0 = 13,
    Tex1 = 14,
    Tex2 = 15,
    Tex3 = 16,
    Tex4 = 17,
    Tex5 = 18,
    Tex6 = 19,
    Tex7 = 20,
}

impl VtxAttr {
    pub const COUNT: usize = 21;

    /// Return the attribute corresponding to an index (0..=20), if valid.
    pub fn from_index(i: u8) -> Option<Self> {
        if i <= 20 {
            // SAFETY: repr(u8) with contiguous discriminants 0..=20.
            Some(unsafe { std::mem::transmute::<u8, VtxAttr>(i) })
        } else {
            None
        }
    }
}

/// How vertex data for a particular attribute is supplied.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum VtxInputType {
    /// Attribute is not present in the vertex.
    #[default]
    None = 0,
    /// Data is inlined in the vertex stream.
    Direct = 1,
    /// 8-bit index into an external array.
    Index8 = 2,
    /// 16-bit index into an external array.
    Index16 = 3,
}

/// Descriptor for a single vertex attribute: which attribute slot it occupies
/// and how the data is sourced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VtxDesc {
    pub attr: VtxAttr,
    pub input_type: VtxInputType,
}

impl VtxDesc {
    pub const fn new(attr: VtxAttr, input_type: VtxInputType) -> Self {
        Self { attr, input_type }
    }
}

/// Per-format-table description of a single attribute's binary layout.
///
/// * `component_count` -- e.g. 2 for XY, 3 for XYZ.
/// * `component_type`  -- encodes the GX component type enum (u8/s8/u16/s16/f32).
/// * `frac_bits`       -- fixed-point fractional bit count (0 for float).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct VtxAttrFmt {
    pub component_count: u8,
    pub component_type: u8,
    pub frac_bits: u8,
}

// ---------------------------------------------------------------------------
// TEV (Texture Environment) stage
// ---------------------------------------------------------------------------

/// A single TEV combiner stage. The GameCube supports up to 16 cascaded
/// stages, each blending up to four color and four alpha inputs using a
/// configurable operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TevStage {
    // Color combiner inputs (GX_CC_* selectors).
    pub color_in_a: u8,
    pub color_in_b: u8,
    pub color_in_c: u8,
    pub color_in_d: u8,

    // Alpha combiner inputs (GX_CA_* selectors).
    pub alpha_in_a: u8,
    pub alpha_in_b: u8,
    pub alpha_in_c: u8,
    pub alpha_in_d: u8,

    /// Color combiner operation (GX_TEV_ADD, GX_TEV_SUB, ...).
    pub color_op: u8,
    /// Alpha combiner operation.
    pub alpha_op: u8,

    /// Whether to clamp the color result to [0,1].
    pub color_clamp: bool,
    /// Whether to clamp the alpha result to [0,1].
    pub alpha_clamp: bool,

    /// Output scale for color (0=1x, 1=2x, 2=4x, 3=0.5x).
    pub color_scale: u8,
    /// Output scale for alpha.
    pub alpha_scale: u8,

    /// Destination register for color output (GX_TEVPREV..GX_TEVREG2).
    pub color_dest: u8,
    /// Destination register for alpha output.
    pub alpha_dest: u8,

    /// Texture coordinate generator index used by this stage.
    pub tex_coord: u8,
    /// Texture map index used by this stage.
    pub tex_map: u8,
    /// Color channel feeding this stage (GX_COLOR0A0, GX_COLOR1A1, ...).
    pub channel: u8,
}

impl Default for TevStage {
    fn default() -> Self {
        Self {
            // Default: pass-through from CPREV
            color_in_a: 0x0F, // GX_CC_ZERO
            color_in_b: 0x0F, // GX_CC_ZERO
            color_in_c: 0x0F, // GX_CC_ZERO
            color_in_d: 0x00, // GX_CC_CPREV
            alpha_in_a: 0x07, // GX_CA_ZERO
            alpha_in_b: 0x07, // GX_CA_ZERO
            alpha_in_c: 0x07, // GX_CA_ZERO
            alpha_in_d: 0x00, // GX_CA_APREV
            color_op: 0,      // GX_TEV_ADD
            alpha_op: 0,      // GX_TEV_ADD
            color_clamp: true,
            alpha_clamp: true,
            color_scale: 0, // 1x
            alpha_scale: 0, // 1x
            color_dest: 0,  // GX_TEVPREV
            alpha_dest: 0,  // GX_TEVPREV
            tex_coord: 0xFF,
            tex_map: 0xFF,
            channel: 0xFF,
        }
    }
}

// ---------------------------------------------------------------------------
// Blend, depth, and rasterizer state
// ---------------------------------------------------------------------------

/// Blend-mode factor selectors matching GX blend factor enums.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BlendFactor {
    Zero = 0,
    #[default]
    One = 1,
    SrcColor = 2,
    InvSrcColor = 3,
    SrcAlpha = 4,
    InvSrcAlpha = 5,
    DstAlpha = 6,
    InvDstAlpha = 7,
}

/// Logic-op selectors (used when blend type is GX_BM_LOGIC).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum LogicOp {
    Clear = 0,
    And = 1,
    RevAnd = 2,
    #[default]
    Copy = 3,
    InvAnd = 4,
    Noop = 5,
    Xor = 6,
    Or = 7,
    Nor = 8,
    Equiv = 9,
    Inv = 10,
    RevOr = 11,
    InvCopy = 12,
    InvOr = 13,
    Nand = 14,
    Set = 15,
}

/// Full blend-mode state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlendMode {
    pub enabled: bool,
    pub src_factor: BlendFactor,
    pub dst_factor: BlendFactor,
    pub logic_op: LogicOp,
}

impl Default for BlendMode {
    fn default() -> Self {
        Self {
            enabled: false,
            src_factor: BlendFactor::SrcAlpha,
            dst_factor: BlendFactor::InvSrcAlpha,
            logic_op: LogicOp::Copy,
        }
    }
}

/// GX compare function, shared by depth test and alpha compare.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CompareFunction {
    Never = 0,
    Less = 1,
    Equal = 2,
    #[default]
    LessEqual = 3,
    Greater = 4,
    NotEqual = 5,
    GreaterEqual = 6,
    Always = 7,
}

/// Z-buffer (depth) mode state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZMode {
    pub enable: bool,
    pub function: CompareFunction,
    pub update: bool,
}

impl Default for ZMode {
    fn default() -> Self {
        Self {
            enable: true,
            function: CompareFunction::LessEqual,
            update: true,
        }
    }
}

/// Scissor rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scissor {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Default for Scissor {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 640,
            height: 480,
        }
    }
}

/// Viewport transform parameters (maps clip space to screen space).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 640.0,
            height: 480.0,
            near: 0.0,
            far: 1.0,
        }
    }
}

/// Face-culling mode.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CullMode {
    None = 0,
    Front = 1,
    #[default]
    Back = 2,
    All = 3,
}

// ---------------------------------------------------------------------------
// Matrix state
// ---------------------------------------------------------------------------

/// Identity 4x4 matrix in column-major order.
const IDENTITY_4X4: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, //
    0.0, 1.0, 0.0, 0.0, //
    0.0, 0.0, 1.0, 0.0, //
    0.0, 0.0, 0.0, 1.0,
];

/// All matrix arrays managed by GX.
///
/// The GameCube provides 10 position/normal matrix slots and 10 texture
/// matrix slots, plus a single projection matrix. Matrices are stored in
/// column-major layout as flat `[f32; 16]` arrays for easy upload to the GPU.
#[derive(Debug, Clone)]
pub struct GxMatrices {
    /// Current projection matrix (perspective or orthographic).
    pub projection: [f32; 16],
    /// Position/normal matrix array (indexed 0..9).
    pub position: [[f32; 16]; 10],
    /// Texture coordinate matrix array (indexed 0..9).
    pub texture: [[f32; 16]; 10],
    /// Index of the currently active position/normal matrix (0..9).
    pub current_position_mtx: u8,
}

impl Default for GxMatrices {
    fn default() -> Self {
        Self {
            projection: IDENTITY_4X4,
            position: [IDENTITY_4X4; 10],
            texture: [IDENTITY_4X4; 10],
            current_position_mtx: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Top-level GX state
// ---------------------------------------------------------------------------

/// Complete mutable state of the GameCube GX graphics pipeline.
///
/// An instance of this struct represents every register that a game can
/// modify through the GX API before issuing draw calls. The runtime
/// translates this state into the corresponding wgpu pipeline and bind-group
/// configuration each time a draw is flushed.
#[derive(Debug, Clone)]
pub struct GxState {
    // -- Vertex layout ---------------------------------------------------
    /// Per-attribute descriptors defining which attributes are present and
    /// how they are sourced (none / direct / indexed).
    pub vertex_descriptors: [VtxDesc; VtxAttr::COUNT],

    /// Eight vertex-format tables (GX_VTXFMT0..GX_VTXFMT7). Each table
    /// contains one `VtxAttrFmt` per attribute, describing the binary
    /// layout (component count, type, fractional bits).
    pub vertex_formats: [[VtxAttrFmt; VtxAttr::COUNT]; 8],

    // -- TEV pipeline ----------------------------------------------------
    /// The 16 TEV combiner stages.
    pub tev_stages: [TevStage; 16],

    /// Number of active TEV stages (1..=16).
    pub num_tev_stages: u8,

    /// Four TEV color registers: CPREV, C0, C1, C2 (RGBA as `[f32; 4]`).
    pub tev_colors: [[f32; 4]; 4],

    /// Four TEV constant-color registers (RGBA).
    pub tev_konst_colors: [[f32; 4]; 4],

    // -- Transform -------------------------------------------------------
    /// Projection, position, and texture matrices.
    pub matrices: GxMatrices,

    // -- Rasterizer / output merger --------------------------------------
    /// Framebuffer blend mode.
    pub blend_mode: BlendMode,

    /// Depth-buffer test and write configuration.
    pub z_mode: ZMode,

    /// Scissor rectangle (in EFB coordinates).
    pub scissor: Scissor,

    /// Viewport transform.
    pub viewport: Viewport,

    /// Triangle face-culling mode.
    pub cull_mode: CullMode,

    // -- Lighting / channels ---------------------------------------------
    /// Two material channel diffuse colors (RGBA).
    pub material_colors: [[f32; 4]; 2],

    /// Two ambient channel colors (RGBA).
    pub ambient_colors: [[f32; 4]; 2],

    /// Number of active color channels (0..=2).
    pub num_channels: u8,

    /// Number of active texture-coordinate generators (0..=8).
    pub num_tex_gens: u8,

    // -- Copy / clear ----------------------------------------------------
    /// Clear color used by EFB-to-XFB copy (RGBA).
    pub copy_clear_color: [f32; 4],

    /// Clear Z value used by EFB-to-XFB copy (24-bit depth).
    pub copy_clear_z: u32,

    // -- Per-pixel write masks -------------------------------------------
    /// Whether color channels (RGB) are written to the EFB.
    pub color_update: bool,

    /// Whether the alpha channel is written to the EFB.
    pub alpha_update: bool,
}

// Helper: build the default vertex descriptor array with all inputs as None.
fn default_vertex_descriptors() -> [VtxDesc; VtxAttr::COUNT] {
    [
        VtxDesc::new(VtxAttr::PositionMatrixIdx, VtxInputType::None),
        VtxDesc::new(VtxAttr::Tex0MatrixIdx, VtxInputType::None),
        VtxDesc::new(VtxAttr::Tex1MatrixIdx, VtxInputType::None),
        VtxDesc::new(VtxAttr::Tex2MatrixIdx, VtxInputType::None),
        VtxDesc::new(VtxAttr::Tex3MatrixIdx, VtxInputType::None),
        VtxDesc::new(VtxAttr::Tex4MatrixIdx, VtxInputType::None),
        VtxDesc::new(VtxAttr::Tex5MatrixIdx, VtxInputType::None),
        VtxDesc::new(VtxAttr::Tex6MatrixIdx, VtxInputType::None),
        VtxDesc::new(VtxAttr::Tex7MatrixIdx, VtxInputType::None),
        VtxDesc::new(VtxAttr::Position, VtxInputType::None),
        VtxDesc::new(VtxAttr::Normal, VtxInputType::None),
        VtxDesc::new(VtxAttr::Color0, VtxInputType::None),
        VtxDesc::new(VtxAttr::Color1, VtxInputType::None),
        VtxDesc::new(VtxAttr::Tex0, VtxInputType::None),
        VtxDesc::new(VtxAttr::Tex1, VtxInputType::None),
        VtxDesc::new(VtxAttr::Tex2, VtxInputType::None),
        VtxDesc::new(VtxAttr::Tex3, VtxInputType::None),
        VtxDesc::new(VtxAttr::Tex4, VtxInputType::None),
        VtxDesc::new(VtxAttr::Tex5, VtxInputType::None),
        VtxDesc::new(VtxAttr::Tex6, VtxInputType::None),
        VtxDesc::new(VtxAttr::Tex7, VtxInputType::None),
    ]
}

impl GxState {
    /// Create a new `GxState` initialized to sane power-on defaults that
    /// match the GameCube's boot-time GX configuration.
    pub fn new() -> Self {
        Self {
            vertex_descriptors: default_vertex_descriptors(),
            vertex_formats: [[VtxAttrFmt::default(); VtxAttr::COUNT]; 8],

            tev_stages: [TevStage::default(); 16],
            num_tev_stages: 1,
            tev_colors: [[0.0; 4]; 4],
            tev_konst_colors: [[1.0; 4]; 4],

            matrices: GxMatrices::default(),

            blend_mode: BlendMode::default(),
            z_mode: ZMode::default(),
            scissor: Scissor::default(),
            viewport: Viewport::default(),
            cull_mode: CullMode::default(),

            material_colors: [[1.0, 1.0, 1.0, 1.0]; 2],
            ambient_colors: [[0.0, 0.0, 0.0, 1.0]; 2],
            num_channels: 1,
            num_tex_gens: 0,

            copy_clear_color: [0.0, 0.0, 0.0, 1.0],
            copy_clear_z: 0x00FF_FFFF, // max 24-bit depth

            color_update: true,
            alpha_update: true,
        }
    }

    /// Reset the entire GX state to power-on defaults.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    // -- Vertex descriptor helpers ---------------------------------------

    /// Set the input type for a single vertex attribute.
    pub fn set_vtx_desc(&mut self, attr: VtxAttr, input_type: VtxInputType) {
        self.vertex_descriptors[attr as usize].input_type = input_type;
    }

    /// Clear all vertex attribute descriptors to `None`.
    pub fn clear_vtx_descs(&mut self) {
        for desc in &mut self.vertex_descriptors {
            desc.input_type = VtxInputType::None;
        }
    }

    /// Set the format of a vertex attribute within a specific format table.
    pub fn set_vtx_attr_fmt(
        &mut self,
        fmt_index: u8,
        attr: VtxAttr,
        component_count: u8,
        component_type: u8,
        frac_bits: u8,
    ) {
        let table = &mut self.vertex_formats[fmt_index as usize];
        table[attr as usize] = VtxAttrFmt {
            component_count,
            component_type,
            frac_bits,
        };
    }

    // -- TEV helpers -----------------------------------------------------

    /// Configure the color combiner inputs for a TEV stage.
    pub fn set_tev_color_in(&mut self, stage: u8, a: u8, b: u8, c: u8, d: u8) {
        let s = &mut self.tev_stages[stage as usize];
        s.color_in_a = a;
        s.color_in_b = b;
        s.color_in_c = c;
        s.color_in_d = d;
    }

    /// Configure the alpha combiner inputs for a TEV stage.
    pub fn set_tev_alpha_in(&mut self, stage: u8, a: u8, b: u8, c: u8, d: u8) {
        let s = &mut self.tev_stages[stage as usize];
        s.alpha_in_a = a;
        s.alpha_in_b = b;
        s.alpha_in_c = c;
        s.alpha_in_d = d;
    }

    /// Configure the color combiner operation for a TEV stage.
    pub fn set_tev_color_op(&mut self, stage: u8, op: u8, clamp: bool, scale: u8, dest: u8) {
        let s = &mut self.tev_stages[stage as usize];
        s.color_op = op;
        s.color_clamp = clamp;
        s.color_scale = scale;
        s.color_dest = dest;
    }

    /// Configure the alpha combiner operation for a TEV stage.
    pub fn set_tev_alpha_op(&mut self, stage: u8, op: u8, clamp: bool, scale: u8, dest: u8) {
        let s = &mut self.tev_stages[stage as usize];
        s.alpha_op = op;
        s.alpha_clamp = clamp;
        s.alpha_scale = scale;
        s.alpha_dest = dest;
    }

    /// Bind a texture coordinate generator and texture map to a TEV stage.
    pub fn set_tev_order(&mut self, stage: u8, tex_coord: u8, tex_map: u8, channel: u8) {
        let s = &mut self.tev_stages[stage as usize];
        s.tex_coord = tex_coord;
        s.tex_map = tex_map;
        s.channel = channel;
    }

    /// Set a TEV color register (0=CPREV, 1=C0, 2=C1, 3=C2).
    pub fn set_tev_color(&mut self, reg: u8, r: f32, g: f32, b: f32, a: f32) {
        self.tev_colors[reg as usize] = [r, g, b, a];
    }

    /// Set a TEV constant-color register.
    pub fn set_tev_konst_color(&mut self, reg: u8, r: f32, g: f32, b: f32, a: f32) {
        self.tev_konst_colors[reg as usize] = [r, g, b, a];
    }

    // -- Matrix helpers --------------------------------------------------

    /// Load a 4x4 projection matrix (column-major).
    pub fn set_projection(&mut self, mtx: &[f32; 16]) {
        self.matrices.projection = *mtx;
    }

    /// Load a 4x4 position/normal matrix into a specific slot.
    pub fn set_position_matrix(&mut self, index: u8, mtx: &[f32; 16]) {
        self.matrices.position[index as usize] = *mtx;
    }

    /// Set which position matrix slot is currently active.
    pub fn set_current_position_matrix(&mut self, index: u8) {
        self.matrices.current_position_mtx = index;
    }

    /// Load a 4x4 texture matrix into a specific slot.
    pub fn set_texture_matrix(&mut self, index: u8, mtx: &[f32; 16]) {
        self.matrices.texture[index as usize] = *mtx;
    }

    // -- Blend / depth / rasterizer helpers ------------------------------

    /// Set the framebuffer blend mode.
    pub fn set_blend_mode(
        &mut self,
        enabled: bool,
        src: BlendFactor,
        dst: BlendFactor,
        logic: LogicOp,
    ) {
        self.blend_mode = BlendMode {
            enabled,
            src_factor: src,
            dst_factor: dst,
            logic_op: logic,
        };
    }

    /// Set the Z-buffer (depth) mode.
    pub fn set_z_mode(&mut self, enable: bool, function: CompareFunction, update: bool) {
        self.z_mode = ZMode {
            enable,
            function,
            update,
        };
    }

    /// Set the scissor rectangle.
    pub fn set_scissor(&mut self, x: u16, y: u16, w: u16, h: u16) {
        self.scissor = Scissor {
            x,
            y,
            width: w,
            height: h,
        };
    }

    /// Set the viewport transform.
    pub fn set_viewport(&mut self, x: f32, y: f32, w: f32, h: f32, near: f32, far: f32) {
        self.viewport = Viewport {
            x,
            y,
            width: w,
            height: h,
            near,
            far,
        };
    }

    /// Set the triangle face-culling mode.
    pub fn set_cull_mode(&mut self, mode: CullMode) {
        self.cull_mode = mode;
    }

    // -- Channel / lighting helpers --------------------------------------

    /// Set a material channel color (index 0 or 1).
    pub fn set_material_color(&mut self, index: u8, r: f32, g: f32, b: f32, a: f32) {
        self.material_colors[index as usize] = [r, g, b, a];
    }

    /// Set an ambient channel color (index 0 or 1).
    pub fn set_ambient_color(&mut self, index: u8, r: f32, g: f32, b: f32, a: f32) {
        self.ambient_colors[index as usize] = [r, g, b, a];
    }

    // -- Copy / clear helpers --------------------------------------------

    /// Set the EFB copy clear color.
    pub fn set_copy_clear_color(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.copy_clear_color = [r, g, b, a];
    }

    /// Set the EFB copy clear depth value (24-bit).
    pub fn set_copy_clear_z(&mut self, z: u32) {
        self.copy_clear_z = z & 0x00FF_FFFF;
    }

    /// Set per-pixel color and alpha write enables.
    pub fn set_color_update(&mut self, color: bool, alpha: bool) {
        self.color_update = color;
        self.alpha_update = alpha;
    }
}

impl Default for GxState {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_default_has_sane_values() {
        let state = GxState::new();
        assert_eq!(state.num_tev_stages, 1);
        assert_eq!(state.num_channels, 1);
        assert_eq!(state.num_tex_gens, 0);
        assert!(state.z_mode.enable);
        assert!(state.color_update);
        assert!(state.alpha_update);
        assert_eq!(state.cull_mode, CullMode::Back);
        assert_eq!(state.copy_clear_z, 0x00FF_FFFF);
    }

    #[test]
    fn vtx_attr_round_trip() {
        for i in 0..=20u8 {
            let attr = VtxAttr::from_index(i).unwrap();
            assert_eq!(attr as u8, i);
        }
        assert!(VtxAttr::from_index(21).is_none());
    }

    #[test]
    fn reset_restores_defaults() {
        let mut state = GxState::new();
        state.num_tev_stages = 8;
        state.cull_mode = CullMode::None;
        state.z_mode.enable = false;
        state.set_blend_mode(true, BlendFactor::One, BlendFactor::Zero, LogicOp::Noop);
        state.reset();
        assert_eq!(state.num_tev_stages, 1);
        assert_eq!(state.cull_mode, CullMode::Back);
        assert!(state.z_mode.enable);
        assert!(!state.blend_mode.enabled);
    }

    #[test]
    fn set_vtx_desc_modifies_correct_slot() {
        let mut state = GxState::new();
        state.set_vtx_desc(VtxAttr::Position, VtxInputType::Direct);
        state.set_vtx_desc(VtxAttr::Normal, VtxInputType::Index16);
        assert_eq!(
            state.vertex_descriptors[VtxAttr::Position as usize].input_type,
            VtxInputType::Direct,
        );
        assert_eq!(
            state.vertex_descriptors[VtxAttr::Normal as usize].input_type,
            VtxInputType::Index16,
        );
    }

    #[test]
    fn clear_vtx_descs_resets_all() {
        let mut state = GxState::new();
        state.set_vtx_desc(VtxAttr::Position, VtxInputType::Direct);
        state.set_vtx_desc(VtxAttr::Color0, VtxInputType::Index8);
        state.clear_vtx_descs();
        for desc in &state.vertex_descriptors {
            assert_eq!(desc.input_type, VtxInputType::None);
        }
    }

    #[test]
    fn tev_stage_configuration() {
        let mut state = GxState::new();
        state.set_tev_color_in(0, 0x08, 0x0C, 0x0A, 0x0F);
        let s = &state.tev_stages[0];
        assert_eq!(s.color_in_a, 0x08);
        assert_eq!(s.color_in_b, 0x0C);
        assert_eq!(s.color_in_c, 0x0A);
        assert_eq!(s.color_in_d, 0x0F);
    }

    #[test]
    fn projection_matrix_load() {
        let mut state = GxState::new();
        let mut mtx = [0.0f32; 16];
        mtx[0] = 2.0;
        mtx[5] = 2.0;
        mtx[10] = -1.0;
        mtx[15] = 1.0;
        state.set_projection(&mtx);
        assert_eq!(state.matrices.projection[0], 2.0);
        assert_eq!(state.matrices.projection[10], -1.0);
    }

    #[test]
    fn copy_clear_z_masked_to_24_bits() {
        let mut state = GxState::new();
        state.set_copy_clear_z(0xFFFF_FFFF);
        assert_eq!(state.copy_clear_z, 0x00FF_FFFF);
    }
}
