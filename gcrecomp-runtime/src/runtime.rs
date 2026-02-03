// Complete runtime system integration
use crate::graphics::Renderer;
use crate::input::ControllerManager;
use crate::memory::{ARam, DmaSystem, Ram, VRam};
use crate::texture::TextureLoader;
use anyhow::Result;
use std::sync::Arc;

pub struct Runtime {
    controller_manager: ControllerManager,
    renderer: Option<Renderer>,
    texture_loader: TextureLoader,
    ram: Ram,
    vram: VRam,
    aram: ARam,
    dma: DmaSystem,
}

impl Runtime {
    pub fn new() -> Result<Self> {
        Ok(Self {
            controller_manager: ControllerManager::new()?,
            renderer: None,
            texture_loader: TextureLoader::new(),
            ram: Ram::new(),
            vram: VRam::new(),
            aram: ARam::new(),
            dma: DmaSystem::new(),
        })
    }

    pub fn initialize_graphics(&mut self, window: Arc<winit::window::Window>) -> Result<()> {
        self.renderer = Some(Renderer::new(window)?);
        Ok(())
    }

    pub fn update(&mut self) -> Result<()> {
        // Update controller manager
        self.controller_manager.update()?;

        // Update DMA transfers
        // Process any active DMA transfers

        Ok(())
    }

    pub fn get_controller_input(
        &self,
        controller_id: usize,
    ) -> Option<crate::input::controller::GameCubeInput> {
        self.controller_manager.get_gamecube_input(controller_id)
    }

    pub fn ram_mut(&mut self) -> &mut Ram {
        &mut self.ram
    }

    pub fn ram(&self) -> &Ram {
        &self.ram
    }

    pub fn vram_mut(&mut self) -> &mut VRam {
        &mut self.vram
    }

    pub fn vram(&self) -> &VRam {
        &self.vram
    }

    pub fn aram_mut(&mut self) -> &mut ARam {
        &mut self.aram
    }

    pub fn aram(&self) -> &ARam {
        &self.aram
    }

    pub fn dma_mut(&mut self) -> &mut DmaSystem {
        &mut self.dma
    }

    pub fn renderer_mut(&mut self) -> Option<&mut Renderer> {
        self.renderer.as_mut()
    }

    pub fn texture_loader_mut(&mut self) -> &mut TextureLoader {
        &mut self.texture_loader
    }
}
