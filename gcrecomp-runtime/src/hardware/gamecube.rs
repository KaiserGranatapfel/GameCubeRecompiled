//! GameCube Hardware Abstraction
//!
//! This module provides GameCube-specific hardware emulation.

/// GameCube memory map constants.
pub mod memory_map {
    /// Main RAM base address
    pub const MAIN_RAM_BASE: u32 = 0x80000000;
    /// Main RAM size (24MB)
    pub const MAIN_RAM_SIZE: u32 = 0x01800000;
    /// ARAM base address
    pub const ARAM_BASE: u32 = 0xCC000000;
    /// ARAM size (16MB)
    pub const ARAM_SIZE: u32 = 0x01000000;
}

/// GameCube hardware registers.
pub struct GameCubeHardware {
    /// Hardware register state
    registers: HashMap<u32, u32>,
}

impl GameCubeHardware {
    /// Create new GameCube hardware emulation.
    pub fn new() -> Self {
        Self {
            registers: HashMap::new(),
        }
    }

    /// Read hardware register.
    pub fn read_register(&self, address: u32) -> u32 {
        self.registers.get(&address).copied().unwrap_or(0)
    }

    /// Write hardware register.
    pub fn write_register(&mut self, address: u32, value: u32) {
        self.registers.insert(address, value);
    }
}

impl Default for GameCubeHardware {
    fn default() -> Self {
        Self::new()
    }
}

use std::collections::HashMap;
