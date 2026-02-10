/// Pipeline cache: creates/caches wgpu::RenderPipeline from GX state.
use std::collections::HashMap;
use wgpu::*;

/// Key derived from the GX state that determines which pipeline to use.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PipelineKey {
    pub num_tev_stages: u8,
    pub blend_src: u32,
    pub blend_dst: u32,
    pub z_enable: bool,
    pub z_write: bool,
    pub z_func: u8,
    pub cull_mode: u8,
    pub color_update: bool,
    pub alpha_update: bool,
    pub primitive_topology: u32,
}

pub struct PipelineCache {
    cache: HashMap<PipelineKey, RenderPipeline>,
    bind_group_layout: Option<BindGroupLayout>,
    pipeline_layout: Option<PipelineLayout>,
}

impl PipelineCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            bind_group_layout: None,
            pipeline_layout: None,
        }
    }

    /// Initialize the shared bind group layout and pipeline layout.
    pub fn init_layouts(&mut self, device: &Device) {
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("GX Bind Group Layout"),
            entries: &[
                // Binding 0: uniform buffer (matrices + colors)
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Binding 1: texture
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Binding 2: sampler
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("GX Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        self.bind_group_layout = Some(bind_group_layout);
        self.pipeline_layout = Some(pipeline_layout);
    }

    pub fn bind_group_layout(&self) -> Option<&BindGroupLayout> {
        self.bind_group_layout.as_ref()
    }

    /// Get or create a render pipeline for the given key and shaders.
    pub fn get_or_create(
        &mut self,
        device: &Device,
        key: &PipelineKey,
        vertex_shader: &ShaderModule,
        fragment_shader: &ShaderModule,
        surface_format: TextureFormat,
    ) -> &RenderPipeline {
        if !self.cache.contains_key(key) {
            let pipeline =
                self.create_pipeline(device, key, vertex_shader, fragment_shader, surface_format);
            self.cache.insert(key.clone(), pipeline);
        }
        self.cache.get(key).unwrap()
    }

    fn create_pipeline(
        &self,
        device: &Device,
        key: &PipelineKey,
        vertex_shader: &ShaderModule,
        fragment_shader: &ShaderModule,
        surface_format: TextureFormat,
    ) -> RenderPipeline {
        let cull_mode = match key.cull_mode {
            1 => Some(Face::Front),
            2 => Some(Face::Back),
            _ => None,
        };

        let topology = match key.primitive_topology {
            1 => PrimitiveTopology::LineList,
            2 => PrimitiveTopology::LineStrip,
            3 => PrimitiveTopology::TriangleList,
            4 => PrimitiveTopology::TriangleStrip,
            5 => PrimitiveTopology::PointList,
            _ => PrimitiveTopology::TriangleList,
        };

        let blend_component = BlendComponent {
            src_factor: u32_to_blend_factor(key.blend_src),
            dst_factor: u32_to_blend_factor(key.blend_dst),
            operation: BlendOperation::Add,
        };

        let write_mask = {
            let mut m = ColorWrites::empty();
            if key.color_update {
                m |= ColorWrites::RED | ColorWrites::GREEN | ColorWrites::BLUE;
            }
            if key.alpha_update {
                m |= ColorWrites::ALPHA;
            }
            if m.is_empty() {
                ColorWrites::ALL
            } else {
                m
            }
        };

        let depth_compare = match key.z_func {
            0 => CompareFunction::Never,
            1 => CompareFunction::Less,
            2 => CompareFunction::Equal,
            3 => CompareFunction::LessEqual,
            4 => CompareFunction::Greater,
            5 => CompareFunction::NotEqual,
            6 => CompareFunction::GreaterEqual,
            7 => CompareFunction::Always,
            _ => CompareFunction::LessEqual,
        };

        let pipeline_layout = self
            .pipeline_layout
            .as_ref()
            .expect("Pipeline layout not initialized");

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("GX Render Pipeline"),
            layout: Some(pipeline_layout),
            vertex: VertexState {
                module: vertex_shader,
                entry_point: "main",
                buffers: &[VertexBufferLayout {
                    array_stride: (3 + 3 + 4 + 2) * 4, // pos + normal + color + texcoord
                    step_mode: VertexStepMode::Vertex,
                    attributes: &[
                        // Position
                        VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: VertexFormat::Float32x3,
                        },
                        // Normal
                        VertexAttribute {
                            offset: 12,
                            shader_location: 1,
                            format: VertexFormat::Float32x3,
                        },
                        // Color
                        VertexAttribute {
                            offset: 24,
                            shader_location: 2,
                            format: VertexFormat::Float32x4,
                        },
                        // TexCoord
                        VertexAttribute {
                            offset: 40,
                            shader_location: 3,
                            format: VertexFormat::Float32x2,
                        },
                    ],
                }],
            },
            fragment: Some(FragmentState {
                module: fragment_shader,
                entry_point: "main",
                targets: &[Some(ColorTargetState {
                    format: surface_format,
                    blend: Some(BlendState {
                        color: blend_component,
                        alpha: blend_component,
                    }),
                    write_mask,
                })],
            }),
            primitive: PrimitiveState {
                topology,
                strip_index_format: None,
                front_face: FrontFace::Cw, // GameCube uses CW winding
                cull_mode,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: if key.z_enable {
                Some(DepthStencilState {
                    format: TextureFormat::Depth24Plus,
                    depth_write_enabled: key.z_write,
                    depth_compare,
                    stencil: StencilState::default(),
                    bias: DepthBiasState::default(),
                })
            } else {
                None
            },
            multisample: MultisampleState::default(),
            multiview: None,
        })
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

impl Default for PipelineCache {
    fn default() -> Self {
        Self::new()
    }
}

fn u32_to_blend_factor(f: u32) -> BlendFactor {
    match f {
        0 => BlendFactor::Zero,
        1 => BlendFactor::One,
        2 => BlendFactor::Src,
        3 => BlendFactor::OneMinusSrc,
        4 => BlendFactor::SrcAlpha,
        5 => BlendFactor::OneMinusSrcAlpha,
        6 => BlendFactor::Dst,
        7 => BlendFactor::OneMinusDst,
        _ => BlendFactor::One,
    }
}
