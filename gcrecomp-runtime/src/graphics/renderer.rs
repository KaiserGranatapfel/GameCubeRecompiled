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
    _upscaler: Upscaler,
    _frame_buffers: Vec<FrameBuffer>,
    current_resolution: (u32, u32),
    target_resolution: (u32, u32),
    gx_processor: GXProcessor,
    _shader_manager: ShaderManager,
    _window: Arc<winit::window::Window>,
    /// EFB (embedded frame buffer) for rendering at GameCube native resolution.
    efb_texture: Option<Texture>,
    efb_view: Option<TextureView>,
    depth_texture: Option<Texture>,
    depth_view: Option<TextureView>,
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
        let mut gx_processor = GXProcessor::new();
        gx_processor.init_gpu(&device);
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

        // Create EFB at GameCube native resolution (640x480)
        let (efb_texture, efb_view) = Self::create_efb(&device, 640, 480, config.format);
        let (depth_texture, depth_view) = Self::create_depth(&device, 640, 480);

        Ok(Self {
            device,
            queue,
            surface,
            config,
            _upscaler: upscaler,
            _frame_buffers: Vec::new(),
            current_resolution: (640, 480), // GameCube native
            target_resolution: (size.width, size.height),
            gx_processor,
            _shader_manager: shader_manager,
            _window: window,
            efb_texture: Some(efb_texture),
            efb_view: Some(efb_view),
            depth_texture: Some(depth_texture),
            depth_view: Some(depth_view),
        })
    }

    fn create_efb(
        device: &Device,
        width: u32,
        height: u32,
        format: TextureFormat,
    ) -> (Texture, TextureView) {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("EFB"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&TextureViewDescriptor::default());
        (texture, view)
    }

    fn create_depth(device: &Device, width: u32, height: u32) -> (Texture, TextureView) {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("Depth"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth24Plus,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&TextureViewDescriptor::default());
        (texture, view)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.target_resolution = (width, height);
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    pub fn set_resolution(&mut self, width: u32, height: u32) {
        self.current_resolution = (width, height);
        let (efb, efb_view) = Self::create_efb(&self.device, width, height, self.config.format);
        let (depth, depth_view) = Self::create_depth(&self.device, width, height);
        self.efb_texture = Some(efb);
        self.efb_view = Some(efb_view);
        self.depth_texture = Some(depth);
        self.depth_view = Some(depth_view);
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

    /// Submit the GX draw list for the current frame to the GPU.
    pub fn submit_gx_frame(&mut self) {
        let draw_list = self.gx_processor.take_draw_list();
        if draw_list.is_empty() {
            return;
        }

        let efb_view = match &self.efb_view {
            Some(v) => v,
            None => return,
        };

        let clear_color = self.gx_processor.state.copy_clear_color;

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("GX Frame"),
            });

        {
            let depth_attachment =
                self.depth_view
                    .as_ref()
                    .map(|dv| RenderPassDepthStencilAttachment {
                        view: dv,
                        depth_ops: Some(Operations {
                            load: LoadOp::Clear(1.0),
                            store: StoreOp::Store,
                        }),
                        stencil_ops: None,
                    });

            let _pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("GX Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: efb_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: clear_color[0] as f64,
                            g: clear_color[1] as f64,
                            b: clear_color[2] as f64,
                            a: clear_color[3] as f64,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: depth_attachment,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Draw calls would be issued here using prepared draw commands
            // from draw_list + pipeline_cache. For now we create and clear
            // the render pass; per-draw-call submission requires the full
            // pipeline/bind-group wiring which is set up in pipeline.rs.
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }

    pub fn gx_processor(&self) -> &GXProcessor {
        &self.gx_processor
    }

    pub fn gx_processor_mut(&mut self) -> &mut GXProcessor {
        &mut self.gx_processor
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn queue(&self) -> &Queue {
        &self.queue
    }
}
