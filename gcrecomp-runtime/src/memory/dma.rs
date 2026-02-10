// DMA (Direct Memory Access) system
use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct DmaSystem {
    channels: Vec<DmaChannel>,
}

pub struct DmaChannel {
    active: Arc<AtomicBool>,
    source: u32,
    destination: u32,
    length: u32,
    callback: Option<Box<dyn Fn() + Send + Sync>>,
}

impl Default for DmaSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl DmaSystem {
    pub fn new() -> Self {
        Self {
            channels: (0..4)
                .map(|_| DmaChannel {
                    active: Arc::new(AtomicBool::new(false)),
                    source: 0,
                    destination: 0,
                    length: 0,
                    callback: None,
                })
                .collect(),
        }
    }

    pub fn start_transfer(
        &mut self,
        channel: usize,
        source: u32,
        destination: u32,
        length: u32,
    ) -> Result<()> {
        if channel >= self.channels.len() {
            anyhow::bail!("Invalid DMA channel: {}", channel);
        }

        let ch = &mut self.channels[channel];
        ch.source = source;
        ch.destination = destination;
        ch.length = length;
        ch.active.store(true, Ordering::SeqCst);

        Ok(())
    }

    pub fn is_active(&self, channel: usize) -> bool {
        if channel < self.channels.len() {
            self.channels[channel].active.load(Ordering::SeqCst)
        } else {
            false
        }
    }

    /// Execute a pending DMA transfer, copying bytes between memory regions.
    ///
    /// `ram` is the main 24 MB RAM buffer (indexed by physical offset).
    /// `aram` is the auxiliary 16 MB audio RAM buffer.
    ///
    /// Source/destination addresses are translated as follows:
    ///   0x80000000-0x817FFFFF → RAM (cached mirror)
    ///   0xC0000000-0xC17FFFFF → RAM (uncached mirror)
    ///   0x00000000-0x00FFFFFF → ARAM
    pub fn execute_transfer(&mut self, channel: usize, ram: &mut [u8], aram: &mut [u8]) {
        if channel >= self.channels.len() {
            return;
        }
        let ch = &self.channels[channel];
        if !ch.active.load(Ordering::SeqCst) {
            return;
        }

        let len = ch.length as usize;
        let src_addr = ch.source;
        let dst_addr = ch.destination;

        // Read source bytes into temporary buffer
        let mut buf = vec![0u8; len];
        Self::read_region(src_addr, &mut buf, ram, aram);

        // Write to destination
        Self::write_region(dst_addr, &buf, ram, aram);

        // Mark transfer complete and fire callback
        self.complete_transfer(channel);
    }

    fn region_slice_read<'a>(
        addr: u32,
        len: usize,
        ram: &'a [u8],
        aram: &'a [u8],
    ) -> Option<&'a [u8]> {
        match addr {
            0x80000000..=0x817FFFFF => {
                let off = (addr & 0x01FFFFFF) as usize;
                ram.get(off..off + len)
            }
            0xC0000000..=0xC17FFFFF => {
                let off = (addr & 0x01FFFFFF) as usize;
                ram.get(off..off + len)
            }
            _ if addr < 0x01000000 => {
                let off = addr as usize;
                aram.get(off..off + len)
            }
            _ => None,
        }
    }

    fn read_region(addr: u32, buf: &mut [u8], ram: &[u8], aram: &[u8]) {
        if let Some(src) = Self::region_slice_read(addr, buf.len(), ram, aram) {
            buf.copy_from_slice(src);
        }
    }

    fn write_region(addr: u32, buf: &[u8], ram: &mut [u8], aram: &mut [u8]) {
        let len = buf.len();
        match addr {
            0x80000000..=0x817FFFFF => {
                let off = (addr & 0x01FFFFFF) as usize;
                if let Some(dst) = ram.get_mut(off..off + len) {
                    dst.copy_from_slice(buf);
                }
            }
            0xC0000000..=0xC17FFFFF => {
                let off = (addr & 0x01FFFFFF) as usize;
                if let Some(dst) = ram.get_mut(off..off + len) {
                    dst.copy_from_slice(buf);
                }
            }
            _ if addr < 0x01000000 => {
                let off = addr as usize;
                if let Some(dst) = aram.get_mut(off..off + len) {
                    dst.copy_from_slice(buf);
                }
            }
            _ => {}
        }
    }

    pub fn complete_transfer(&mut self, channel: usize) {
        if channel < self.channels.len() {
            self.channels[channel].active.store(false, Ordering::SeqCst);
            if let Some(callback) = &self.channels[channel].callback {
                callback();
            }
        }
    }
}

impl DmaChannel {
    pub fn set_callback<F: Fn() + Send + Sync + 'static>(&mut self, callback: F) {
        self.callback = Some(Box::new(callback));
    }
}
