// Shader management
use anyhow::Result;
use wgpu::*;

#[derive(Default)]
pub struct ShaderManager {
    shaders: std::collections::HashMap<String, ShaderModule>,
}

impl ShaderManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_shader(&mut self, device: &Device, name: &str, source: &str) -> Result<()> {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some(name),
            source: ShaderSource::Wgsl(source.into()),
        });

        self.shaders.insert(name.to_string(), shader);
        Ok(())
    }

    pub fn get_shader(&self, name: &str) -> Option<&ShaderModule> {
        self.shaders.get(name)
    }
}
