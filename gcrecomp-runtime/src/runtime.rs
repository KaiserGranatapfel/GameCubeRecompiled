// Complete runtime system integration
use crate::audio::ai::AudioInterface;
use crate::audio::mixer::AudioMixer;
use crate::audio::output::AudioOutput;
use crate::graphics::Renderer;
use crate::input::ControllerManager;
use crate::memory::{ARam, DmaSystem, Ram, VRam};
use crate::texture::TextureLoader;
use crate::video::VideoInterface;
use anyhow::Result;
use std::sync::{Arc, Mutex};

pub struct Runtime {
    controller_manager: ControllerManager,
    renderer: Option<Renderer>,
    texture_loader: TextureLoader,
    ram: Ram,
    vram: VRam,
    aram: ARam,
    dma: DmaSystem,
    video: VideoInterface,
    audio: AudioInterface,
    audio_mixer: Arc<Mutex<AudioMixer>>,
    audio_output: AudioOutput,
}

impl Runtime {
    pub fn new() -> Result<Self> {
        let audio_mixer = Arc::new(Mutex::new(AudioMixer::new(48000)));
        let audio_output = AudioOutput::new(audio_mixer.clone());

        Ok(Self {
            controller_manager: ControllerManager::new()?,
            renderer: None,
            texture_loader: TextureLoader::new(),
            ram: Ram::new(),
            vram: VRam::new(),
            aram: ARam::new(),
            dma: DmaSystem::new(),
            video: VideoInterface::new(),
            audio: AudioInterface::new(),
            audio_mixer,
            audio_output,
        })
    }

    pub fn initialize_graphics(&mut self, window: Arc<winit::window::Window>) -> Result<()> {
        self.renderer = Some(Renderer::new(window)?);
        Ok(())
    }

    pub fn initialize_audio(&mut self) -> Result<()> {
        self.audio.init();
        self.audio_output.start()?;
        Ok(())
    }

    pub fn update(&mut self) -> Result<()> {
        // Update controller manager
        self.controller_manager.update()?;

        // Process any active DMA transfers
        for ch in 0..4 {
            if self.dma.is_active(ch) {
                // Execute transfer would happen here with RAM/ARAM access
                self.dma.complete_transfer(ch);
            }
        }

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

    pub fn video(&self) -> &VideoInterface {
        &self.video
    }

    pub fn video_mut(&mut self) -> &mut VideoInterface {
        &mut self.video
    }

    pub fn audio(&self) -> &AudioInterface {
        &self.audio
    }

    pub fn audio_mut(&mut self) -> &mut AudioInterface {
        &mut self.audio
    }

    pub fn audio_mixer(&self) -> &Arc<Mutex<AudioMixer>> {
        &self.audio_mixer
    }
}
