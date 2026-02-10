// GX (Graphics eXecutor) — GameCube GPU pipeline emulation.
//
// Submodules implement individual hardware subsystems; `GXProcessor`
// is the top-level façade exposed to the rest of the runtime.

pub mod draw;
pub mod lighting;
pub mod pipeline;
pub mod state;
pub mod tev;
pub mod transform;
pub mod vertex;

use self::pipeline::PipelineCache;
use self::state::GxState;
use self::vertex::{DrawCall, VertexAccumulator};

/// Top-level GX processor that games interact with through SDK calls.
///
/// Owns the full mutable GX state, vertex accumulator, draw list for
/// the current frame, and the wgpu pipeline cache.
pub struct GXProcessor {
    /// All GX register state (vertex descriptors, TEV, blend, matrices, …).
    pub state: GxState,
    /// Vertex buffer accumulator for the current `GXBegin` / `GXEnd`.
    accumulator: VertexAccumulator,
    /// Completed draw calls for the current frame (flushed on `copy_disp`).
    draw_list: Vec<DrawCall>,
    /// Cached wgpu render pipelines keyed by GX state hash.
    pipeline_cache: PipelineCache,
}

impl GXProcessor {
    pub fn new() -> Self {
        Self {
            state: GxState::new(),
            accumulator: VertexAccumulator::new(),
            draw_list: Vec::new(),
            pipeline_cache: PipelineCache::new(),
        }
    }

    /// Initialize wgpu-dependent resources (bind group / pipeline layouts).
    pub fn init_gpu(&mut self, device: &wgpu::Device) {
        self.pipeline_cache.init_layouts(device);
    }

    // -- Vertex submission (GXBegin / GXEnd wrappers) --------------------

    pub fn begin(&mut self, primitive: u8, vtx_fmt: u8, count: u16) {
        self.accumulator.begin(primitive, vtx_fmt, count);
    }

    pub fn end(&mut self) {
        if let Some(dc) = self.accumulator.end() {
            self.draw_list.push(dc);
        }
    }

    pub fn position_3f32(&mut self, x: f32, y: f32, z: f32) {
        self.accumulator.position_3f32(x, y, z);
    }

    pub fn position_3s16(&mut self, x: i16, y: i16, z: i16) {
        self.accumulator.position_3s16(x, y, z);
    }

    pub fn normal_3f32(&mut self, x: f32, y: f32, z: f32) {
        self.accumulator.normal_3f32(x, y, z);
    }

    pub fn color_4u8(&mut self, r: u8, g: u8, b: u8, a: u8) {
        self.accumulator.color_4u8(r, g, b, a);
    }

    pub fn texcoord_2f32(&mut self, s: f32, t: f32) {
        self.accumulator.texcoord_2f32(s, t);
    }

    // -- Frame lifecycle -------------------------------------------------

    /// Take the accumulated draw list for rendering and clear it.
    pub fn take_draw_list(&mut self) -> Vec<DrawCall> {
        std::mem::take(&mut self.draw_list)
    }

    /// Pipeline cache accessor.
    pub fn pipeline_cache(&self) -> &PipelineCache {
        &self.pipeline_cache
    }

    pub fn pipeline_cache_mut(&mut self) -> &mut PipelineCache {
        &mut self.pipeline_cache
    }

    /// Reset all GX state to power-on defaults.
    pub fn reset(&mut self) {
        self.state.reset();
        self.draw_list.clear();
        self.pipeline_cache.clear();
    }
}

impl Default for GXProcessor {
    fn default() -> Self {
        Self::new()
    }
}
