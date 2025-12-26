// Settings persistence
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    pub fps_limit: Option<u32>,
    pub resolution: (u32, u32),
    pub vsync: bool,
    pub aspect_ratio: AspectRatio,
    pub render_scale: f32,
    pub master_volume: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,
    pub audio_backend: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AspectRatio {
    Original,
    Widescreen,
    UltraWide,
    Custom(f32),
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            fps_limit: Some(60),
            resolution: (1920, 1080),
            vsync: true,
            aspect_ratio: AspectRatio::Widescreen,
            render_scale: 1.0,
            master_volume: 1.0,
            music_volume: 1.0,
            sfx_volume: 1.0,
            audio_backend: "default".to_string(),
        }
    }
}

impl GameConfig {
    fn config_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("gcrecomp");
        path.push("config.json");
        path
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path).context("Failed to read config file")?;
            let config: GameConfig =
                serde_json::from_str(&content).context("Failed to parse config file")?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create config directory")?;
        }
        let content = serde_json::to_string_pretty(self).context("Failed to serialize config")?;
        std::fs::write(&path, content).context("Failed to write config file")?;
        Ok(())
    }
}
