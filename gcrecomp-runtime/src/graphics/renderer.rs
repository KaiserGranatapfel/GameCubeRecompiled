// Main renderer
use crate::graphics::framebuffer::FrameBuffer;
use crate::graphics::gx::GXProcessor;
use crate::graphics::shaders::ShaderManager;
use crate::graphics::upscaler::Upscaler;
use anyhow::Result;
use std::sync::Arc;
use wgpu::*;

pub struct Renderer {
    device: Device,
    queue: Queue,
    surface: Surface<'static>,
    config: SurfaceConfiguration,
    upscaler: Upscaler,
    frame_buffers: Vec<FrameBuffer>,
    current_resolution: (u32, u32),
    target_resolution: (u32, u32),
    gx_processor: GXProcessor,
    shader_manager: ShaderManager,
    _window: Arc<winit::window::Window>,
}

impl Renderer {
    pub fn new(window: Arc<winit::window::Window>) -> Result<Self> {
        let instance = Instance::new(InstanceDescriptor::default());
        // SAFETY: The window is stored in Arc in the struct, ensuring it outlives the surface
        let surface = instance.create_surface(window.clone())?;

        let adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .ok_or_else(|| anyhow::anyhow!("Failed to find suitable GPU adapter"))?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &DeviceDescriptor {
                label: None,
                required_features: Features::empty(),
                required_limits: Limits::default(),
            },
            None,
        ))?;

        let size = window.inner_size();
        let config = surface
            .get_default_config(&adapter, size.width, size.height)
            .ok_or_else(|| anyhow::anyhow!("Failed to get surface config"))?;

        surface.configure(&device, &config);

        let upscaler = Upscaler::new(&device, &config)?;
        let gx_processor = GXProcessor::new();
        let mut shader_manager = ShaderManager::new();

        // Load default shaders
        let default_vert = r#"
            struct VertexInput {
                @location(0) position: vec3<f32>,
                @location(1) tex_coord: vec2<f32>,
            }
            struct VertexOutput {
                @builtin(position) position: vec4<f32>,
                @location(0) tex_coord: vec2<f32>,
            }
            @vertex
            fn main(input: VertexInput) -> VertexOutput {
                var output: VertexOutput;
                output.position = vec4<f32>(input.position, 1.0);
                output.tex_coord = input.tex_coord;
                return output;
            }
        "#;

        let default_frag = r#"
            @group(0) @binding(0) var texture: texture_2d<f32>;
            @group(0) @binding(1) var sampler: sampler;
            struct FragmentInput {
                @location(0) tex_coord: vec2<f32>,
            }
            @fragment
            fn main(input: FragmentInput) -> @location(0) vec4<f32> {
                return textureSample(texture, sampler, input.tex_coord);
            }
        "#;

        shader_manager.load_shader(&device, "default_vertex", default_vert)?;
        shader_manager.load_shader(&device, "default_fragment", default_frag)?;

        Ok(Self {
            device,
            queue,
            surface,
            config,
            upscaler,
            frame_buffers: Vec::new(),
            current_resolution: (640, 480), // GameCube native
            target_resolution: (size.width, size.height),
            gx_processor,
            shader_manager,
            _window: window,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.target_resolution = (width, height);
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    pub fn set_resolution(&mut self, width: u32, height: u32) {
        self.current_resolution = (width, height);
    }

    pub fn set_upscale_factor(&mut self, factor: f32) -> Result<()> {
        let target_w = (self.current_resolution.0 as f32 * factor) as u32;
        let target_h = (self.current_resolution.1 as f32 * factor) as u32;
        self.target_resolution = (target_w, target_h);
        Ok(())
    }

    pub fn begin_frame(&mut self) -> Result<wgpu::SurfaceTexture> {
        let output = self.surface.get_current_texture()?;
        Ok(output)
    }

    pub fn end_frame(&mut self, frame: wgpu::SurfaceTexture) {
        frame.present();
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn queue(&self) -> &Queue {
        &self.queue
    }
}
