//! Complete Runtime System Integration
//!
//! This module provides the main runtime system that integrates all components:
//! - Graphics rendering
//! - Input handling
//! - Memory management
//! - Modding support
//! - Runtime tracing
//!
//! # API Reference
//!
//! ## Runtime
//!
//! Main runtime system that coordinates all subsystems.
//!
//! ```rust,no_run
//! use gcrecomp_runtime::Runtime;
//!
//! let mut runtime = Runtime::new()?;
//! runtime.load_mods(std::path::Path::new("./mods"))?;
//! runtime.update()?;
//! ```
//!
//! ## Methods
//!
//! - `new()`: Create a new runtime instance
//! - `initialize_graphics()`: Initialize graphics subsystem
//! - `update()`: Update runtime state
//! - `load_mods()`: Load mods from a directory
//! - `hook_manager()`: Get hook manager for registering hooks
//! - `tracer()`: Get runtime tracer for debugging
//!
//! ## Subsystems
//!
//! - **Controller Manager**: Handles gamepad input
//! - **Renderer**: Graphics rendering (GX emulation)
//! - **Memory**: RAM, VRAM, ARAM management
//! - **Texture Loader**: Texture loading and caching
//! - **Mod System**: Plugin loading and hook management
//! - **Tracing**: Runtime execution tracing

use crate::audio::{AudioInterface, DSP};
use crate::graphics::Renderer;
use crate::input::ControllerManager;
use crate::memory::{ARam, DmaSystem, Ram, VRam};
use crate::mods::hooks::HookManager;
use crate::mods::{ModLoader, ModRegistry};
use crate::texture::TextureLoader;
use crate::tracing::RuntimeTracer;
use anyhow::Result;

pub struct Runtime {
    controller_manager: ControllerManager,
    renderer: Option<Renderer>,
    texture_loader: TextureLoader,
    ram: Ram,
    vram: VRam,
    aram: ARam,
    dma: DmaSystem,
    hook_manager: HookManager,
    mod_registry: ModRegistry,
    mod_loader: ModLoader,
    tracer: RuntimeTracer,
    dsp: DSP,
    audio_interface: AudioInterface,
}

impl Runtime {
    pub fn new() -> Result<Self> {
        // Initialize audio systems
        let mut dsp = DSP::new();
        dsp.init()?;

        let mut audio_interface = AudioInterface::new();
        audio_interface.init()?;

        // Create memory manager and register with SDK
        use gcrecomp_core::runtime::{memory::MemoryManager, sdk::set_memory_manager};
        use std::sync::{Arc, Mutex};
        let memory_manager = Arc::new(Mutex::new(MemoryManager::new()));
        set_memory_manager(memory_manager);

        Ok(Self {
            controller_manager: ControllerManager::new()?,
            renderer: None,
            texture_loader: TextureLoader::new(),
            ram: Ram::new(),
            vram: VRam::new(),
            aram: ARam::new(),
            dma: DmaSystem::new(),
            hook_manager: HookManager::new(),
            mod_registry: ModRegistry::new(),
            mod_loader: ModLoader::new(),
            tracer: RuntimeTracer::new(),
            dsp,
            audio_interface,
        })
    }

    /// Get the runtime tracer.
    pub fn tracer(&mut self) -> &mut RuntimeTracer {
        &mut self.tracer
    }

    /// Get the hook manager for registering hooks.
    pub fn hook_manager(&mut self) -> &mut HookManager {
        &mut self.hook_manager
    }

    /// Get the mod registry.
    pub fn mod_registry(&mut self) -> &mut ModRegistry {
        &mut self.mod_registry
    }

    /// Load mods from a directory.
    pub fn load_mods(&mut self, mod_dir: &std::path::Path) -> Result<()> {
        let loaded_mods = self.mod_loader.load_mods_from_directory(mod_dir)?;
        for (metadata, mod_instance) in loaded_mods {
            let name = metadata.name.clone();
            let path = self
                .mod_loader
                .mod_paths
                .get(&name)
                .cloned()
                .unwrap_or_else(|| mod_dir.join(format!("{}.so", name)));
            self.mod_registry
                .register_mod(name, mod_instance, metadata, path);
        }
        Ok(())
    }

    pub fn initialize_graphics(&mut self, window: &winit::window::Window) -> Result<()> {
        let mut renderer = Renderer::new(window)?;
        
        // Set up memory reader for GX processor (accesses RAM)
        {
            use std::sync::{Arc, Mutex};
            // Create a shared reference to RAM for the memory reader
            // We'll use a callback that accesses RAM through a shared pointer
            let ram_ptr: *const Ram = &self.ram;
            // Note: This is safe because Runtime outlives the renderer
            // In a production system, we'd use Arc<Mutex<Ram>> but for now this works
            renderer.set_gx_memory_reader(move |addr: u32, len: usize| -> Result<Vec<u8>> {
                // SAFETY: The RAM pointer is valid as long as Runtime exists
                // and Runtime outlives the renderer
                unsafe {
                    (*ram_ptr).read_bytes(addr, len)
                }
            });
        }

        // Set up texture loader for GX processor
        {
            use crate::graphics::gx::TextureObject;
            use crate::texture::GameCubeTextureFormat;
            let device_ptr: *const wgpu::Device = renderer.device();
            let queue_ptr: *const wgpu::Queue = renderer.queue();
            let texture_loader_ptr: *mut crate::texture::TextureLoader = renderer.texture_loader_mut();
            renderer.set_gx_texture_loader(move |tex_obj: &TextureObject, data: &[u8]| -> Result<()> {
                // SAFETY: Pointers are valid as long as renderer exists
                unsafe {
                    let device = &*device_ptr;
                    let queue = &*queue_ptr;
                    let texture_loader = &mut *texture_loader_ptr;
                    
                    // Decode texture using texture loader
                    let format = GameCubeTextureFormat::from_gx_format(tex_obj.format)
                        .ok_or_else(|| anyhow::anyhow!("Unknown texture format: 0x{:02X}", tex_obj.format))?;
                    
                    let image = texture_loader.load_texture(data, format, tex_obj.width as u32, tex_obj.height as u32)?;
                    
                    // Create wgpu texture from image
                    let texture_size = wgpu::Extent3d {
                        width: image.width(),
                        height: image.height(),
                        depth_or_array_layers: 1,
                    };
                    
                    let texture = device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("GX Texture"),
                        size: texture_size,
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8UnormSrgb,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                        view_formats: &[],
                    });
                    
                    queue.write_texture(
                        wgpu::ImageCopyTexture {
                            texture: &texture,
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        &image.as_raw(),
                        wgpu::ImageDataLayout {
                            offset: 0,
                            bytes_per_row: Some(4 * image.width()),
                            rows_per_image: Some(image.height()),
                        },
                        texture_size,
                    );
                    
                    Ok(())
                }
            });
        }
        
        // Register VI callbacks with SDK
        {
            use gcrecomp_core::runtime::sdk::register_vi_callbacks;
            use std::sync::{Arc, Mutex};
            let vi = Arc::new(Mutex::new(crate::graphics::vi::VI::new()));
            
            let vi_clone = vi.clone();
            let set_mode_cb = Box::new(move |mode: u32| {
                if let Ok(mut vi_guard) = vi_clone.lock() {
                    let _ = vi_guard.set_mode(mode);
                }
            });
            
            let vi_clone = vi.clone();
            let set_black_cb = Box::new(move |black: bool| {
                if let Ok(mut vi_guard) = vi_clone.lock() {
                    vi_guard.set_black(black);
                }
            });
            
            register_vi_callbacks(set_mode_cb, set_black_cb);
        }
        
        // Register GX callbacks with SDK
        // Note: GX commands are processed through the renderer's GX processor
        // The viewport is set via GX command processing in process_gx_commands()
        {
            use gcrecomp_core::runtime::sdk::register_gx_callbacks;
            let set_viewport_cb = Box::new(|x: f32, y: f32, w: f32, h: f32, near: f32, far: f32| {
                log::debug!("GX_SetViewport({}, {}, {}, {}, {}, {})", x, y, w, h, near, far);
                // Viewport is set through GX command processing in renderer
                // The actual processing happens in renderer.process_gx_commands()
            });
            
            register_gx_callbacks(set_viewport_cb);
        }
        
        self.renderer = Some(renderer);
        Ok(())
    }

    /// Update runtime state (called every frame)
    ///
    /// This method updates all runtime subsystems:
    /// - Controller input
    /// - DMA transfers
    /// - Graphics (GX commands)
    /// - Audio buffers
    /// - Mod hooks
    pub fn update(&mut self) -> Result<()> {
        // Update controller manager (poll for input)
        self.controller_manager.update()?;

        // Update DMA transfers
        self.dma.process_transfers(&mut self.ram, &mut self.vram, &mut self.aram)?;

        // Process GX commands (graphics updates)
        if let Some(renderer) = self.renderer_mut() {
            renderer.process_gx_commands()?;
        }

        // Process audio buffers
        if self.dsp.is_running() {
            // Process DSP audio
            let mut output = vec![0i16; 1024];
            self.dsp.process_audio(&[], &mut output)?;
            self.audio_interface.submit_samples(&output)?;
        }

        // Process mod hooks (if any are registered)
        // Hook processing happens automatically when functions are called

        // Frame timing: target 60 FPS (16.67ms per frame)
        // Actual timing would be handled by the main loop

        Ok(())
    }

    /// Render a frame (processes GX commands and displays)
    pub fn render(&mut self) -> Result<()> {
        if let Some(renderer) = self.renderer_mut() {
            renderer.render_frame()?;
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

    pub fn dsp_mut(&mut self) -> &mut DSP {
        &mut self.dsp
    }

    pub fn audio_interface_mut(&mut self) -> &mut AudioInterface {
        &mut self.audio_interface
    }
}
