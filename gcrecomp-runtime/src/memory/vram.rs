// Video RAM simulation (2MB)
use anyhow::Result;

pub struct VRam {
    data: Vec<u8>,
    size: usize,
}

impl Default for VRam {
    fn default() -> Self {
        Self::new()
    }
}

impl VRam {
    pub fn new() -> Self {
        const VRAM_SIZE: usize = 2 * 1024 * 1024; // 2MB
        Self {
            data: vec![0; VRAM_SIZE],
            size: VRAM_SIZE,
        }
    }

    pub fn read_u32(&self, address: u32) -> Result<u32> {
        let addr = (address & 0x001FFFFF) as usize; // 21-bit addressing
        if addr + 4 <= self.size {
            Ok(u32::from_be_bytes([
                self.data[addr],
                self.data[addr + 1],
                self.data[addr + 2],
                self.data[addr + 3],
            ]))
        } else {
            anyhow::bail!("VRAM read out of bounds: 0x{:08X}", address);
        }
    }

    pub fn write_u32(&mut self, address: u32, value: u32) -> Result<()> {
        let addr = (address & 0x001FFFFF) as usize;
        if addr + 4 <= self.size {
            let bytes = value.to_be_bytes();
            self.data[addr..addr + 4].copy_from_slice(&bytes);
            Ok(())
        } else {
            anyhow::bail!("VRAM write out of bounds: 0x{:08X}", address);
        }
    }

    pub fn read_bytes(&self, address: u32, len: usize) -> Result<Vec<u8>> {
        let addr = (address & 0x001FFFFF) as usize;
        if addr + len <= self.size {
            Ok(self.data[addr..addr + len].to_vec())
        } else {
            anyhow::bail!("VRAM read out of bounds: 0x{:08X} len {}", address, len);
        }
    }

    pub fn write_bytes(&mut self, address: u32, data: &[u8]) -> Result<()> {
        let addr = (address & 0x001FFFFF) as usize;
        if addr + data.len() <= self.size {
            self.data[addr..addr + data.len()].copy_from_slice(data);
            Ok(())
        } else {
            anyhow::bail!(
                "VRAM write out of bounds: 0x{:08X} len {}",
                address,
                data.len()
            );
        }
    }
}
