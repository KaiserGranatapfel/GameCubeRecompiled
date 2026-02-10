// SDL2 backend for cross-platform controller support
use crate::input::backends::{Backend, ControllerInfo, ControllerType, RawInput};
use anyhow::Result;
use sdl2::GameControllerSubsystem;
use std::collections::HashMap;

pub struct SDL2Backend {
    _sdl_context: sdl2::Sdl,
    controller_subsystem: GameControllerSubsystem,
    controllers: HashMap<usize, sdl2::controller::GameController>,
    _next_id: usize,
}

impl SDL2Backend {
    pub fn new() -> Result<Self> {
        let sdl_context =
            sdl2::init().map_err(|e| anyhow::anyhow!("Failed to initialize SDL2: {}", e))?;

        let controller_subsystem = sdl_context.game_controller().map_err(|e| {
            anyhow::anyhow!("Failed to initialize SDL2 game controller subsystem: {}", e)
        })?;

        Ok(Self {
            _sdl_context: sdl_context,
            controller_subsystem,
            controllers: HashMap::new(),
            _next_id: 0,
        })
    }
}

impl Backend for SDL2Backend {
    fn update(&mut self) -> Result<()> {
        // SDL2 handles events automatically
        Ok(())
    }

    fn enumerate_controllers(&mut self) -> Result<Vec<ControllerInfo>> {
        let mut controllers = Vec::new();
        let num_joysticks = self
            .controller_subsystem
            .num_joysticks()
            .map_err(|e| anyhow::anyhow!("Failed to get joystick count: {}", e))?;

        for i in 0..num_joysticks {
            if self.controller_subsystem.is_game_controller(i) {
                if let Ok(name) = self.controller_subsystem.name_for_index(i) {
                    let controller_type = detect_controller_type(&name);
                    let id = i as usize;

                    // Try to open controller to add to our map
                    if let Ok(controller) = self.controller_subsystem.open(i) {
                        self.controllers.insert(id, controller);
                    }

                    controllers.push(ControllerInfo {
                        id,
                        name: name.to_string(),
                        controller_type,
                        button_count: 16, // SDL2 standard
                        axis_count: 6,    // 2 sticks + 2 triggers
                    });
                }
            }
        }

        Ok(controllers)
    }

    fn get_input(&self, controller_id: usize) -> Result<RawInput> {
        if let Some(controller) = self.controllers.get(&controller_id) {
            let mut buttons = Vec::new();
            let mut axes = Vec::new();
            let mut triggers = Vec::new();

            // Read buttons - SDL2 button enum
            use sdl2::controller::Button;
            buttons.push(controller.button(Button::A));
            buttons.push(controller.button(Button::B));
            buttons.push(controller.button(Button::X));
            buttons.push(controller.button(Button::Y));
            buttons.push(controller.button(Button::Back));
            buttons.push(controller.button(Button::Guide));
            buttons.push(controller.button(Button::Start));
            buttons.push(controller.button(Button::LeftStick));
            buttons.push(controller.button(Button::RightStick));
            buttons.push(controller.button(Button::LeftShoulder));
            buttons.push(controller.button(Button::RightShoulder));
            buttons.push(controller.button(Button::DPadUp));
            buttons.push(controller.button(Button::DPadDown));
            buttons.push(controller.button(Button::DPadLeft));
            buttons.push(controller.button(Button::DPadRight));
            buttons.push(false); // Extra button slot

            // Read axes
            use sdl2::controller::Axis;
            axes.push(controller.axis(Axis::LeftX) as f32 / 32768.0);
            axes.push(controller.axis(Axis::LeftY) as f32 / 32768.0);
            axes.push(controller.axis(Axis::RightX) as f32 / 32768.0);
            axes.push(controller.axis(Axis::RightY) as f32 / 32768.0);

            // Read triggers
            triggers.push(controller.axis(Axis::TriggerLeft) as f32 / 32768.0);
            triggers.push(controller.axis(Axis::TriggerRight) as f32 / 32768.0);

            Ok(RawInput {
                buttons,
                axes,
                triggers,
                hat: None,
            })
        } else {
            anyhow::bail!("Controller not found: {}", controller_id);
        }
    }
}

fn detect_controller_type(name: &str) -> ControllerType {
    let name_lower = name.to_lowercase();
    if name_lower.contains("xbox") || name_lower.contains("xinput") {
        ControllerType::Xbox
    } else if name_lower.contains("playstation")
        || name_lower.contains("dualshock")
        || name_lower.contains("dualsense")
    {
        ControllerType::PlayStation
    } else if name_lower.contains("switch") || name_lower.contains("pro controller") {
        ControllerType::SwitchPro
    } else {
        ControllerType::Generic
    }
}
