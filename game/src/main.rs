// Game entry point — full game runtime
#[allow(dead_code)]
mod assets;
#[allow(dead_code)]
mod recompiled;

use anyhow::Result;
use gcrecomp_core::runtime::context::CpuContext;
use gcrecomp_core::runtime::memory::MemoryManager;
use gcrecomp_core::runtime::sdk::os::OsState;
use log::info;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::{KeyEvent, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::Window;

struct GameApp {
    window: Option<Arc<Window>>,
    runtime: Option<gcrecomp_runtime::runtime::Runtime>,
    _memory: MemoryManager,
    _os_state: OsState,
    _ctx: CpuContext,
    menu_visible: bool,
}

impl GameApp {
    fn new() -> Self {
        let mut memory = MemoryManager::new();
        let mut os_state = OsState::new();
        let mut ctx = CpuContext::new();

        // Run SDK init sequence
        gcrecomp_core::runtime::sdk::os::os_init(&mut os_state, &mut memory);

        // Initialize DVD virtual filesystem from embedded assets
        os_state.init_dvd(assets::ARCHIVE);

        // Setup initial CPU context
        ctx.set_register(1, 0x817F_FF00); // r1 = stack pointer (top of MEM1)
        ctx.set_register(13, 0x8040_0000); // Typical SDA base
        ctx.set_register(2, 0x8040_0000); // Typical SDA2 base

        info!("SDK initialized, CPU context ready");

        Self {
            window: None,
            runtime: None,
            _memory: memory,
            _os_state: os_state,
            _ctx: ctx,
            menu_visible: false,
        }
    }
}

impl ApplicationHandler for GameApp {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = Window::default_attributes()
            .with_title("GCRecomp")
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));

        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .expect("Failed to create window"),
        );

        let mut runtime =
            gcrecomp_runtime::runtime::Runtime::new().expect("Failed to init runtime");
        runtime
            .initialize_graphics(window.clone())
            .expect("Failed to init graphics");
        if let Err(e) = runtime.initialize_audio() {
            log::warn!(
                "Audio initialization failed (continuing without audio): {}",
                e
            );
        }
        info!("Runtime initialized: graphics, input, audio, video");

        self.window = Some(window);
        self.runtime = Some(runtime);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let runtime = match self.runtime.as_mut() {
            Some(r) => r,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => {
                info!("Window close requested");
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = runtime.renderer_mut() {
                    renderer.resize(size.width, size.height);
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: Key::Named(NamedKey::Escape),
                        state: winit::event::ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                self.menu_visible = !self.menu_visible;
                info!("Menu toggle: {}", self.menu_visible);
            }
            WindowEvent::RedrawRequested => {
                if let Some(renderer) = runtime.renderer_mut() {
                    match renderer.begin_frame() {
                        Ok(frame) => {
                            renderer.end_frame(frame);
                        }
                        Err(e) => {
                            log::warn!("Frame error: {}", e);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        if let Some(runtime) = self.runtime.as_mut() {
            if let Err(e) = runtime.update() {
                log::warn!("Runtime update error: {}", e);
            }
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn main() -> Result<()> {
    // 1. Init logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    info!("GCRecomp game runtime starting");

    // 2. Init Lua engine and load UI screens
    let lua_engine = gcrecomp_lua::engine::LuaEngine::new()?;
    info!("Lua scripting engine initialized");

    // Load UI screen definitions
    let ui_init = std::path::Path::new("lua/ui/init.lua");
    if ui_init.exists() {
        if let Err(e) = lua_engine.execute_file(ui_init) {
            log::warn!("Failed to load UI screens: {}", e);
        }
    }

    // Load game initialization scripts
    let game_init = std::path::Path::new("lua/game/init.lua");
    if game_init.exists() {
        if let Err(e) = lua_engine.execute_file(game_init) {
            log::warn!("Failed to load game scripts: {}", e);
        }
    }

    // 3. Create event loop and run
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = GameApp::new();
    event_loop.run_app(&mut app)?;

    Ok(())
}
