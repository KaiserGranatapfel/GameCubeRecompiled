// Controller profile management
use crate::input::gamecube_mapping::{
    AxisMapping, ButtonMapping, ButtonMappings, DeadZones, GameCubeMapping, Sensitivity,
    StickMappings, TriggerMappings,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerProfile {
    pub name: String,
    pub controller_type: String,
    pub mapping: SerializedMapping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedMapping {
    pub buttons: SerializedButtons,
    pub sticks: SerializedSticks,
    pub triggers: SerializedTriggers,
    pub dead_zones: SerializedDeadZones,
    pub sensitivity: SerializedSensitivity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedButtons {
    pub a: SerializedButtonMapping,
    pub b: SerializedButtonMapping,
    pub x: SerializedButtonMapping,
    pub y: SerializedButtonMapping,
    pub start: SerializedButtonMapping,
    pub d_up: SerializedButtonMapping,
    pub d_down: SerializedButtonMapping,
    pub d_left: SerializedButtonMapping,
    pub d_right: SerializedButtonMapping,
    pub l: SerializedButtonMapping,
    pub r: SerializedButtonMapping,
    pub z: SerializedButtonMapping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SerializedButtonMapping {
    Button(usize),
    AxisPositive(usize),
    AxisNegative(usize),
    Trigger(usize, f32),
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedSticks {
    pub left: SerializedAxisMapping,
    pub right: SerializedAxisMapping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedAxisMapping {
    pub x_axis: usize,
    pub y_axis: usize,
    pub invert_x: bool,
    pub invert_y: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedTriggers {
    pub left: usize,
    pub right: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedDeadZones {
    pub left_stick: f32,
    pub right_stick: f32,
    pub left_trigger: f32,
    pub right_trigger: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedSensitivity {
    pub left_stick: f32,
    pub right_stick: f32,
}

// -- Conversion helpers --------------------------------------------------

fn serialize_button(mapping: &ButtonMapping) -> SerializedButtonMapping {
    match mapping {
        ButtonMapping::Button(i) => SerializedButtonMapping::Button(*i),
        ButtonMapping::AxisPositive(i) => SerializedButtonMapping::AxisPositive(*i),
        ButtonMapping::AxisNegative(i) => SerializedButtonMapping::AxisNegative(*i),
        ButtonMapping::Trigger(i, t) => SerializedButtonMapping::Trigger(*i, *t),
        ButtonMapping::None => SerializedButtonMapping::None,
    }
}

fn deserialize_button(mapping: &SerializedButtonMapping) -> ButtonMapping {
    match mapping {
        SerializedButtonMapping::Button(i) => ButtonMapping::Button(*i),
        SerializedButtonMapping::AxisPositive(i) => ButtonMapping::AxisPositive(*i),
        SerializedButtonMapping::AxisNegative(i) => ButtonMapping::AxisNegative(*i),
        SerializedButtonMapping::Trigger(i, t) => ButtonMapping::Trigger(*i, *t),
        SerializedButtonMapping::None => ButtonMapping::None,
    }
}

impl ControllerProfile {
    pub fn from_mapping(name: String, mapping: GameCubeMapping) -> Self {
        let bm = &mapping.button_mappings;
        let sm = &mapping.stick_mappings;
        let tm = &mapping.trigger_mappings;
        let dz = &mapping.dead_zones;
        let sn = &mapping.sensitivity;

        Self {
            name,
            controller_type: format!("{:?}", mapping.controller_type),
            mapping: SerializedMapping {
                buttons: SerializedButtons {
                    a: serialize_button(&bm.a),
                    b: serialize_button(&bm.b),
                    x: serialize_button(&bm.x),
                    y: serialize_button(&bm.y),
                    start: serialize_button(&bm.start),
                    d_up: serialize_button(&bm.d_up),
                    d_down: serialize_button(&bm.d_down),
                    d_left: serialize_button(&bm.d_left),
                    d_right: serialize_button(&bm.d_right),
                    l: serialize_button(&bm.l),
                    r: serialize_button(&bm.r),
                    z: serialize_button(&bm.z),
                },
                sticks: SerializedSticks {
                    left: SerializedAxisMapping {
                        x_axis: sm.left_stick.x_axis,
                        y_axis: sm.left_stick.y_axis,
                        invert_x: sm.left_stick.invert_x,
                        invert_y: sm.left_stick.invert_y,
                    },
                    right: SerializedAxisMapping {
                        x_axis: sm.right_stick.x_axis,
                        y_axis: sm.right_stick.y_axis,
                        invert_x: sm.right_stick.invert_x,
                        invert_y: sm.right_stick.invert_y,
                    },
                },
                triggers: SerializedTriggers {
                    left: tm.left_trigger,
                    right: tm.right_trigger,
                },
                dead_zones: SerializedDeadZones {
                    left_stick: dz.left_stick,
                    right_stick: dz.right_stick,
                    left_trigger: dz.left_trigger,
                    right_trigger: dz.right_trigger,
                },
                sensitivity: SerializedSensitivity {
                    left_stick: sn.left_stick,
                    right_stick: sn.right_stick,
                },
            },
        }
    }

    pub fn to_gamecube_mapping(&self) -> Result<GameCubeMapping> {
        use crate::input::backends::ControllerType;

        let sm = &self.mapping;
        let sb = &sm.buttons;

        let controller_type = match self.controller_type.as_str() {
            "Xbox" => ControllerType::Xbox,
            "PlayStation" => ControllerType::PlayStation,
            "SwitchPro" => ControllerType::SwitchPro,
            _ => ControllerType::Generic,
        };

        Ok(GameCubeMapping {
            controller_type,
            button_mappings: ButtonMappings {
                a: deserialize_button(&sb.a),
                b: deserialize_button(&sb.b),
                x: deserialize_button(&sb.x),
                y: deserialize_button(&sb.y),
                start: deserialize_button(&sb.start),
                d_up: deserialize_button(&sb.d_up),
                d_down: deserialize_button(&sb.d_down),
                d_left: deserialize_button(&sb.d_left),
                d_right: deserialize_button(&sb.d_right),
                l: deserialize_button(&sb.l),
                r: deserialize_button(&sb.r),
                z: deserialize_button(&sb.z),
            },
            stick_mappings: StickMappings {
                left_stick: AxisMapping {
                    x_axis: sm.sticks.left.x_axis,
                    y_axis: sm.sticks.left.y_axis,
                    invert_x: sm.sticks.left.invert_x,
                    invert_y: sm.sticks.left.invert_y,
                },
                right_stick: AxisMapping {
                    x_axis: sm.sticks.right.x_axis,
                    y_axis: sm.sticks.right.y_axis,
                    invert_x: sm.sticks.right.invert_x,
                    invert_y: sm.sticks.right.invert_y,
                },
            },
            trigger_mappings: TriggerMappings {
                left_trigger: sm.triggers.left,
                right_trigger: sm.triggers.right,
            },
            dead_zones: DeadZones {
                left_stick: sm.dead_zones.left_stick,
                right_stick: sm.dead_zones.right_stick,
                left_trigger: sm.dead_zones.left_trigger,
                right_trigger: sm.dead_zones.right_trigger,
            },
            sensitivity: Sensitivity {
                left_stick: sm.sensitivity.left_stick,
                right_stick: sm.sensitivity.right_stick,
            },
        })
    }

    pub fn save_to_file(&self, path: &std::path::Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load_from_file(path: &std::path::Path) -> Result<Self> {
        let json = std::fs::read_to_string(path)?;
        let profile: ControllerProfile = serde_json::from_str(&json)?;
        Ok(profile)
    }
}
