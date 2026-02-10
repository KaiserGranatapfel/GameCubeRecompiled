// Game integration hooks
use anyhow::Result;
use winit::event::{ElementState, WindowEvent};
use winit::keyboard::{Key, NamedKey};
use winit::window::Window;

type LuaEventHandler = Box<dyn Fn(&str) -> bool + Send>;

#[derive(Default)]
pub struct GameIntegration {
    menu_visible: bool,
    lua_event_handler: Option<LuaEventHandler>,
}

impl GameIntegration {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_lua_event_handler(&mut self, handler: LuaEventHandler) {
        self.lua_event_handler = Some(handler);
    }

    pub fn handle_event(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed
                    && event.logical_key == Key::Named(NamedKey::Escape)
                {
                    self.menu_visible = !self.menu_visible;
                    return true; // Event handled, don't pass to game
                }
                if self.menu_visible {
                    // Try Lua handler first
                    if let Some(ref handler) = self.lua_event_handler {
                        if handler("keyboard") {
                            return true;
                        }
                    }
                    return true; // Consume keyboard input when menu is visible
                }
                false
            }
            _ if self.menu_visible => {
                // If menu is visible, consume all input events
                true
            }
            _ => false, // Pass event to game
        }
    }

    pub fn is_menu_visible(&self) -> bool {
        self.menu_visible
    }

    pub fn set_menu_visible(&mut self, visible: bool) {
        self.menu_visible = visible;
    }
}

pub fn hook_rendering_pipeline(_window: &Window) -> Result<()> {
    // In a real implementation, this would hook into the game's rendering pipeline
    // For now, we'll use iced's built-in rendering
    Ok(())
}

pub fn overlay_menu(_window: &Window) -> Result<()> {
    // The menu overlay is handled by the iced Application
    // This function can be used for additional overlay rendering if needed
    Ok(())
}
