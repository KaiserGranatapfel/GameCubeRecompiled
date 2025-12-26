// Audio RAM simulation (16MB)
use anyhow::Result;

pub struct ARam {
    data: Vec<u8>,
    size: usize,
}

impl ARam {
    pub fn new() -> Self {
        const ARAM_SIZE: usize = 16 * 1024 * 1024; // 16MB
        Self {
            data: vec![0; ARAM_SIZE],
            size: ARAM_SIZE,
        }
    }
    
    pub fn read_u16(&self, address: u32) -> Result<u16> {
        let addr = (address & 0x00FFFFFF) as usize; // 24-bit addressing
        if addr + 2 <= self.size {
            Ok(u16::from_be_bytes([
                self.data[addr],
                self.data[addr + 1],
            ]))
        } else {
            anyhow::bail!("ARAM read out of bounds: 0x{:08X}", address);
        }
    }
    
    pub fn write_u16(&mut self, address: u32, value: u16) -> Result<()> {
        let addr = (address & 0x00FFFFFF) as usize;
        if addr + 2 <= self.size {
            let bytes = value.to_be_bytes();
            self.data[addr..addr + 2].copy_from_slice(&bytes);
            Ok(())
        } else {
            anyhow::bail!("ARAM write out of bounds: 0x{:08X}", address);
        }
    }
    
    pub fn read_bytes(&self, address: u32, len: usize) -> Result<Vec<u8>> {
        let addr = (address & 0x00FFFFFF) as usize;
        if addr + len <= self.size {
            Ok(self.data[addr..addr + len].to_vec())
        } else {
            anyhow::bail!("ARAM read out of bounds: 0x{:08X} len {}", address, len);
        }
    }
    
    pub fn write_bytes(&mut self, address: u32, data: &[u8]) -> Result<()> {
        let addr = (address & 0x00FFFFFF) as usize;
        if addr + data.len() <= self.size {
            self.data[addr..addr + data.len()].copy_from_slice(data);
            Ok(())
        } else {
            anyhow::bail!("ARAM write out of bounds: 0x{:08X} len {}", address, data.len());
        }
    }
}

