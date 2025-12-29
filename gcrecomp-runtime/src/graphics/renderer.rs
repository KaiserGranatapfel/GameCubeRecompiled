// Main renderer
use crate::graphics::framebuffer::FrameBuffer;
use crate::graphics::gx::{GXProcessor, TextureObject};
use crate::graphics::post_processing::PostProcessor;
use crate::graphics::shaders::ShaderManager;
use crate::graphics::splash::SplashScreen;
use crate::graphics::upscaler::Upscaler;
use crate::texture::{GameCubeTextureFormat, TextureLoader};
use anyhow::Result;
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
    pending_vertex_count: u32,
    vertex_buffer: Option<wgpu::Buffer>,
    render_pipeline: Option<wgpu::RenderPipeline>,
    texture_loader: TextureLoader,
    current_texture: Option<wgpu::Texture>,
    texture_bind_group: Option<wgpu::BindGroup>,
    splash_screen: Option<SplashScreen>,
    post_processor: Option<PostProcessor>,
    anisotropic_filtering: u32, // 0 = off, 2, 4, 8, 16
}

impl Renderer {
    pub fn new(window: &winit::window::Window) -> Result<Self> {
        let instance = Instance::new(InstanceDescriptor::default());
        let surface = instance.create_surface(window)?;

        let adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .ok_or_else(|| anyhow::anyhow!("Failed to find suitable GPU adapter"))?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &DeviceDescriptor {
                label: None,
                features: Features::empty(),
                limits: Limits::default(),
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
        let texture_loader = TextureLoader::new();

        // Load default shaders
        let default_vert = r#"
            struct VertexInput {
                @location(0) position: vec3<f32>,
                @location(1) tex_coord: vec2<f32>,
                @location(2) color: vec4<f32>,
            }
            struct VertexOutput {
                @builtin(position) position: vec4<f32>,
                @location(0) tex_coord: vec2<f32>,
                @location(1) color: vec4<f32>,
            }
            @vertex
            fn main(input: VertexInput) -> VertexOutput {
                var output: VertexOutput;
                output.position = vec4<f32>(input.position, 1.0);
                output.tex_coord = input.tex_coord;
                output.color = input.color;
                return output;
            }
        "#;

        let default_frag = r#"
            struct FragmentInput {
                @location(0) tex_coord: vec2<f32>,
                @location(1) color: vec4<f32>,
            }
            @fragment
            fn main(input: FragmentInput) -> @location(0) vec4<f32> {
                // For now, just return vertex color
                // Texture sampling would be added when textures are implemented
                return input.color;
            }
        "#;

        shader_manager.load_shader(&device, "default_vertex", default_vert)?;
        shader_manager.load_shader(&device, "default_fragment", default_frag)?;

        // Create splash screen
        let splash_screen = SplashScreen::new(&device, &queue, &config).ok();

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
            pending_vertex_count: 0,
            vertex_buffer: None,
            render_pipeline: None,
            texture_loader,
            current_texture: None,
            texture_bind_group: None,
            splash_screen,
            post_processor: PostProcessor::new(&device, &queue, &config).ok(),
            anisotropic_filtering: 0,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.target_resolution = (width, height);
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        
        if let Some(ref mut post_processor) = self.post_processor {
            post_processor.resize(width, height).ok();
        }
    }

    /// Set anisotropic filtering level (0 = off, 2, 4, 8, 16)
    pub fn set_anisotropic_filtering(&mut self, level: u32) {
        self.anisotropic_filtering = level;
    }

    /// Get post-processor
    pub fn post_processor(&self) -> Option<&PostProcessor> {
        self.post_processor.as_ref()
    }

    /// Get mutable post-processor
    pub fn post_processor_mut(&mut self) -> Option<&mut PostProcessor> {
        self.post_processor.as_mut()
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

    /// Process GX commands and translate to wgpu operations
    ///
    /// This method processes queued GX commands and translates them
    /// to wgpu rendering operations.
    pub fn process_gx_commands(&mut self) -> Result<()> {
        // Get viewport from GX processor
        let viewport = self.gx_processor.viewport();
        
        // Update renderer viewport if changed
        if viewport.width > 0.0 && viewport.height > 0.0 {
            self.current_resolution = (viewport.width as u32, viewport.height as u32);
        }

        // Process queued draw commands
        let vertices = self.gx_processor.get_pending_vertices();
        
        if !vertices.is_empty() {
            // Create vertex buffer from GX vertices
            // Convert GameCube vertex format to wgpu-compatible format
            let vertex_data: Vec<u8> = vertices
                .iter()
                .flat_map(|v| {
                    // Pack vertex data: position (12 bytes) + tex_coord (8 bytes) + color (16 bytes) = 36 bytes
                    let mut data = Vec::with_capacity(36);
                    // Position (3 f32 = 12 bytes)
                    data.extend_from_slice(&v.position[0].to_le_bytes());
                    data.extend_from_slice(&v.position[1].to_le_bytes());
                    data.extend_from_slice(&v.position[2].to_le_bytes());
                    // Tex coord (2 f32 = 8 bytes)
                    data.extend_from_slice(&v.tex_coord[0].to_le_bytes());
                    data.extend_from_slice(&v.tex_coord[1].to_le_bytes());
                    // Color (4 f32 = 16 bytes)
                    data.extend_from_slice(&v.color[0].to_le_bytes());
                    data.extend_from_slice(&v.color[1].to_le_bytes());
                    data.extend_from_slice(&v.color[2].to_le_bytes());
                    data.extend_from_slice(&v.color[3].to_le_bytes());
                    data
                })
                .collect();

            // Create wgpu buffer (we'll use this in render_frame)
            // For now, just store the vertex count for rendering
            self.pending_vertex_count = vertices.len() as u32;
        }

        // Flush GX command queue
        self.gx_processor.flush_commands()?;

        Ok(())
    }

    /// Get mutable reference to GX processor
    pub fn gx_processor_mut(&mut self) -> &mut GXProcessor {
        &mut self.gx_processor
    }

    /// Get reference to GX processor
    pub fn gx_processor(&self) -> Option<&GXProcessor> {
        Some(&self.gx_processor)
    }

    /// Set memory reader for GX processor
    pub fn set_gx_memory_reader<F>(&mut self, reader: F)
    where
        F: Fn(u32, usize) -> Result<Vec<u8>> + Send + Sync + 'static,
    {
        self.gx_processor.set_memory_reader(reader);
    }

    /// Set memory reader for GX processor
    pub fn set_gx_memory_reader<F>(&mut self, reader: F)
    where
        F: Fn(u32, usize) -> Result<Vec<u8>> + Send + Sync + 'static,
    {
        self.gx_processor.set_memory_reader(reader);
    }

    /// Render a frame with GX commands
    ///
    /// Processes GX commands and renders to the current frame buffer
    pub fn render_frame(&mut self) -> Result<()> {
        // Begin render pass
        let frame = self.begin_frame()?;
        let view = frame.texture.create_view(&TextureViewDescriptor::default());

        // Check if splash screen should be displayed
        if let Some(ref mut splash) = self.splash_screen {
            if splash.should_display() {
                splash.update(&self.queue);
                let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
                    label: Some("Splash Render Encoder"),
                });
                splash.render(&mut encoder, &view);
                self.queue.submit(Some(encoder.finish()));
                frame.present();
                return Ok(());
            } else {
                // Splash screen finished, remove it
                self.splash_screen = None;
            }
        }

        // Process GX commands (this populates vertex data)
        self.process_gx_commands()?;

        // Create render pass
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // Render GX vertices if any are pending
        if self.pending_vertex_count > 0 {
            let vertices = self.gx_processor.get_pending_vertices();
            
            // Create or update vertex buffer
            let vertex_data: Vec<u8> = vertices
                .iter()
                .flat_map(|v| {
                    let mut data = Vec::with_capacity(36);
                    // Position (3 f32)
                    data.extend_from_slice(&v.position[0].to_le_bytes());
                    data.extend_from_slice(&v.position[1].to_le_bytes());
                    data.extend_from_slice(&v.position[2].to_le_bytes());
                    // Tex coord (2 f32)
                    data.extend_from_slice(&v.tex_coord[0].to_le_bytes());
                    data.extend_from_slice(&v.tex_coord[1].to_le_bytes());
                    // Color (4 f32)
                    data.extend_from_slice(&v.color[0].to_le_bytes());
                    data.extend_from_slice(&v.color[1].to_le_bytes());
                    data.extend_from_slice(&v.color[2].to_le_bytes());
                    data.extend_from_slice(&v.color[3].to_le_bytes());
                    data
                })
                .collect();

            // Create vertex buffer if needed or update existing one
            let buffer = if let Some(ref buf) = self.vertex_buffer {
                if buf.size() < vertex_data.len() as u64 {
                    // Recreate buffer if too small
                    self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Vertex Buffer"),
                        contents: &vertex_data,
                        usage: wgpu::BufferUsages::VERTEX,
                    })
                } else {
                    // Update existing buffer
                    self.queue.write_buffer(buf, 0, &vertex_data);
                    buf.clone()
                }
            } else {
                self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: &vertex_data,
                    usage: wgpu::BufferUsages::VERTEX,
                })
            };
            self.vertex_buffer = Some(buffer.clone());

            // Create render pipeline if needed
            if self.render_pipeline.is_none() {
                let pipeline = self.create_render_pipeline()?;
                self.render_pipeline = Some(pipeline);
            }

            // Begin render pass with actual rendering
            {
                let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 1.0,
                            }),
                            store: StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                });

                // Set pipeline and draw
                if let Some(ref pipeline) = self.render_pipeline {
                    render_pass.set_pipeline(pipeline);
                    render_pass.set_vertex_buffer(0, buffer.slice(..));
                    render_pass.draw(0..self.pending_vertex_count, 0..1);
                }
            }

            // Clear pending vertices after rendering
            self.gx_processor.clear_pending_vertices();
            self.pending_vertex_count = 0;
        } else {
            // No vertices to render, just clear the screen
            {
                let _render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 1.0,
                            }),
                            store: StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                });
            }
        }

        // Submit commands
        self.queue.submit(std::iter::once(encoder.finish()));
        self.end_frame(frame);

        Ok(())
    }

    /// Create render pipeline for GX rendering
    fn create_render_pipeline(&self) -> Result<wgpu::RenderPipeline> {
        let shader = self.shader_manager.get_shader("default_vertex")
            .ok_or_else(|| anyhow::anyhow!("Default vertex shader not found"))?;

        let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("GX Render Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let pipeline = self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("GX Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 36, // 36 bytes per vertex (position + texcoord + color)
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        // Position: location 0, offset 0, format Float32x3
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        // Tex coord: location 1, offset 12, format Float32x2
                        wgpu::VertexAttribute {
                            offset: 12,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        // Color: location 2, offset 20, format Float32x4
                        wgpu::VertexAttribute {
                            offset: 20,
                            shader_location: 2,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: self.shader_manager.get_shader("default_fragment")
                    .ok_or_else(|| anyhow::anyhow!("Default fragment shader not found"))?,
                entry_point: Some("main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: self.config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Ok(pipeline)
    }
}
