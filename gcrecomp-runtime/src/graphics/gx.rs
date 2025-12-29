//! GX (Graphics eXecutor) command processing
//!
//! This module processes GameCube GX commands. GX commands are 32-bit words:
//! - Bits 31-24: Command opcode (8 bits)
//! - Bits 23-0: Command data (24 bits)
//!
//! Commands are written to the GX FIFO and processed by the graphics hardware.

use anyhow::{Context, Result};
use glam::{Mat4, Vec3, Vec4};
use crate::graphics::gx_state::{GXRenderingState, transform_vertex};

/// GX command opcodes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GXCommand {
    // Vertex commands
    LoadBPReg = 0x61,  // Load BP (Blending/Processing) register
    LoadCPReg = 0x08,  // Load CP (Command Processor) register
    LoadXFReg = 0x10,  // Load XF (Transform) register
    LoadIndexedXF = 0x11, // Load indexed XF register
    
    // Draw commands
    DrawQuads = 0x80,  // Draw quads
    DrawTriangles = 0x90, // Draw triangles
    DrawTriangleStrip = 0x98, // Draw triangle strip
    DrawTriangleFan = 0xA0, // Draw triangle fan
    DrawLines = 0xA8, // Draw lines
    DrawLineStrip = 0xB0, // Draw line strip
    DrawPoints = 0xB8, // Draw points
    
    // Vertex format commands
    SetVtxDesc = 0x70, // Set vertex descriptor
    SetVtxAttrFmt = 0x20, // Set vertex attribute format
    SetArray = 0x28, // Set vertex array
    
    // Texture commands
    LoadTexObj = 0x30, // Load texture object
    LoadTlut = 0x31, // Load texture lookup table
    
    // State commands
    SetViewport = 0x40, // Set viewport
    SetScissor = 0x41, // Set scissor box
    SetProjection = 0x42, // Set projection matrix
    
    // Unknown/unsupported command
    Unknown = 0xFF,
}

impl GXCommand {
    /// Decode command from 32-bit word
    pub fn from_u32(word: u32) -> Self {
        let opcode = (word >> 24) as u8;
        match opcode {
            0x61 => Self::LoadBPReg,
            0x08 => Self::LoadCPReg,
            0x10 => Self::LoadXFReg,
            0x11 => Self::LoadIndexedXF,
            0x80 => Self::DrawQuads,
            0x90 => Self::DrawTriangles,
            0x98 => Self::DrawTriangleStrip,
            0xA0 => Self::DrawTriangleFan,
            0xA8 => Self::DrawLines,
            0xB0 => Self::DrawLineStrip,
            0xB8 => Self::DrawPoints,
            0x70 => Self::SetVtxDesc,
            0x20 => Self::SetVtxAttrFmt,
            0x28 => Self::SetArray,
            0x30 => Self::LoadTexObj,
            0x31 => Self::LoadTlut,
            0x40 => Self::SetViewport,
            0x41 => Self::SetScissor,
            0x42 => Self::SetProjection,
            _ => Self::Unknown,
        }
    }
}

/// Vertex array information
#[derive(Debug, Clone, Copy)]
pub struct VertexArray {
    /// Base address in memory
    pub base_address: u32,
    /// Stride in bytes between vertices
    pub stride: u32,
}

/// Texture object information
#[derive(Debug, Clone)]
pub struct TextureObject {
    /// Texture image address in memory
    pub image_addr: u32,
    /// Texture format
    pub format: u8,
    /// Texture width
    pub width: u16,
    /// Texture height
    pub height: u16,
    /// Wrap mode S
    pub wrap_s: u8,
    /// Wrap mode T
    pub wrap_t: u8,
    /// Min filter
    pub min_filter: u8,
    /// Mag filter
    pub mag_filter: u8,
    /// Texture lookup table address (for paletted formats)
    pub tlut_addr: u32,
}

/// GX processor state
#[derive(Debug)]
pub struct GXProcessor {
    /// Current vertex descriptor
    vtx_desc: u32,
    /// Current vertex attribute format
    vtx_attr_fmt: [u32; 8],
    /// Current viewport settings
    viewport: Viewport,
    /// Current projection matrix
    projection: [f32; 16],
    /// Pending commands queue
    command_queue: Vec<GXCommandData>,
    /// Vertex data buffer
    vertex_buffer: Vec<Vertex>,
    /// Vertex arrays (indexed by attribute: 0=position, 1=normal, 2=color0, 3=color1, 4=tex0-7)
    vertex_arrays: [Option<VertexArray>; 13],
    /// Memory access callback (for reading vertex data)
    memory_reader: Option<Box<dyn Fn(u32, usize) -> Result<Vec<u8>> + Send + Sync>>,
    /// Current texture objects (up to 8 texture units)
    texture_objects: [Option<TextureObject>; 8],
    /// Texture loader callback (for creating wgpu textures)
    texture_loader: Option<Box<dyn Fn(&TextureObject, &[u8]) -> Result<()> + Send + Sync>>,
    /// Rendering state (blending, fog, alpha test, etc.)
    rendering_state: GXRenderingState,
    /// Model-view matrix stack
    model_view_stack: Vec<Mat4>,
}

/// Viewport settings
#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub near: f32,
    pub far: f32,
}

/// Vertex data
#[derive(Debug, Clone)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
    pub tex_coord: [f32; 2],
}

/// GX command with data
#[derive(Debug, Clone)]
pub struct GXCommandData {
    pub command: GXCommand,
    pub data: u32,
    pub args: Vec<u32>,
}

impl GXProcessor {
    /// Create a new GX processor
    pub fn new() -> Self {
        Self {
            vtx_desc: 0,
            vtx_attr_fmt: [0; 8],
            viewport: Viewport {
                x: 0.0,
                y: 0.0,
                width: 640.0,
                height: 480.0,
                near: 0.0,
                far: 1.0,
            },
            projection: [
                1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.0, 0.0, 1.0,
            ],
            command_queue: Vec::new(),
            vertex_buffer: Vec::new(),
            vertex_arrays: [None; 13],
            memory_reader: None,
            texture_objects: [None; 8],
            texture_loader: None,
            rendering_state: GXRenderingState::new(),
            model_view_stack: vec![Mat4::IDENTITY],
        }
    }

    /// Set memory reader callback for accessing vertex data
    pub fn set_memory_reader<F>(&mut self, reader: F)
    where
        F: Fn(u32, usize) -> Result<Vec<u8>> + Send + Sync + 'static,
    {
        self.memory_reader = Some(Box::new(reader));
    }

    /// Set texture loader callback for creating wgpu textures
    pub fn set_texture_loader<F>(&mut self, loader: F)
    where
        F: Fn(&TextureObject, &[u8]) -> Result<()> + Send + Sync + 'static,
    {
        self.texture_loader = Some(Box::new(loader));
    }

    /// Process a GX command
    ///
    /// # Arguments
    /// * `command` - 32-bit command word
    /// * `args` - Additional command arguments (for multi-word commands)
    pub fn process_command(&mut self, command: u32, args: &[u32]) -> Result<()> {
        let cmd = GXCommand::from_u32(command);
        let data = command & 0x00FFFFFF; // Bottom 24 bits

        match cmd {
            GXCommand::LoadBPReg => {
                self.process_load_bp_reg(data)?;
            }
            GXCommand::LoadCPReg => {
                self.process_load_cp_reg(data)?;
            }
            GXCommand::LoadXFReg => {
                self.process_load_xf_reg(data, args)?;
            }
            GXCommand::LoadIndexedXF => {
                self.process_load_indexed_xf(data, args)?;
            }
            GXCommand::SetVtxDesc => {
                self.process_set_vtx_desc(data)?;
            }
            GXCommand::SetVtxAttrFmt => {
                self.process_set_vtx_attr_fmt(data, args)?;
            }
            GXCommand::SetArray => {
                self.process_set_array(data, args)?;
            }
            GXCommand::SetViewport => {
                self.process_set_viewport(data, args)?;
            }
            GXCommand::SetScissor => {
                self.process_set_scissor(data, args)?;
            }
            GXCommand::SetProjection => {
                self.process_set_projection(data, args)?;
            }
            GXCommand::LoadTexObj => {
                self.process_load_tex_obj(data, args)?;
            }
            GXCommand::DrawQuads
            | GXCommand::DrawTriangles
            | GXCommand::DrawTriangleStrip
            | GXCommand::DrawTriangleFan
            | GXCommand::DrawLines
            | GXCommand::DrawLineStrip
            | GXCommand::DrawPoints => {
                self.process_draw_command(cmd, data)?;
            }
            GXCommand::Unknown => {
                log::warn!("Unknown GX command: 0x{:08X}", command);
            }
        }

        // Queue command for batch processing
        self.command_queue.push(GXCommandData {
            command: cmd,
            data,
            args: args.to_vec(),
        });

        Ok(())
    }

    /// Process LoadBPReg command
    fn process_load_bp_reg(&mut self, data: u32) -> Result<()> {
        let reg = (data >> 16) & 0xFF;
        let value = data & 0xFFFF;
        log::debug!("GX LoadBPReg: reg=0x{:02X}, value=0x{:04X}", reg, value);
        
        // BP registers control blending, fog, alpha test, etc.
        // This is a simplified implementation - full BP register handling would decode all registers
        match reg {
            0x00..=0x0F => {
                // Blending control registers
                let blend_mode = (value >> 8) & 0x7;
                self.rendering_state.blending = match blend_mode {
                    0 => crate::graphics::gx_state::BlendingMode::None,
                    1 => crate::graphics::gx_state::BlendingMode::Alpha,
                    2 => crate::graphics::gx_state::BlendingMode::Additive,
                    3 => crate::graphics::gx_state::BlendingMode::Subtractive,
                    4 => crate::graphics::gx_state::BlendingMode::Multiply,
                    _ => crate::graphics::gx_state::BlendingMode::None,
                };
            }
            0x10..=0x1F => {
                // Alpha test registers
                let func = (value >> 8) & 0x7;
                self.rendering_state.alpha_test_enabled = (value & 0x1) != 0;
                self.rendering_state.alpha_test_func = match func {
                    0 => crate::graphics::gx_state::AlphaTestFunc::Never,
                    1 => crate::graphics::gx_state::AlphaTestFunc::Less,
                    2 => crate::graphics::gx_state::AlphaTestFunc::Equal,
                    3 => crate::graphics::gx_state::AlphaTestFunc::LessEqual,
                    4 => crate::graphics::gx_state::AlphaTestFunc::Greater,
                    5 => crate::graphics::gx_state::AlphaTestFunc::NotEqual,
                    6 => crate::graphics::gx_state::AlphaTestFunc::GreaterEqual,
                    7 => crate::graphics::gx_state::AlphaTestFunc::Always,
                    _ => crate::graphics::gx_state::AlphaTestFunc::Always,
                };
                self.rendering_state.alpha_test_ref = ((value >> 4) & 0xFF) as f32 / 255.0;
            }
            0x20..=0x2F => {
                // Fog registers
                self.rendering_state.fog_enabled = (value & 0x1) != 0;
                let fog_mode = (value >> 8) & 0x3;
                self.rendering_state.fog_mode = match fog_mode {
                    0 => crate::graphics::gx_state::FogMode::None,
                    1 => crate::graphics::gx_state::FogMode::Linear,
                    2 => crate::graphics::gx_state::FogMode::Exponential,
                    3 => crate::graphics::gx_state::FogMode::Exponential2,
                    _ => crate::graphics::gx_state::FogMode::None,
                };
            }
            _ => {
                // Other BP registers - not fully implemented yet
            }
        }
        
        Ok(())
    }

    /// Process LoadCPReg command
    fn process_load_cp_reg(&mut self, data: u32) -> Result<()> {
        let reg = (data >> 16) & 0xFF;
        let value = data & 0xFFFF;
        log::debug!("GX LoadCPReg: reg=0x{:02X}, value=0x{:04X}", reg, value);
        // CP registers control command processor state
        Ok(())
    }

    /// Process LoadXFReg command
    fn process_load_xf_reg(&mut self, data: u32, args: &[u32]) -> Result<()> {
        let reg = (data >> 16) & 0xFFF;
        let count = ((data & 0xFFFF) >> 8) + 1;
        log::debug!("GX LoadXFReg: reg=0x{:03X}, count={}", reg, count);
        
        // XF registers are transform/lighting registers
        // Args contain the register data
        if args.len() < count as usize {
            anyhow::bail!("Not enough arguments for LoadXFReg");
        }
        
        // Handle matrix loading (simplified - full implementation would handle all XF registers)
        match reg {
            0x1008..=0x1017 => {
                // Projection matrix (4x4, 16 words)
                if args.len() >= 16 {
                    for i in 0..16 {
                        self.projection[i] = f32::from_bits(args[i]);
                    }
                }
            }
             0x1018..=0x1027 => {
                // Model-view matrix (4x4, 16 words)
                if args.len() >= 16 {
                    let mut matrix_data = [0.0f32; 16];
                    for i in 0..16 {
                        matrix_data[i] = f32::from_bits(args[i]);
                    }
                    let matrix = Mat4::from_cols_array(&matrix_data);
                    if let Some(last) = self.model_view_stack.last_mut() {
                        *last = matrix;
                        self.rendering_state.model_view = *last;
                    }
                }
            }
            _ => {
                // Other XF registers - not fully implemented yet
            }
        }
        
        Ok(())
    }

    /// Process LoadIndexedXF command
    fn process_load_indexed_xf(&mut self, data: u32, args: &[u32]) -> Result<()> {
        let base_reg = (data >> 16) & 0xFFF;
        let count = ((data & 0xFFFF) >> 8) + 1;
        log::debug!("GX LoadIndexedXF: base=0x{:03X}, count={}", base_reg, count);
        Ok(())
    }

    /// Process SetVtxDesc command
    fn process_set_vtx_desc(&mut self, data: u32) -> Result<()> {
        self.vtx_desc = data;
        log::debug!("GX SetVtxDesc: 0x{:08X}", data);
        Ok(())
    }

    /// Process SetVtxAttrFmt command
    fn process_set_vtx_attr_fmt(&mut self, data: u32, args: &[u32]) -> Result<()> {
        let attr = (data >> 16) & 0x7;
        if attr < 8 {
            self.vtx_attr_fmt[attr as usize] = data;
        }
        log::debug!("GX SetVtxAttrFmt: attr={}, data=0x{:08X}", attr, data);
        Ok(())
    }

    /// Process SetArray command
    /// 
    /// Sets vertex array base address and stride for a specific attribute.
    /// Format: attr (3 bits) | stride (13 bits) in data, base address in args[0]
    fn process_set_array(&mut self, data: u32, args: &[u32]) -> Result<()> {
        let attr = (data >> 16) & 0x7;
        let stride = data & 0x1FFF; // 13-bit stride
        
        if args.is_empty() {
            anyhow::bail!("SetArray requires base address argument");
        }
        
        let base_address = args[0];
        
        // Map attribute to array index:
        // 0 = Position (GX_VA_POS)
        // 1 = Normal (GX_VA_NRM)
        // 2 = Color0 (GX_VA_CLR0)
        // 3 = Color1 (GX_VA_CLR1)
        // 4-11 = Tex0-7 (GX_VA_TEX0-7)
        let array_idx = if attr <= 3 {
            attr as usize
        } else if attr >= 8 && attr <= 15 {
            // Texture coordinates: 8-15 map to indices 4-11
            (attr - 4) as usize
        } else {
            log::warn!("SetArray: invalid attribute index {}", attr);
            return Ok(());
        };
        
        if array_idx < self.vertex_arrays.len() {
            self.vertex_arrays[array_idx] = Some(VertexArray {
                base_address,
                stride: if stride == 0 { 1 } else { stride }, // Stride 0 means 1 byte
            });
            log::debug!("GX SetArray: attr={}, base=0x{:08X}, stride={}", attr, base_address, stride);
        }
        
        Ok(())
    }

    /// Process SetViewport command
    fn process_set_viewport(&mut self, data: u32, args: &[u32]) -> Result<()> {
        // Viewport can be set via command data or args
        if args.len() >= 6 {
            // Viewport data is in args (as f32 bits)
            self.viewport.x = f32::from_bits(args[0]);
            self.viewport.y = f32::from_bits(args[1]);
            self.viewport.width = f32::from_bits(args[2]);
            self.viewport.height = f32::from_bits(args[3]);
            self.viewport.near = f32::from_bits(args[4]);
            self.viewport.far = f32::from_bits(args[5]);
        } else {
            // Viewport data might be in command word itself
            // Extract from data field (simplified)
            log::debug!("GX SetViewport: data=0x{:08X}, args={:?}", data, args);
        }
        log::debug!("GX SetViewport: {:?}", self.viewport);
        Ok(())
    }

    /// Process SetScissor command
    fn process_set_scissor(&mut self, data: u32, args: &[u32]) -> Result<()> {
        log::debug!("GX SetScissor: 0x{:08X}", data);
        
        // Scissor box: x, y, width, height
        if args.len() >= 4 {
            let x = f32::from_bits(args[0]);
            let y = f32::from_bits(args[1]);
            let width = f32::from_bits(args[2]);
            let height = f32::from_bits(args[3]);
            self.rendering_state.scissor = Some((x, y, width, height));
        } else {
            // Extract from data if args not available
            let x = ((data >> 12) & 0xFFF) as f32;
            let y = (data & 0xFFF) as f32;
            // Width and height would need additional data
            self.rendering_state.scissor = Some((x, y, self.viewport.width, self.viewport.height));
        }
        
        Ok(())
    }

    /// Process SetProjection command
    fn process_set_projection(&mut self, data: u32, args: &[u32]) -> Result<()> {
        if args.len() >= 16 {
            for i in 0..16 {
                self.projection[i] = f32::from_bits(args[i]);
            }
            log::debug!("GX SetProjection: matrix set");
        }
        Ok(())
    }

    /// Process LoadTexObj command
    /// 
    /// Loads a texture object from memory. The texture object structure is:
    /// - Word 0: Image address (24 bits) | Format (8 bits)
    /// - Word 1: Width (12 bits) | Height (12 bits) | Wrap S (4 bits) | Wrap T (4 bits)
    /// - Word 2: Min filter (8 bits) | Mag filter (8 bits) | Reserved (16 bits)
    /// - Word 3: TLUT address (24 bits) | Reserved (8 bits)
    fn process_load_tex_obj(&mut self, data: u32, args: &[u32]) -> Result<()> {
        // Texture object address is in the command data (bottom 24 bits)
        let tex_obj_addr = data & 0x00FFFFFF;
        
        // Read texture object from memory (4 words = 16 bytes)
        let reader = self.memory_reader.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Memory reader not set"))?;
        let tex_obj_data = reader(tex_obj_addr, 16)?;
        
        if tex_obj_data.len() < 16 {
            anyhow::bail!("Texture object data too short: {} bytes", tex_obj_data.len());
        }
        
        // Parse texture object structure (big-endian)
        let word0 = u32::from_be_bytes([tex_obj_data[0], tex_obj_data[1], tex_obj_data[2], tex_obj_data[3]]);
        let word1 = u32::from_be_bytes([tex_obj_data[4], tex_obj_data[5], tex_obj_data[6], tex_obj_data[7]]);
        let word2 = u32::from_be_bytes([tex_obj_data[8], tex_obj_data[9], tex_obj_data[10], tex_obj_data[11]]);
        let word3 = u32::from_be_bytes([tex_obj_data[12], tex_obj_data[13], tex_obj_data[14], tex_obj_data[15]]);
        
        // Extract fields
        let image_addr = word0 & 0x00FFFFFF;
        let format = ((word0 >> 24) & 0xFF) as u8;
        let width = ((word1 >> 16) & 0xFFF) as u16;
        let height = (word1 & 0xFFF) as u16;
        let wrap_s = ((word1 >> 12) & 0xF) as u8;
        let wrap_t = ((word1 >> 8) & 0xF) as u8;
        let min_filter = ((word2 >> 24) & 0xFF) as u8;
        let mag_filter = ((word2 >> 16) & 0xFF) as u8;
        let tlut_addr = word3 & 0x00FFFFFF;
        
        let tex_obj = TextureObject {
            image_addr,
            format,
            width: width.max(1),
            height: height.max(1),
            wrap_s,
            wrap_t,
            min_filter,
            mag_filter,
            tlut_addr,
        };
        
        // Store in texture unit 0 (for now - could support multiple units)
        self.texture_objects[0] = Some(tex_obj.clone());
        
        // Load texture data from memory
        let tex_size = self.calculate_texture_size(format, width, height)?;
        let tex_data = reader(image_addr, tex_size)?;
        
        // Use texture loader callback to create wgpu texture
        if let Some(ref loader) = self.texture_loader {
            loader(&tex_obj, &tex_data)?;
        }
        
        log::debug!("GX LoadTexObj: addr=0x{:08X}, format=0x{:02X}, {}x{}", 
                   image_addr, format, width, height);
        
        Ok(())
    }

    /// Calculate texture size in bytes based on format and dimensions
    fn calculate_texture_size(&self, format: u8, width: u16, height: u16) -> Result<usize> {
        use crate::texture::formats::GameCubeTextureFormat;
        
        let gc_format = GameCubeTextureFormat::from_gx_format(format)
            .ok_or_else(|| anyhow::anyhow!("Unknown texture format: 0x{:02X}", format))?;
        
        let bytes_per_pixel = match gc_format {
            GameCubeTextureFormat::CMPR => 0, // Compressed - 8 bytes per 4x4 block
            GameCubeTextureFormat::I4 => 1,
            GameCubeTextureFormat::I8 => 1,
            GameCubeTextureFormat::IA4 => 1,
            GameCubeTextureFormat::IA8 => 2,
            GameCubeTextureFormat::RGB565 => 2,
            GameCubeTextureFormat::RGB5A3 => 2,
            GameCubeTextureFormat::RGBA8 => 4,
        };
        
        if gc_format == GameCubeTextureFormat::CMPR {
            // CMPR: 8 bytes per 4x4 block
            let blocks_w = (width as usize + 3) / 4;
            let blocks_h = (height as usize + 3) / 4;
            Ok(blocks_w * blocks_h * 8)
        } else {
            Ok((width as usize * height as usize * bytes_per_pixel as usize))
        }
    }

    /// Get current texture object
    pub fn get_texture_object(&self, unit: usize) -> Option<&TextureObject> {
        if unit < self.texture_objects.len() {
            self.texture_objects[unit].as_ref()
        } else {
            None
        }
    }

    /// Process draw command
    /// 
    /// Parses vertices from vertex arrays and builds draw call data
    fn process_draw_command(&mut self, cmd: GXCommand, vertex_count: u32) -> Result<()> {
        let count = vertex_count & 0xFFFF;
        log::debug!("GX {:?}: {} vertices", cmd, count);
        
        // Fetch vertices from memory based on current vertex format
        let vertices = self.fetch_vertices(0, count)?;
        
        // Apply viewport and projection transformations
        let projection_mat = Mat4::from_cols_array(&self.projection);
        let viewport = (
            self.viewport.x,
            self.viewport.y,
            self.viewport.width,
            self.viewport.height,
        );
        
        let transformed_vertices: Vec<Vertex> = vertices
            .into_iter()
            .map(|mut v| {
                // Transform vertex position
                let pos = Vec3::new(v.position[0], v.position[1], v.position[2]);
                let (transformed_pos, w) = transform_vertex(
                    pos,
                    self.rendering_state.model_view,
                    projection_mat,
                    viewport,
                );
                
                v.position[0] = transformed_pos.x;
                v.position[1] = transformed_pos.y;
                v.position[2] = transformed_pos.z;
                
                // Apply fog if enabled
                if self.rendering_state.fog_enabled {
                    let distance = w;
                    let color = Vec4::new(
                        v.color[0],
                        v.color[1],
                        v.color[2],
                        v.color[3],
                    );
                    let fogged = self.rendering_state.apply_fog(color, distance);
                    v.color[0] = fogged.x;
                    v.color[1] = fogged.y;
                    v.color[2] = fogged.z;
                    v.color[3] = fogged.w;
                }
                
                v
            })
            .collect();
        
        // Store vertices in vertex buffer for rendering
        self.vertex_buffer.extend(transformed_vertices);
        
        Ok(())
    }

    /// Get pending draw calls (vertices ready for rendering)
    pub fn get_pending_vertices(&self) -> &[Vertex] {
        &self.vertex_buffer
    }

    /// Clear pending vertices (called after rendering)
    pub fn clear_pending_vertices(&mut self) {
        self.vertex_buffer.clear();
    }

    /// Flush command queue (process all queued commands)
    pub fn flush_commands(&mut self) -> Result<()> {
        // Commands are processed immediately, but this can be used for batch processing
        self.command_queue.clear();
        Ok(())
    }

    /// Get current viewport
    pub fn viewport(&self) -> Viewport {
        self.viewport
    }

    /// Get current projection matrix
    pub fn projection(&self) -> &[f32; 16] {
        &self.projection
    }

    /// Clear vertex buffer
    pub fn clear_vertex_buffer(&mut self) {
        self.vertex_buffer.clear();
    }

    /// Fetch vertices from memory based on current vertex format
    /// 
    /// # Arguments
    /// * `start_index` - Starting vertex index
    /// * `count` - Number of vertices to fetch
    /// 
    /// # Returns
    /// Vector of parsed vertices
    pub fn fetch_vertices(&self, start_index: u32, count: u32) -> Result<Vec<Vertex>> {
        let mut vertices = Vec::with_capacity(count as usize);
        
        // Check if memory reader is available
        let reader = self.memory_reader.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Memory reader not set"))?;
        
        // Parse vertex descriptor to determine which attributes are enabled
        // vtx_desc bits: each 2-bit field indicates attribute type (0=not present, 1=direct, 2=indexed, 3=reserved)
        let pos_enabled = (self.vtx_desc & 0x00000003) != 0;
        let nrm_enabled = (self.vtx_desc & 0x0000000C) != 0;
        let clr0_enabled = (self.vtx_desc & 0x00000030) != 0;
        let clr1_enabled = (self.vtx_desc & 0x000000C0) != 0;
        let tex0_enabled = (self.vtx_desc & 0x00000300) != 0;
        
        for i in 0..count {
            let vertex_idx = start_index + i;
            let mut vertex = Vertex {
                position: [0.0, 0.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
                tex_coord: [0.0, 0.0],
            };
            
            // Fetch position
            if pos_enabled {
                if let Some(array) = self.vertex_arrays[0] {
                    let offset = array.base_address.wrapping_add(vertex_idx.wrapping_mul(array.stride));
                    let data = reader(offset, 12)?; // 3 floats = 12 bytes
                    if data.len() >= 12 {
                        vertex.position[0] = f32::from_bits(u32::from_be_bytes([data[0], data[1], data[2], data[3]]));
                        vertex.position[1] = f32::from_bits(u32::from_be_bytes([data[4], data[5], data[6], data[7]]));
                        vertex.position[2] = f32::from_bits(u32::from_be_bytes([data[8], data[9], data[10], data[11]]));
                    }
                }
            }
            
            // Fetch normal
            if nrm_enabled {
                if let Some(array) = self.vertex_arrays[1] {
                    let offset = array.base_address.wrapping_add(vertex_idx.wrapping_mul(array.stride));
                    let data = reader(offset, 12)?;
                    if data.len() >= 12 {
                        vertex.normal[0] = f32::from_bits(u32::from_be_bytes([data[0], data[1], data[2], data[3]]));
                        vertex.normal[1] = f32::from_bits(u32::from_be_bytes([data[4], data[5], data[6], data[7]]));
                        vertex.normal[2] = f32::from_bits(u32::from_be_bytes([data[8], data[9], data[10], data[11]]));
                    }
                }
            }
            
            // Fetch color0
            if clr0_enabled {
                if let Some(array) = self.vertex_arrays[2] {
                    let offset = array.base_address.wrapping_add(vertex_idx.wrapping_mul(array.stride));
                    let data = reader(offset, 4)?; // RGBA = 4 bytes
                    if data.len() >= 4 {
                        vertex.color[0] = data[0] as f32 / 255.0;
                        vertex.color[1] = data[1] as f32 / 255.0;
                        vertex.color[2] = data[2] as f32 / 255.0;
                        vertex.color[3] = data[3] as f32 / 255.0;
                    }
                }
            }
            
            // Fetch texcoord0
            if tex0_enabled {
                if let Some(array) = self.vertex_arrays[4] {
                    let offset = array.base_address.wrapping_add(vertex_idx.wrapping_mul(array.stride));
                    let data = reader(offset, 8)?; // 2 floats = 8 bytes
                    if data.len() >= 8 {
                        vertex.tex_coord[0] = f32::from_bits(u32::from_be_bytes([data[0], data[1], data[2], data[3]]));
                        vertex.tex_coord[1] = f32::from_bits(u32::from_be_bytes([data[4], data[5], data[6], data[7]]));
                    }
                }
            }
            
            vertices.push(vertex);
        }
        
        Ok(vertices)
    }

    /// Get vertex arrays (for renderer access)
    pub fn vertex_arrays(&self) -> &[Option<VertexArray>; 13] {
        &self.vertex_arrays
    }

    /// Get vertex descriptor
    pub fn vertex_descriptor(&self) -> u32 {
        self.vtx_desc
    }

    /// Get vertex attribute format
    pub fn vertex_attribute_format(&self, attr: usize) -> Option<u32> {
        if attr < 8 {
            Some(self.vtx_attr_fmt[attr])
        } else {
            None
        }
    }

    /// Get rendering state
    pub fn rendering_state(&self) -> &GXRenderingState {
        &self.rendering_state
    }

    /// Get mutable rendering state
    pub fn rendering_state_mut(&mut self) -> &mut GXRenderingState {
        &mut self.rendering_state
    }
}

impl Default for GXProcessor {
    fn default() -> Self {
        Self::new()
    }
}
