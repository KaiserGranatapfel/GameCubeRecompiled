// Gilrs backend for cross-platform gamepad support
use crate::input::backends::{Backend, ControllerInfo, ControllerType, HatState, RawInput};
use anyhow::Result;
use gilrs::{Axis, Gilrs};

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

    fn enumerate_controllers(&mut self) -> Result<Vec<ControllerInfo>> {
        let mut controllers = Vec::new();

        for (id, gamepad) in self.gilrs.gamepads() {
            let name = gamepad.name();
            let controller_type = detect_controller_type(name);

            controllers.push(ControllerInfo {
                id: id.into(),
                name: name.to_string(),
                controller_type,
                button_count: 16, // Standard gamepad button count
                axis_count: 6,    // Standard gamepad axis count
            });
        }

        Ok(controllers)
    }

    fn get_input(&self, controller_id: usize) -> Result<RawInput> {
        // Find gamepad by iterating gamepads (gilrs 0.10 API)
        let gamepad = self.gilrs.gamepads()
            .find(|(id, _)| usize::from(*id) == controller_id)
            .map(|(_, g)| g);

        if let Some(gamepad) = gamepad {
            let mut buttons = Vec::new();
            let mut axes = Vec::new();
            let mut triggers = Vec::new();

            // Read standard buttons explicitly (gilrs 0.10 API)
            use gilrs::Button;
            buttons.push(gamepad.is_pressed(Button::South));
            buttons.push(gamepad.is_pressed(Button::East));
            buttons.push(gamepad.is_pressed(Button::West));
            buttons.push(gamepad.is_pressed(Button::North));
            buttons.push(gamepad.is_pressed(Button::LeftTrigger));
            buttons.push(gamepad.is_pressed(Button::RightTrigger));
            buttons.push(gamepad.is_pressed(Button::LeftTrigger2));
            buttons.push(gamepad.is_pressed(Button::RightTrigger2));
            buttons.push(gamepad.is_pressed(Button::Select));
            buttons.push(gamepad.is_pressed(Button::Start));
            buttons.push(gamepad.is_pressed(Button::Mode));
            buttons.push(gamepad.is_pressed(Button::LeftThumb));
            buttons.push(gamepad.is_pressed(Button::RightThumb));
            buttons.push(gamepad.is_pressed(Button::DPadUp));
            buttons.push(gamepad.is_pressed(Button::DPadDown));
            buttons.push(gamepad.is_pressed(Button::DPadLeft));

            // Read axes explicitly
            axes.push(gamepad.value(Axis::LeftStickX));
            axes.push(gamepad.value(Axis::LeftStickY));
            axes.push(gamepad.value(Axis::RightStickX));
            axes.push(gamepad.value(Axis::RightStickY));

            // Read triggers
            let left_trigger = gamepad.value(Axis::LeftZ);
            let right_trigger = gamepad.value(Axis::RightZ);
            triggers.push((left_trigger + 1.0) / 2.0);  // Normalize to 0-1
            triggers.push((right_trigger + 1.0) / 2.0); // Normalize to 0-1

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
