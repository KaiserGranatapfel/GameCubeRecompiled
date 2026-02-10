// GX vertex buffer accumulation system
//
// Implements the GameCube GX vertex submission pipeline.
// Between GXBegin and GXEnd, the game submits individual vertex
// components (position, normal, color, texcoord) which are
// accumulated into a flat f32 buffer for GPU upload.

use log::warn;

// ── GX primitive types ──────────────────────────────────────────

/// GameCube GX primitive types, matching hardware command values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GxPrimitive {
    Quads = 0x80,
    Triangles = 0x90,
    TriangleStrip = 0x98,
    TriangleFan = 0xA0,
    Lines = 0xA8,
    LineStrip = 0xB0,
    Points = 0xB8,
}

impl GxPrimitive {
    /// Decode a raw `u8` command byte into a primitive type.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x80 => Some(Self::Quads),
            0x90 => Some(Self::Triangles),
            0x98 => Some(Self::TriangleStrip),
            0xA0 => Some(Self::TriangleFan),
            0xA8 => Some(Self::Lines),
            0xB0 => Some(Self::LineStrip),
            0xB8 => Some(Self::Points),
            _ => None,
        }
    }
}

// ── Per-vertex staging area ─────────────────────────────────────

/// Staging area for a single vertex being assembled from
/// individual component submissions.
#[derive(Debug, Clone)]
pub struct CurrentVertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub color: [[u8; 4]; 2],
    pub texcoord: [[f32; 2]; 8],

    pub has_position: bool,
    pub has_normal: bool,
    pub has_color: [bool; 2],
    pub has_texcoord: [bool; 8],
}

impl Default for CurrentVertex {
    fn default() -> Self {
        Self {
            pos: [0.0; 3],
            normal: [0.0; 3],
            color: [[0; 4]; 2],
            texcoord: [[0.0; 2]; 8],

            has_position: false,
            has_normal: false,
            has_color: [false; 2],
            has_texcoord: [false; 8],
        }
    }
}

impl CurrentVertex {
    /// Reset all "has" flags and zero the staging data.
    fn clear(&mut self) {
        self.pos = [0.0; 3];
        self.normal = [0.0; 3];
        self.color = [[0; 4]; 2];
        self.texcoord = [[0.0; 2]; 8];
        self.has_position = false;
        self.has_normal = false;
        self.has_color = [false; 2];
        self.has_texcoord = [false; 8];
    }
}

// ── Completed draw call ─────────────────────────────────────────

/// A completed draw call produced by `VertexAccumulator::end`.
#[derive(Debug, Clone)]
pub struct DrawCall {
    /// The GX primitive type for this draw call.
    pub primitive: GxPrimitive,
    /// Interleaved vertex data as flat f32 values.
    pub vertex_data: Vec<f32>,
    /// Number of vertices in this draw call.
    pub vertex_count: u16,
    /// Number of f32 values per vertex (stride).
    pub stride: u32,
}

// ── Vertex accumulator ──────────────────────────────────────────

/// Accumulates vertex data between GXBegin / GXEnd pairs.
///
/// The game calls `begin` to start a primitive, then submits
/// individual vertex components via `position_3f32`, `normal_3f32`,
/// etc. When a vertex is complete (position has been submitted and
/// all expected attributes provided), `flush_vertex` packs the
/// data into the flat `vertices` buffer.  Finally, `end` returns
/// the completed `DrawCall`.
pub struct VertexAccumulator {
    /// Flat interleaved vertex data accumulated so far.
    vertices: Vec<f32>,
    /// Current primitive type.
    primitive: GxPrimitive,
    /// Active vertex format index (VTX_FMT 0-7).
    vertex_format: u8,
    /// Total vertex count expected for this draw call.
    expected_count: u16,
    /// Number of vertices flushed so far.
    current_count: u16,
    /// `true` while between `begin` and `end`.
    active: bool,
    /// Staging area for the vertex currently being assembled.
    current_vertex: CurrentVertex,
}

impl Default for VertexAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

impl VertexAccumulator {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            primitive: GxPrimitive::Triangles,
            vertex_format: 0,
            expected_count: 0,
            current_count: 0,
            active: false,
            current_vertex: CurrentVertex::default(),
        }
    }

    // ── Public accessors ────────────────────────────────────────

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn primitive(&self) -> GxPrimitive {
        self.primitive
    }

    pub fn vertex_format(&self) -> u8 {
        self.vertex_format
    }

    pub fn current_count(&self) -> u16 {
        self.current_count
    }

    pub fn expected_count(&self) -> u16 {
        self.expected_count
    }

    // ── Begin / End ─────────────────────────────────────────────

    /// Start accumulating vertices for a new primitive.
    ///
    /// `primitive_raw` is the raw GX command byte (e.g. 0x90 for
    /// triangles).  `vtx_fmt` selects one of the eight hardware
    /// vertex formats.  `count` is the number of vertices the game
    /// intends to send.
    pub fn begin(&mut self, primitive_raw: u8, vtx_fmt: u8, count: u16) {
        if self.active {
            warn!(
                "GX begin called while already active \
                 (primitive 0x{:02X}, dropped {} of {} verts)",
                primitive_raw,
                self.expected_count - self.current_count,
                self.expected_count,
            );
        }

        let primitive = match GxPrimitive::from_u8(primitive_raw) {
            Some(p) => p,
            None => {
                warn!(
                    "Unknown GX primitive 0x{:02X}, \
                     defaulting to Triangles",
                    primitive_raw,
                );
                GxPrimitive::Triangles
            }
        };

        self.primitive = primitive;
        self.vertex_format = vtx_fmt;
        self.expected_count = count;
        self.current_count = 0;
        self.active = true;
        self.vertices.clear();
        self.current_vertex.clear();
    }

    /// Finalize the current draw call and return the result.
    ///
    /// Returns `None` if no vertices were accumulated or if
    /// `begin` was never called.
    pub fn end(&mut self) -> Option<DrawCall> {
        if !self.active {
            warn!("GX end called without matching begin");
            return None;
        }

        // Flush any partially-built vertex that has a position.
        if self.current_vertex.has_position {
            self.flush_vertex();
        }

        self.active = false;

        if self.current_count != self.expected_count {
            warn!(
                "GX end: expected {} vertices but got {}",
                self.expected_count, self.current_count,
            );
        }

        if self.current_count == 0 {
            return None;
        }

        let stride = self.compute_stride();

        Some(DrawCall {
            primitive: self.primitive,
            vertex_data: std::mem::take(&mut self.vertices),
            vertex_count: self.current_count,
            stride,
        })
    }

    // ── Attribute submissions ───────────────────────────────────

    /// Submit a 3-component f32 position.
    pub fn position_3f32(&mut self, x: f32, y: f32, z: f32) {
        if !self.active {
            warn!("position_3f32 called outside begin/end");
            return;
        }

        // If the previous vertex already has a position queued,
        // flush it before starting the next vertex.
        if self.current_vertex.has_position {
            self.flush_vertex();
        }

        self.current_vertex.pos = [x, y, z];
        self.current_vertex.has_position = true;
    }

    /// Submit a 3-component s16 position (converted to f32).
    pub fn position_3s16(&mut self, x: i16, y: i16, z: i16) {
        self.position_3f32(x as f32, y as f32, z as f32);
    }

    /// Submit a 3-component f32 normal.
    pub fn normal_3f32(&mut self, x: f32, y: f32, z: f32) {
        if !self.active {
            warn!("normal_3f32 called outside begin/end");
            return;
        }
        self.current_vertex.normal = [x, y, z];
        self.current_vertex.has_normal = true;
    }

    /// Submit an RGBA color for color channel 0.
    pub fn color_4u8(&mut self, r: u8, g: u8, b: u8, a: u8) {
        if !self.active {
            warn!("color_4u8 called outside begin/end");
            return;
        }
        self.current_vertex.color[0] = [r, g, b, a];
        self.current_vertex.has_color[0] = true;
    }

    /// Submit an RGBA color for color channel 1.
    pub fn color1_4u8(&mut self, r: u8, g: u8, b: u8, a: u8) {
        if !self.active {
            warn!("color1_4u8 called outside begin/end");
            return;
        }
        self.current_vertex.color[1] = [r, g, b, a];
        self.current_vertex.has_color[1] = true;
    }

    /// Submit a 2-component f32 texture coordinate.
    ///
    /// Coordinates are appended to the first unused texcoord
    /// slot in the current vertex.
    pub fn texcoord_2f32(&mut self, s: f32, t: f32) {
        if !self.active {
            warn!("texcoord_2f32 called outside begin/end");
            return;
        }

        // Find the first texcoord slot that has not been set.
        let slot = self
            .current_vertex
            .has_texcoord
            .iter()
            .position(|&set| !set);

        match slot {
            Some(i) => {
                self.current_vertex.texcoord[i] = [s, t];
                self.current_vertex.has_texcoord[i] = true;
            }
            None => {
                warn!(
                    "texcoord_2f32: all 8 texcoord slots \
                     already filled"
                );
            }
        }
    }

    // ── Internal helpers ────────────────────────────────────────

    /// Pack the current vertex into the flat `vertices` buffer and
    /// reset the staging area for the next vertex.
    ///
    /// The layout written per vertex is:
    ///   position (3 f32)
    ///   [normal  (3 f32)]  -- if present
    ///   [color0  (4 f32)]  -- if present (u8 -> 0..1 f32)
    ///   [color1  (4 f32)]  -- if present
    ///   [tc0     (2 f32)]  -- for each texcoord present
    ///   ...
    fn flush_vertex(&mut self) {
        if !self.current_vertex.has_position {
            warn!("flush_vertex called without position data");
            return;
        }

        // Position -- always present.
        self.vertices.extend_from_slice(&self.current_vertex.pos);

        // Normal
        if self.current_vertex.has_normal {
            self.vertices.extend_from_slice(&self.current_vertex.normal);
        }

        // Color channel 0
        if self.current_vertex.has_color[0] {
            let c = &self.current_vertex.color[0];
            self.vertices.push(c[0] as f32 / 255.0);
            self.vertices.push(c[1] as f32 / 255.0);
            self.vertices.push(c[2] as f32 / 255.0);
            self.vertices.push(c[3] as f32 / 255.0);
        }

        // Color channel 1
        if self.current_vertex.has_color[1] {
            let c = &self.current_vertex.color[1];
            self.vertices.push(c[0] as f32 / 255.0);
            self.vertices.push(c[1] as f32 / 255.0);
            self.vertices.push(c[2] as f32 / 255.0);
            self.vertices.push(c[3] as f32 / 255.0);
        }

        // Texture coordinates (only the slots that were set).
        for i in 0..8 {
            if self.current_vertex.has_texcoord[i] {
                self.vertices
                    .extend_from_slice(&self.current_vertex.texcoord[i]);
            }
        }

        self.current_count += 1;
        self.current_vertex.clear();
    }

    /// Compute the number of f32 values per vertex (stride) based
    /// on the attribute layout of the **first** flushed vertex.
    ///
    /// This is safe because all vertices within a single GXBegin /
    /// GXEnd pair share the same format.
    fn compute_stride(&self) -> u32 {
        if self.current_count == 0 {
            return 0;
        }
        (self.vertices.len() as u32)
            .checked_div(self.current_count as u32)
            .unwrap_or(0)
    }
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_triangle() {
        let mut acc = VertexAccumulator::new();
        acc.begin(0x90, 0, 3); // Triangles, VTX_FMT 0, 3 verts

        acc.position_3f32(0.0, 1.0, 0.0);
        acc.position_3f32(-1.0, -1.0, 0.0);
        acc.position_3f32(1.0, -1.0, 0.0);

        let dc = acc.end().expect("should produce a draw call");
        assert_eq!(dc.primitive, GxPrimitive::Triangles);
        assert_eq!(dc.vertex_count, 3);
        assert_eq!(dc.stride, 3); // position only
        assert_eq!(dc.vertex_data.len(), 9);
    }

    #[test]
    fn position_with_color_and_texcoord() {
        let mut acc = VertexAccumulator::new();
        acc.begin(0x98, 0, 1); // TriangleStrip

        acc.position_3f32(1.0, 2.0, 3.0);
        acc.color_4u8(255, 0, 128, 255);
        acc.texcoord_2f32(0.5, 0.75);

        let dc = acc.end().expect("should produce a draw call");
        assert_eq!(dc.vertex_count, 1);
        // 3 (pos) + 4 (color) + 2 (texcoord) = 9
        assert_eq!(dc.stride, 9);
        assert_eq!(dc.vertex_data.len(), 9);

        // Verify color normalisation
        assert!((dc.vertex_data[3] - 1.0).abs() < f32::EPSILON);
        assert!((dc.vertex_data[4] - 0.0).abs() < f32::EPSILON);
        let expected_g = 128.0 / 255.0;
        assert!((dc.vertex_data[5] - expected_g).abs() < 1e-4);
    }

    #[test]
    fn s16_position() {
        let mut acc = VertexAccumulator::new();
        acc.begin(0xB8, 0, 1); // Points

        acc.position_3s16(100, -200, 300);

        let dc = acc.end().expect("should produce a draw call");
        assert_eq!(dc.vertex_data[0], 100.0);
        assert_eq!(dc.vertex_data[1], -200.0);
        assert_eq!(dc.vertex_data[2], 300.0);
    }

    #[test]
    fn end_without_begin_returns_none() {
        let mut acc = VertexAccumulator::new();
        assert!(acc.end().is_none());
    }

    #[test]
    fn empty_draw_returns_none() {
        let mut acc = VertexAccumulator::new();
        acc.begin(0x90, 0, 3);
        // Submit zero vertices
        assert!(acc.end().is_none());
    }

    #[test]
    fn primitive_from_u8_roundtrip() {
        let cases: &[(u8, GxPrimitive)] = &[
            (0x80, GxPrimitive::Quads),
            (0x90, GxPrimitive::Triangles),
            (0x98, GxPrimitive::TriangleStrip),
            (0xA0, GxPrimitive::TriangleFan),
            (0xA8, GxPrimitive::Lines),
            (0xB0, GxPrimitive::LineStrip),
            (0xB8, GxPrimitive::Points),
        ];
        for &(raw, expected) in cases {
            assert_eq!(GxPrimitive::from_u8(raw), Some(expected),);
        }
        assert_eq!(GxPrimitive::from_u8(0xFF), None);
    }

    #[test]
    fn dual_color_channels() {
        let mut acc = VertexAccumulator::new();
        acc.begin(0x90, 0, 1);

        acc.position_3f32(0.0, 0.0, 0.0);
        acc.color_4u8(255, 255, 255, 255);
        acc.color1_4u8(0, 0, 0, 0);

        let dc = acc.end().expect("should produce a draw call");
        // 3 (pos) + 4 (color0) + 4 (color1) = 11
        assert_eq!(dc.stride, 11);
    }

    #[test]
    fn multiple_texcoords() {
        let mut acc = VertexAccumulator::new();
        acc.begin(0x90, 0, 1);

        acc.position_3f32(1.0, 0.0, 0.0);
        acc.texcoord_2f32(0.0, 0.0);
        acc.texcoord_2f32(1.0, 1.0);

        let dc = acc.end().expect("should produce a draw call");
        // 3 (pos) + 2 (tc0) + 2 (tc1) = 7
        assert_eq!(dc.stride, 7);
        assert_eq!(dc.vertex_data[3], 0.0);
        assert_eq!(dc.vertex_data[4], 0.0);
        assert_eq!(dc.vertex_data[5], 1.0);
        assert_eq!(dc.vertex_data[6], 1.0);
    }
}
