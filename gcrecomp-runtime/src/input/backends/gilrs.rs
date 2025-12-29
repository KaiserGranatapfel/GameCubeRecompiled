// Gilrs backend for cross-platform gamepad support
use crate::input::backends::{Backend, ControllerInfo, ControllerType, HatState, RawInput};
use anyhow::Result;
use gilrs::{Axis, Button, Gilrs};

pub struct GilrsBackend {
    gilrs: Gilrs,
}

impl GilrsBackend {
    pub fn new() -> Result<Self> {
        let gilrs =
            Gilrs::new().map_err(|e| anyhow::anyhow!("Failed to initialize gilrs: {}", e))?;

        Ok(Self { gilrs })
    }
}

impl Backend for GilrsBackend {
    fn update(&mut self) -> Result<()> {
        // Process events
        while self.gilrs.next_event().is_some() {}
        Ok(())
    }

    fn enumerate_controllers(&self) -> Result<Vec<ControllerInfo>> {
        let mut controllers = Vec::new();

        for (id, gamepad) in self.gilrs.gamepads() {
            let name = gamepad.name();
            let controller_type = detect_controller_type(name);

            controllers.push(ControllerInfo {
                id: id.into(),
                name: name.to_string(),
                controller_type,
                button_count: gamepad.buttons().count(),
                axis_count: gamepad.axes().count(),
            });
        }

        Ok(controllers)
    }

    fn get_input(&self, controller_id: usize) -> Result<RawInput> {
        if let Some(gamepad) = self
            .gilrs
            .gamepad(gilrs::GamepadId::from(controller_id as u32))
        {
            let mut buttons = Vec::new();
            let mut axes = Vec::new();
            let mut triggers = Vec::new();

            // Read buttons
            for button in gamepad.buttons() {
                buttons.push(gamepad.is_pressed(button));
            }

            // Read axes
            for axis in gamepad.axes() {
                let value = gamepad.value(axis);
                axes.push(value);

                // Check if this is a trigger
                if matches!(axis, Axis::LeftZ | Axis::RightZ) {
                    triggers.push((value + 1.0) / 2.0); // Normalize to 0-1
                }
            }

            // Gilrs doesn't have direct gyro support in the main API
            // However, we can try to read gyro data if the controller supports it
            // via platform-specific APIs or hidapi
            // For now, return None - can be enhanced with hidapi integration
            let gyro = None; // Would need hidapi integration for gilrs controllers
            
            Ok(RawInput {
                buttons,
                axes,
                triggers,
                hat: None,
                gyro,
            })
        } else {
            anyhow::bail!("Controller not found: {}", controller_id);
        }
    }
}

fn detect_controller_type(name: &str) -> ControllerType {
    let name_lower = name.to_lowercase();
    if name_lower.contains("xbox") {
        ControllerType::Xbox
    } else if name_lower.contains("playstation") || name_lower.contains("dualshock") {
        ControllerType::PlayStation
    } else if name_lower.contains("switch") || name_lower.contains("pro controller") {
        ControllerType::SwitchPro
    } else {
        ControllerType::Generic
    }
}
