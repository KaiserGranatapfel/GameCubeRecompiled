// Memory mapping and address translation
use anyhow::Result;

pub struct MemoryMapper {
    // Maps virtual addresses to physical memory regions
}

impl Default for MemoryMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryMapper {
    pub fn new() -> Self {
        Self {}
    }

    pub fn translate_address(&self, virtual_addr: u32) -> Result<MemoryRegion> {
        // GameCube memory map
        match virtual_addr {
            // Main RAM — cached mirror (24 MB)
            0x80000000..=0x817FFFFF => Ok(MemoryRegion::Ram(virtual_addr & 0x01FFFFFF)),
            // Main RAM — uncached mirror (same physical RAM)
            0xC0000000..=0xC17FFFFF => Ok(MemoryRegion::Ram(virtual_addr & 0x01FFFFFF)),
            // Hardware registers (includes VI, PE/EFB, SI, EXI, AI, DSP, GX FIFO)
            0xCC000000..=0xCC00FFFF => Ok(MemoryRegion::IO(virtual_addr)),
            _ => {
                // Unmapped or unrecognised
                Ok(MemoryRegion::IO(virtual_addr))
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MemoryRegion {
    Ram(u32), // Physical RAM offset
    IO(u32),  // I/O register address
}
