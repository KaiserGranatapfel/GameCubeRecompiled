// Frame buffer management
use anyhow::Result;
use wgpu::*;

pub struct FrameBuffer {
    texture: Texture,
    view: TextureView,
    width: u32,
    height: u32,
}

impl FrameBuffer {
    pub fn new(device: &Device, width: u32, height: u32, format: TextureFormat) -> Result<Self> {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("FrameBuffer"),
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
        
        Ok(Self {
            texture,
            view,
            width,
            height,
        })
    }
    
    pub fn view(&self) -> &TextureView {
        &self.view
    }
    
    pub fn texture(&self) -> &Texture {
        &self.texture
    }
    
    pub fn width(&self) -> u32 {
        self.width
    }
    
    pub fn height(&self) -> u32 {
        self.height
    }
}

