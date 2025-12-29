//! Splash screen with pixel art GameCube cube
//!
//! Displays a pixel art GameCube cube animation during startup

use anyhow::Result;
use std::time::Instant;
use wgpu::*;

pub struct SplashScreen {
    texture: Texture,
    texture_view: TextureView,
    sampler: Sampler,
    render_pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    uniform_buffer: Buffer,
    bind_group: BindGroup,
    start_time: Instant,
    fade_progress: f32,
    rotation: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    tex_coord: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    fade: f32,
    rotation: f32,
    _padding: [f32; 2],
}

impl Vertex {
    fn desc() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x2,
                },
                VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Float32x2,
                },
            ],
        }
    }
}

impl SplashScreen {
    /// Create a new splash screen
    pub fn new(device: &Device, queue: &Queue, config: &SurfaceConfiguration) -> Result<Self> {
        // Create pixel art GameCube cube texture (128x128)
        let cube_image = Self::create_gamecube_cube_image();
        let texture_size = Extent3d {
            width: 128,
            height: 128,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&TextureDescriptor {
            label: Some("Splash Cube Texture"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            &cube_image,
            ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * 128),
                rows_per_image: Some(128),
            },
            texture_size,
        );

        let texture_view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        // Create vertex buffer (fullscreen quad)
        let vertices = [
            Vertex {
                position: [-1.0, -1.0],
                tex_coord: [0.0, 1.0],
            },
            Vertex {
                position: [1.0, -1.0],
                tex_coord: [1.0, 1.0],
            },
            Vertex {
                position: [1.0, 1.0],
                tex_coord: [1.0, 0.0],
            },
            Vertex {
                position: [-1.0, 1.0],
                tex_coord: [0.0, 0.0],
            },
        ];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Splash Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: BufferUsages::VERTEX,
        });

        // Create uniform buffer
        let uniforms = Uniforms {
            fade: 0.0,
            rotation: 0.0,
            _padding: [0.0; 2],
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Splash Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Splash Bind Group Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Splash Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        // Create shader
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Splash Shader"),
            source: ShaderSource::Wgsl(include_str!("splash.wgsl").into()),
        });

        // Create render pipeline
        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Splash Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Splash Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: PipelineCompilationOptions::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                polygon_mode: PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Ok(Self {
            texture,
            texture_view,
            sampler,
            render_pipeline,
            vertex_buffer,
            uniform_buffer,
            bind_group,
            start_time: Instant::now(),
            fade_progress: 0.0,
            rotation: 0.0,
        })
    }

    /// Update splash screen animation
    pub fn update(&mut self, queue: &Queue) {
        let elapsed = self.start_time.elapsed().as_secs_f32();
        
        // Fade in over 0.5 seconds, hold for 2 seconds, fade out over 0.5 seconds
        if elapsed < 0.5 {
            self.fade_progress = elapsed / 0.5;
        } else if elapsed < 2.5 {
            self.fade_progress = 1.0;
        } else if elapsed < 3.0 {
            self.fade_progress = 1.0 - ((elapsed - 2.5) / 0.5);
        } else {
            self.fade_progress = 0.0;
        }

        // Rotate slowly
        self.rotation = elapsed * 0.5;

        // Update uniform buffer
        let uniforms = Uniforms {
            fade: self.fade_progress,
            rotation: self.rotation,
            _padding: [0.0; 2],
        };

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    /// Check if splash screen should be displayed
    pub fn should_display(&self) -> bool {
        self.start_time.elapsed().as_secs_f32() < 3.0
    }

    /// Render the splash screen
    pub fn render(&self, encoder: &mut CommandEncoder, view: &TextureView) {
        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Splash Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color {
                        r: 0.05,
                        g: 0.05,
                        b: 0.1,
                        a: 1.0,
                    }),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..6, 0..1);
    }

    /// Create pixel art GameCube cube image
    fn create_gamecube_cube_image() -> Vec<u8> {
        let size = 128;
        let mut image = vec![0u8; size * size * 4];

        // GameCube colors: purple/indigo
        let dark_purple = [0x2D, 0x1B, 0x4E, 0xFF];
        let purple = [0x5D, 0x3F, 0x8E, 0xFF];
        let light_purple = [0x8B, 0x6F, 0xBD, 0xFF];
        let highlight = [0xB8, 0xA5, 0xE0, 0xFF];

        // Draw a simple cube face (isometric view)
        for y in 0..size {
            for x in 0..size {
                let idx = (y * size + x) * 4;
                let fx = x as f32 / size as f32;
                let fy = y as f32 / size as f32;

                // Center the cube
                let cx = 0.5;
                let cy = 0.5;
                let dx = fx - cx;
                let dy = fy - cy;

                // Draw cube face with simple shading
                let dist = (dx * dx + dy * dy).sqrt();
                let color = if dist < 0.15 {
                    // Center highlight
                    highlight
                } else if dist < 0.25 {
                    // Light area
                    light_purple
                } else if dist < 0.35 {
                    // Medium area
                    purple
                } else if dist < 0.45 {
                    // Dark area
                    dark_purple
                } else {
                    // Background
                    [0x10, 0x10, 0x20, 0x00]
                };

                image[idx] = color[0];
                image[idx + 1] = color[1];
                image[idx + 2] = color[2];
                image[idx + 3] = color[3];
            }
        }

        image
    }
}

