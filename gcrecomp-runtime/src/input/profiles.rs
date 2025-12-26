// Controller profile management
use serde::{Deserialize, Serialize};
use crate::input::gamecube_mapping::GameCubeMapping;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerProfile {
    pub name: String,
    pub controller_type: String,
    pub mapping: SerializedMapping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedMapping {
    // Simplified serialization - would need full mapping structure
    pub button_mappings: Vec<u8>,
    pub axis_mappings: Vec<u8>,
    pub dead_zones: Vec<f32>,
    pub sensitivity: Vec<f32>,
}

impl ControllerProfile {
    pub fn from_mapping(name: String, mapping: GameCubeMapping) -> Self {
        // Convert mapping to serializable format
        Self {
            name,
            controller_type: format!("{:?}", mapping.controller_type),
            mapping: SerializedMapping {
                button_mappings: vec![], // Would serialize actual mappings
                axis_mappings: vec![],
                dead_zones: vec![
                    mapping.dead_zones.left_stick,
                    mapping.dead_zones.right_stick,
                    mapping.dead_zones.left_trigger,
                    mapping.dead_zones.right_trigger,
                ],
                sensitivity: vec![
                    mapping.sensitivity.left_stick,
                    mapping.sensitivity.right_stick,
                ],
            },
        }
    }
    
    pub fn to_gamecube_mapping(&self) -> Result<GameCubeMapping> {
        // Convert serialized format back to mapping
        // For now, return default based on controller type
        match self.controller_type.as_str() {
            "Xbox" => Ok(GameCubeMapping::xbox_default()),
            "PlayStation" => Ok(GameCubeMapping::playstation_default()),
            "SwitchPro" => Ok(GameCubeMapping::switch_pro_default()),
            _ => Ok(GameCubeMapping::generic_default()),
        }
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

