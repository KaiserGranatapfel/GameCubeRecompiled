// GameCube controller button mapping
use crate::input::backends::{RawInput, ControllerType};
use crate::input::controller::{GameCubeInput, GameCubeButtons};

#[derive(Debug, Clone)]
pub struct GameCubeMapping {
    pub controller_type: ControllerType,
    pub button_mappings: ButtonMappings,
    pub stick_mappings: StickMappings,
    pub trigger_mappings: TriggerMappings,
    pub dead_zones: DeadZones,
    pub sensitivity: Sensitivity,
}

#[derive(Debug, Clone)]
pub struct ButtonMappings {
    pub a: ButtonMapping,
    pub b: ButtonMapping,
    pub x: ButtonMapping,
    pub y: ButtonMapping,
    pub start: ButtonMapping,
    pub d_up: ButtonMapping,
    pub d_down: ButtonMapping,
    pub d_left: ButtonMapping,
    pub d_right: ButtonMapping,
    pub l: ButtonMapping,
    pub r: ButtonMapping,
    pub z: ButtonMapping,
}

#[derive(Debug, Clone)]
pub enum ButtonMapping {
    Button(usize),
    AxisPositive(usize),
    AxisNegative(usize),
    Trigger(usize, f32), // trigger index, threshold
    None,
}

#[derive(Debug, Clone)]
pub struct StickMappings {
    pub left_stick: AxisMapping,
    pub right_stick: AxisMapping,
}

#[derive(Debug, Clone)]
pub struct AxisMapping {
    pub x_axis: usize,
    pub y_axis: usize,
    pub invert_x: bool,
    pub invert_y: bool,
}

#[derive(Debug, Clone)]
pub struct TriggerMappings {
    pub left_trigger: usize,
    pub right_trigger: usize,
}

#[derive(Debug, Clone)]
pub struct DeadZones {
    pub left_stick: f32,
    pub right_stick: f32,
    pub left_trigger: f32,
    pub right_trigger: f32,
}

#[derive(Debug, Clone)]
pub struct Sensitivity {
    pub left_stick: f32,
    pub right_stick: f32,
}

impl GameCubeMapping {
    pub fn default_for_controller(controller_info: &crate::input::backends::ControllerInfo) -> Result<Self> {
        match controller_info.controller_type {
            ControllerType::Xbox => Ok(Self::xbox_default()),
            ControllerType::PlayStation => Ok(Self::playstation_default()),
            ControllerType::SwitchPro => Ok(Self::switch_pro_default()),
            _ => Ok(Self::generic_default()),
        }
    }
    
    pub fn xbox_default() -> Self {
        Self {
            controller_type: ControllerType::Xbox,
            button_mappings: ButtonMappings {
                a: ButtonMapping::Button(0), // A button
                b: ButtonMapping::Button(1), // B button
                x: ButtonMapping::Button(2), // X button
                y: ButtonMapping::Button(3), // Y button
                start: ButtonMapping::Button(6), // Menu button
                d_up: ButtonMapping::Button(11), // D-pad up
                d_down: ButtonMapping::Button(12), // D-pad down
                d_left: ButtonMapping::Button(13), // D-pad left
                d_right: ButtonMapping::Button(14), // D-pad right
                l: ButtonMapping::Trigger(4, 0.3), // Left trigger
                r: ButtonMapping::Trigger(5, 0.3), // Right trigger
                z: ButtonMapping::Button(4), // Left bumper
            },
            stick_mappings: StickMappings {
                left_stick: AxisMapping {
                    x_axis: 0,
                    y_axis: 1,
                    invert_x: false,
                    invert_y: true, // Y is typically inverted
                },
                right_stick: AxisMapping {
                    x_axis: 2,
                    y_axis: 3,
                    invert_x: false,
                    invert_y: true,
                },
            },
            trigger_mappings: TriggerMappings {
                left_trigger: 4,
                right_trigger: 5,
            },
            dead_zones: DeadZones {
                left_stick: 0.15,
                right_stick: 0.15,
                left_trigger: 0.1,
                right_trigger: 0.1,
            },
            sensitivity: Sensitivity {
                left_stick: 1.0,
                right_stick: 1.0,
            },
        }
    }
    
    pub fn playstation_default() -> Self {
        // Similar to Xbox but with different button indices
        let mut xbox = Self::xbox_default();
        xbox.controller_type = ControllerType::PlayStation;
        xbox.button_mappings.start = ButtonMapping::Button(9); // Options button
        xbox
    }
    
    pub fn switch_pro_default() -> Self {
        let mut xbox = Self::xbox_default();
        xbox.controller_type = ControllerType::SwitchPro;
        // Switch Pro has similar layout to Xbox
        xbox
    }
    
    pub fn generic_default() -> Self {
        Self::xbox_default()
    }
    
    pub fn map_to_gamecube(&self, input: &RawInput) -> GameCubeInput {
        let mut buttons = GameCubeButtons::default();
        
        // Map buttons
        buttons.a = self.get_button_state(&self.button_mappings.a, input);
        buttons.b = self.get_button_state(&self.button_mappings.b, input);
        buttons.x = self.get_button_state(&self.button_mappings.x, input);
        buttons.y = self.get_button_state(&self.button_mappings.y, input);
        buttons.start = self.get_button_state(&self.button_mappings.start, input);
        buttons.d_up = self.get_button_state(&self.button_mappings.d_up, input);
        buttons.d_down = self.get_button_state(&self.button_mappings.d_down, input);
        buttons.d_left = self.get_button_state(&self.button_mappings.d_left, input);
        buttons.d_right = self.get_button_state(&self.button_mappings.d_right, input);
        buttons.l = self.get_button_state(&self.button_mappings.l, input);
        buttons.r = self.get_button_state(&self.button_mappings.r, input);
        buttons.z = self.get_button_state(&self.button_mappings.z, input);
        
        // Map sticks with dead zones and sensitivity
        let left_stick = self.map_stick(
            &self.stick_mappings.left_stick,
            &self.dead_zones.left_stick,
            self.sensitivity.left_stick,
            input,
        );
        
        let right_stick = self.map_stick(
            &self.stick_mappings.right_stick,
            &self.dead_zones.right_stick,
            self.sensitivity.right_stick,
            input,
        );
        
        // Map triggers
        let left_trigger = self.map_trigger(
            self.trigger_mappings.left_trigger,
            self.dead_zones.left_trigger,
            input,
        );
        
        let right_trigger = self.map_trigger(
            self.trigger_mappings.right_trigger,
            self.dead_zones.right_trigger,
            input,
        );
        
        GameCubeInput {
            buttons,
            left_stick,
            right_stick,
            left_trigger,
            right_trigger,
        }
    }
    
    fn get_button_state(&self, mapping: &ButtonMapping, input: &RawInput) -> bool {
        match mapping {
            ButtonMapping::Button(idx) => {
                input.buttons.get(*idx).copied().unwrap_or(false)
            }
            ButtonMapping::AxisPositive(idx) => {
                input.axes.get(*idx).map(|&v| v > 0.5).unwrap_or(false)
            }
            ButtonMapping::AxisNegative(idx) => {
                input.axes.get(*idx).map(|&v| v < -0.5).unwrap_or(false)
            }
            ButtonMapping::Trigger(idx, threshold) => {
                input.triggers.get(*idx).map(|&v| v > *threshold).unwrap_or(false)
            }
            ButtonMapping::None => false,
        }
    }
    
    fn map_stick(
        &self,
        axis_mapping: &AxisMapping,
        dead_zone: &f32,
        sensitivity: f32,
        input: &RawInput,
    ) -> (f32, f32) {
        let x = input.axes.get(axis_mapping.x_axis).copied().unwrap_or(0.0);
        let y = input.axes.get(axis_mapping.y_axis).copied().unwrap_or(0.0);
        
        let mut x = if axis_mapping.invert_x { -x } else { x };
        let mut y = if axis_mapping.invert_y { -y } else { y };
        
        // Apply dead zone
        let magnitude = (x * x + y * y).sqrt();
        if magnitude < *dead_zone {
            return (0.0, 0.0);
        }
        
        // Normalize and scale
        if magnitude > 1.0 {
            x /= magnitude;
            y /= magnitude;
        }
        
        // Apply sensitivity
        x *= sensitivity;
        y *= sensitivity;
        
        (x, y)
    }
    
    fn map_trigger(&self, trigger_idx: usize, dead_zone: f32, input: &RawInput) -> f32 {
        let value = input.triggers.get(trigger_idx).copied().unwrap_or(0.0);
        if value < dead_zone {
            0.0
        } else {
            // Normalize from dead_zone to 1.0
            (value - dead_zone) / (1.0 - dead_zone)
        }
    }
}

use anyhow::Result;

