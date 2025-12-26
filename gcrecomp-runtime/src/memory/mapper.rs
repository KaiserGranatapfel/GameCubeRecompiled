// Memory mapping and address translation
use anyhow::Result;

pub struct MemoryMapper {
    // Maps virtual addresses to physical memory regions
}

impl MemoryMapper {
    pub fn new() -> Self {
        Self {}
    }

    pub fn translate_address(&self, virtual_addr: u32) -> Result<MemoryRegion> {
        // GameCube memory map
        match virtual_addr {
            0x80000000..=0x817FFFFF => {
                // Main RAM (24MB, mirrored)
                Ok(MemoryRegion::Ram((virtual_addr & 0x00FFFFFF) as u32))
            }
            0xCC000000..=0xCC1FFFFF => {
                // Video RAM (2MB)
                Ok(MemoryRegion::VRam((virtual_addr & 0x001FFFFF) as u32))
            }
            0x80000000..=0x80FFFFFF => {
                // Audio RAM (16MB)
                Ok(MemoryRegion::ARam((virtual_addr & 0x00FFFFFF) as u32))
            }
            _ => {
                // I/O registers or unmapped
                Ok(MemoryRegion::IO(virtual_addr))
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MemoryRegion {
    Ram(u32),  // Physical RAM address
    VRam(u32), // Physical VRAM address
    ARam(u32), // Physical ARAM address
    IO(u32),   // I/O register address
}
