//! Save state system
//!
//! Allows saving and loading complete game state for quick save/load functionality

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Save state metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveStateMetadata {
    /// Save state name
    pub name: String,
    /// Game identifier (if available)
    pub game_id: Option<String>,
    /// Timestamp when saved
    pub timestamp: u64,
    /// Save state version (for compatibility)
    pub version: u32,
    /// Optional screenshot path
    pub screenshot_path: Option<PathBuf>,
}

/// Complete save state data
#[derive(Debug, Serialize, Deserialize)]
pub struct SaveState {
    /// Metadata
    pub metadata: SaveStateMetadata,
    /// RAM contents (24MB)
    pub ram: Vec<u8>,
    /// VRAM contents (2MB)
    pub vram: Vec<u8>,
    /// ARAM contents (16MB)
    pub aram: Vec<u8>,
    /// CPU registers (if applicable)
    pub cpu_state: Option<Vec<u8>>,
    /// Additional state data
    pub extra_data: HashMap<String, Vec<u8>>,
}

impl SaveState {
    /// Create a new save state
    pub fn new(name: String, game_id: Option<String>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            metadata: SaveStateMetadata {
                name,
                game_id,
                timestamp,
                version: 1,
                screenshot_path: None,
            },
            ram: Vec::new(),
            vram: Vec::new(),
            aram: Vec::new(),
            cpu_state: None,
            extra_data: HashMap::new(),
        }
    }

    /// Save to file
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let parent = path.parent().context("Invalid save path")?;
        std::fs::create_dir_all(parent)
            .context("Failed to create save directory")?;

        let data = bincode::serialize(self)
            .context("Failed to serialize save state")?;
        std::fs::write(path, data)
            .context("Failed to write save state file")?;

        Ok(())
    }

    /// Load from file
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let data = std::fs::read(path)
            .context("Failed to read save state file")?;
        let save_state: SaveState = bincode::deserialize(&data)
            .context("Failed to deserialize save state")?;

        Ok(save_state)
    }
}

/// Save state manager
pub struct SaveStateManager {
    save_directory: PathBuf,
    current_save: Option<SaveState>,
}

impl SaveStateManager {
    /// Create a new save state manager
    pub fn new(save_directory: PathBuf) -> Self {
        std::fs::create_dir_all(&save_directory).ok();
        Self {
            save_directory,
            current_save: None,
        }
    }

    /// Get save directory
    pub fn save_directory(&self) -> &Path {
        &self.save_directory
    }

    /// Set save directory
    pub fn set_save_directory(&mut self, directory: PathBuf) {
        self.save_directory = directory;
        std::fs::create_dir_all(&self.save_directory).ok();
    }

    /// Create a save state from runtime data
    pub fn create_save_state(
        &mut self,
        name: String,
        game_id: Option<String>,
        ram: &[u8],
        vram: &[u8],
        aram: &[u8],
    ) -> Result<SaveState> {
        let mut save_state = SaveState::new(name, game_id);
        save_state.ram = ram.to_vec();
        save_state.vram = vram.to_vec();
        save_state.aram = aram.to_vec();

        self.current_save = Some(save_state.clone());
        Ok(save_state)
    }

    /// Save current state to file
    pub fn save_to_file(&self, filename: &str) -> Result<()> {
        let save_state = self
            .current_save
            .as_ref()
            .context("No save state to save")?;

        let path = self.save_directory.join(format!("{}.gcsave", filename));
        save_state.save_to_file(&path)?;

        Ok(())
    }

    /// Quick save (auto-named)
    pub fn quick_save(
        &mut self,
        game_id: Option<String>,
        ram: &[u8],
        vram: &[u8],
        aram: &[u8],
    ) -> Result<PathBuf> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let name = if let Some(ref id) = game_id {
            format!("quicksave_{}_{}", id, timestamp)
        } else {
            format!("quicksave_{}", timestamp)
        };

        let save_state = self.create_save_state(name.clone(), game_id, ram, vram, aram)?;
        let path = self.save_directory.join(format!("{}.gcsave", name));
        save_state.save_to_file(&path)?;

        Ok(path)
    }

    /// Load save state from file
    pub fn load_from_file(&mut self, filename: &str) -> Result<SaveState> {
        let path = self.save_directory.join(format!("{}.gcsave", filename));
        let save_state = SaveState::load_from_file(&path)?;
        self.current_save = Some(save_state.clone());
        Ok(save_state)
    }

    /// List all save states
    pub fn list_save_states(&self) -> Result<Vec<SaveStateMetadata>> {
        let mut saves = Vec::new();

        for entry in std::fs::read_dir(&self.save_directory)
            .context("Failed to read save directory")?
        {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("gcsave") {
                if let Ok(save_state) = SaveState::load_from_file(&path) {
                    saves.push(save_state.metadata);
                }
            }
        }

        // Sort by timestamp (newest first)
        saves.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(saves)
    }

    /// Delete a save state
    pub fn delete_save_state(&self, filename: &str) -> Result<()> {
        let path = self.save_directory.join(format!("{}.gcsave", filename));
        std::fs::remove_file(&path)
            .context("Failed to delete save state")?;
        Ok(())
    }

    /// Get current save state
    pub fn current_save(&self) -> Option<&SaveState> {
        self.current_save.as_ref()
    }
}

impl Default for SaveStateManager {
    fn default() -> Self {
        let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("gcrecomp");
        path.push("saves");
        Self::new(path)
    }
}

