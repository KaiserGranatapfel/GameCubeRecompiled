//! Enhanced button mapping system with visual configuration support
//!
//! Provides a user-friendly way to map any controller button/axis to GameCube buttons

use crate::input::backends::RawInput;
use crate::input::gamecube_mapping::{ButtonMapping, GameCubeMapping};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Button mapper with support for custom mappings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonMapper {
    /// Custom button mappings
    pub button_mappings: HashMap<String, ButtonMapping>,
    /// Custom axis mappings
    pub axis_mappings: HashMap<String, AxisMappingConfig>,
    /// Custom trigger mappings
    pub trigger_mappings: HashMap<String, TriggerMappingConfig>,
    /// Dead zone configurations
    pub dead_zones: DeadZoneConfig,
    /// Sensitivity configurations
    pub sensitivity: SensitivityConfig,
}

/// Axis mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxisMappingConfig {
    pub axis_index: usize,
    pub invert: bool,
    pub dead_zone: f32,
    pub sensitivity: f32,
}

/// Trigger mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerMappingConfig {
    pub trigger_index: usize,
    pub threshold: f32,
    pub dead_zone: f32,
}

/// Dead zone configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadZoneConfig {
    pub left_stick: f32,
    pub right_stick: f32,
    pub left_trigger: f32,
    pub right_trigger: f32,
}

/// Sensitivity configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitivityConfig {
    pub left_stick: f32,
    pub right_stick: f32,
}

impl Default for ButtonMapper {
    fn default() -> Self {
        Self {
            button_mappings: HashMap::new(),
            axis_mappings: HashMap::new(),
            trigger_mappings: HashMap::new(),
            dead_zones: DeadZoneConfig {
                left_stick: 0.15,
                right_stick: 0.15,
                left_trigger: 0.1,
                right_trigger: 0.1,
            },
            sensitivity: SensitivityConfig {
                left_stick: 1.0,
                right_stick: 1.0,
            },
        }
    }
}

impl ButtonMapper {
    /// Create a new button mapper
    pub fn new() -> Self {
        Self::default()
    }

    /// Map a GameCube button to a controller button
    pub fn map_button(&mut self, gamecube_button: &str, mapping: ButtonMapping) {
        self.button_mappings.insert(gamecube_button.to_string(), mapping);
    }

    /// Map a stick axis
    pub fn map_axis(&mut self, stick_name: &str, config: AxisMappingConfig) {
        self.axis_mappings.insert(stick_name.to_string(), config);
    }

    /// Map a trigger
    pub fn map_trigger(&mut self, trigger_name: &str, config: TriggerMappingConfig) {
        self.trigger_mappings.insert(trigger_name.to_string(), config);
    }

    /// Convert to GameCubeMapping
    pub fn to_gamecube_mapping(&self, controller_type: crate::input::backends::ControllerType) -> GameCubeMapping {
        let mut mapping = GameCubeMapping::default_for_controller(
            &crate::input::backends::ControllerInfo {
                id: 0,
                name: String::new(),
                controller_type,
                button_count: 0,
                axis_count: 0,
            },
        ).unwrap_or_else(|_| GameCubeMapping::generic_default());

        // Apply custom button mappings
        if let Some(btn_map) = self.button_mappings.get("a") {
            mapping.button_mappings.a = btn_map.clone();
        }
        if let Some(btn_map) = self.button_mappings.get("b") {
            mapping.button_mappings.b = btn_map.clone();
        }
        if let Some(btn_map) = self.button_mappings.get("x") {
            mapping.button_mappings.x = btn_map.clone();
        }
        if let Some(btn_map) = self.button_mappings.get("y") {
            mapping.button_mappings.y = btn_map.clone();
        }
        if let Some(btn_map) = self.button_mappings.get("start") {
            mapping.button_mappings.start = btn_map.clone();
        }
        if let Some(btn_map) = self.button_mappings.get("d_up") {
            mapping.button_mappings.d_up = btn_map.clone();
        }
        if let Some(btn_map) = self.button_mappings.get("d_down") {
            mapping.button_mappings.d_down = btn_map.clone();
        }
        if let Some(btn_map) = self.button_mappings.get("d_left") {
            mapping.button_mappings.d_left = btn_map.clone();
        }
        if let Some(btn_map) = self.button_mappings.get("d_right") {
            mapping.button_mappings.d_right = btn_map.clone();
        }
        if let Some(btn_map) = self.button_mappings.get("l") {
            mapping.button_mappings.l = btn_map.clone();
        }
        if let Some(btn_map) = self.button_mappings.get("r") {
            mapping.button_mappings.r = btn_map.clone();
        }
        if let Some(btn_map) = self.button_mappings.get("z") {
            mapping.button_mappings.z = btn_map.clone();
        }

        // Apply axis mappings
        if let Some(axis_config) = self.axis_mappings.get("left_stick_x") {
            mapping.stick_mappings.left_stick.x_axis = axis_config.axis_index;
            mapping.stick_mappings.left_stick.invert_x = axis_config.invert;
            mapping.dead_zones.left_stick = axis_config.dead_zone;
            mapping.sensitivity.left_stick = axis_config.sensitivity;
        }
        if let Some(axis_config) = self.axis_mappings.get("left_stick_y") {
            mapping.stick_mappings.left_stick.y_axis = axis_config.axis_index;
            mapping.stick_mappings.left_stick.invert_y = axis_config.invert;
        }
        if let Some(axis_config) = self.axis_mappings.get("right_stick_x") {
            mapping.stick_mappings.right_stick.x_axis = axis_config.axis_index;
            mapping.stick_mappings.right_stick.invert_x = axis_config.invert;
            mapping.dead_zones.right_stick = axis_config.dead_zone;
            mapping.sensitivity.right_stick = axis_config.sensitivity;
        }
        if let Some(axis_config) = self.axis_mappings.get("right_stick_y") {
            mapping.stick_mappings.right_stick.y_axis = axis_config.axis_index;
            mapping.stick_mappings.right_stick.invert_y = axis_config.invert;
        }

        // Apply trigger mappings
        if let Some(trigger_config) = self.trigger_mappings.get("left_trigger") {
            mapping.trigger_mappings.left_trigger = trigger_config.trigger_index;
            mapping.dead_zones.left_trigger = trigger_config.dead_zone;
        }
        if let Some(trigger_config) = self.trigger_mappings.get("right_trigger") {
            mapping.trigger_mappings.right_trigger = trigger_config.trigger_index;
            mapping.dead_zones.right_trigger = trigger_config.dead_zone;
        }

        // Apply global dead zones and sensitivity
        mapping.dead_zones = self.dead_zones.clone();
        mapping.sensitivity = self.sensitivity.clone();

        mapping
    }

    /// Save mapper to file
    pub fn save_to_file(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load mapper from file
    pub fn load_from_file(path: &std::path::Path) -> anyhow::Result<Self> {
        let json = std::fs::read_to_string(path)?;
        let mapper: ButtonMapper = serde_json::from_str(&json)?;
        Ok(mapper)
    }
}

/// Helper to detect which button/axis is being pressed
pub struct InputDetector;

impl InputDetector {
    /// Detect which button is currently pressed
    pub fn detect_button(input: &RawInput) -> Option<(usize, String)> {
        for (i, &pressed) in input.buttons.iter().enumerate() {
            if pressed {
                return Some((i, format!("Button {}", i)));
            }
        }
        None
    }

    /// Detect which axis is currently active
    pub fn detect_axis(input: &RawInput, threshold: f32) -> Option<(usize, f32, String)> {
        for (i, &value) in input.axes.iter().enumerate() {
            if value.abs() > threshold {
                return Some((i, value, format!("Axis {} ({:.2})", i, value)));
            }
        }
        None
    }

    /// Detect which trigger is currently active
    pub fn detect_trigger(input: &RawInput, threshold: f32) -> Option<(usize, f32, String)> {
        for (i, &value) in input.triggers.iter().enumerate() {
            if value > threshold {
                return Some((i, value, format!("Trigger {} ({:.2})", i, value)));
            }
        }
        None
    }
}

