// DMA (Direct Memory Access) system
use crate::memory::{ARam, Ram, VRam};
use anyhow::{Context, Result};
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
    priority: u8, // Higher number = higher priority
    bytes_transferred: u32,
}

impl DmaSystem {
    pub fn new() -> Self {
        Self {
            channels: (0..4)
                .map(|i| DmaChannel {
                    active: Arc::new(AtomicBool::new(false)),
                    source: 0,
                    destination: 0,
                    length: 0,
                    callback: None,
                    priority: i as u8, // Channel 0 has highest priority
                    bytes_transferred: 0,
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

        if length == 0 {
            log::warn!("DMA transfer with zero length on channel {}", channel);
            return Ok(());
        }

        let ch = &mut self.channels[channel];
        ch.source = source;
        ch.destination = destination;
        ch.length = length;
        ch.bytes_transferred = 0;
        ch.active.store(true, Ordering::SeqCst);

        log::debug!(
            "DMA transfer started: channel {}, {} bytes from 0x{:08X} to 0x{:08X}",
            channel,
            length,
            source,
            destination
        );

        Ok(())
    }

    /// Set DMA channel priority
    pub fn set_priority(&mut self, channel: usize, priority: u8) {
        if channel < self.channels.len() {
            self.channels[channel].priority = priority;
        }
    }

    pub fn is_active(&self, channel: usize) -> bool {
        if channel < self.channels.len() {
            self.channels[channel].active.load(Ordering::SeqCst)
        } else {
            false
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

    /// Process active DMA transfers
    ///
    /// This method processes all active DMA transfers, copying data
    /// between memory regions (RAM, VRAM, ARAM).
    /// Transfers are processed in priority order (higher priority first).
    ///
    /// # Arguments
    /// * `ram` - Main RAM reference
    /// * `vram` - Video RAM reference
    /// * `aram` - Audio RAM reference
    pub fn process_transfers(
        &mut self,
        ram: &mut Ram,
        vram: &mut VRam,
        aram: &mut ARam,
    ) -> Result<()> {
        // Sort channels by priority (higher priority first)
        let mut active_channels: Vec<(usize, u8)> = self.channels
            .iter()
            .enumerate()
            .filter(|(_, ch)| ch.active.load(Ordering::SeqCst))
            .map(|(idx, ch)| (idx, ch.priority))
            .collect();
        
        active_channels.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by priority descending

        for (channel_idx, _) in active_channels {
            let channel = &mut self.channels[channel_idx];
            if !channel.active.load(Ordering::SeqCst) {
                continue;
            }

            let source = channel.source;
            let dest = channel.destination;
            let length = channel.length;
            
            // Handle alignment: GameCube DMA requires 32-byte alignment
            // For unaligned transfers, we'll handle them but log a warning
            if (source & 0x1F) != 0 || (dest & 0x1F) != 0 {
                log::warn!(
                    "DMA transfer with unaligned addresses: src=0x{:08X}, dst=0x{:08X}",
                    source,
                    dest
                );
            }
            
            // Check for overlapping regions (source and dest overlap)
            let src_end = source.wrapping_add(length);
            let dst_end = dest.wrapping_add(length);
            let overlaps = !(src_end <= dest || dst_end <= source);
            
            if overlaps && source != dest {
                // Handle overlapping transfer by using temporary buffer
                log::debug!("DMA transfer with overlapping regions, using temporary buffer");
            }

            // Determine source and destination regions
            let src_region = Self::get_memory_region(source);
            let dst_region = Self::get_memory_region(dest);

            // Perform transfer based on regions
            let transfer_result: Result<()> = match (src_region, dst_region) {
                (MemoryRegion::Ram(_), MemoryRegion::Ram(_)) => {
                    // RAM to RAM transfer
                    if overlaps && source != dest {
                        // Use temporary buffer for overlapping regions
                        let src_data = ram.read_bytes(source, length as usize)?;
                        ram.write_bytes(dest, &src_data)
                    } else {
                        // Direct transfer (non-overlapping or same address)
                        let src_data = ram.read_bytes(source, length as usize)?;
                        ram.write_bytes(dest, &src_data)
                    }
                }
                (MemoryRegion::Ram(_), MemoryRegion::VRam(_)) => {
                    // RAM to VRAM transfer
                    let src_data = ram.read_bytes(source, length as usize)?;
                    // VRAM address needs to be in VRAM space (0xCC000000)
                    let vram_addr = if dest >= 0xCC000000 {
                        dest
                    } else {
                        0xCC000000 | (dest & 0x001FFFFF)
                    };
                    vram.write_bytes(vram_addr, &src_data)
                }
                (MemoryRegion::Ram(_), MemoryRegion::ARam(_)) => {
                    // RAM to ARAM transfer
                    let src_data = ram.read_bytes(source, length as usize)?;
                    // ARAM address needs to be in ARAM space
                    let aram_addr = if dest >= 0xC0000000 {
                        dest
                    } else {
                        0xC0000000 | (dest & 0x00FFFFFF)
                    };
                    aram.write_bytes(aram_addr, &src_data)
                }
                (MemoryRegion::VRam(_), MemoryRegion::Ram(_)) => {
                    // VRAM to RAM transfer
                    let vram_addr = if source >= 0xCC000000 {
                        source
                    } else {
                        0xCC000000 | (source & 0x001FFFFF)
                    };
                    let src_data = vram.read_bytes(vram_addr, length as usize)?;
                    ram.write_bytes(dest, &src_data)
                }
                (MemoryRegion::VRam(_), MemoryRegion::VRam(_)) => {
                    // VRAM to VRAM transfer
                    let src_vram = if source >= 0xCC000000 {
                        source
                    } else {
                        0xCC000000 | (source & 0x001FFFFF)
                    };
                    let dst_vram = if dest >= 0xCC000000 {
                        dest
                    } else {
                        0xCC000000 | (dest & 0x001FFFFF)
                    };
                    let src_data = vram.read_bytes(src_vram, length as usize)?;
                    vram.write_bytes(dst_vram, &src_data)
                }
                (MemoryRegion::ARam(_), MemoryRegion::Ram(_)) => {
                    // ARAM to RAM transfer
                    let aram_addr = if source >= 0xC0000000 {
                        source
                    } else {
                        0xC0000000 | (source & 0x00FFFFFF)
                    };
                    let src_data = aram.read_bytes(aram_addr, length as usize)?;
                    ram.write_bytes(dest, &src_data)
                }
                (MemoryRegion::ARam(_), MemoryRegion::ARam(_)) => {
                    // ARAM to ARAM transfer
                    let src_aram = if source >= 0xC0000000 {
                        source
                    } else {
                        0xC0000000 | (source & 0x00FFFFFF)
                    };
                    let dst_aram = if dest >= 0xC0000000 {
                        dest
                    } else {
                        0xC0000000 | (dest & 0x00FFFFFF)
                    };
                    let src_data = aram.read_bytes(src_aram, length as usize)?;
                    aram.write_bytes(dst_aram, &src_data)
                }
                (MemoryRegion::ARam(_), MemoryRegion::VRam(_)) => {
                    // ARAM to VRAM transfer
                    let aram_addr = if source >= 0xC0000000 {
                        source
                    } else {
                        0xC0000000 | (source & 0x00FFFFFF)
                    };
                    let vram_addr = if dest >= 0xCC000000 {
                        dest
                    } else {
                        0xCC000000 | (dest & 0x001FFFFF)
                    };
                    let src_data = aram.read_bytes(aram_addr, length as usize)?;
                    vram.write_bytes(vram_addr, &src_data)
                }
                (MemoryRegion::VRam(_), MemoryRegion::ARam(_)) => {
                    // VRAM to ARAM transfer
                    let vram_addr = if source >= 0xCC000000 {
                        source
                    } else {
                        0xCC000000 | (source & 0x001FFFFF)
                    };
                    let aram_addr = if dest >= 0xC0000000 {
                        dest
                    } else {
                        0xC0000000 | (dest & 0x00FFFFFF)
                    };
                    let src_data = vram.read_bytes(vram_addr, length as usize)?;
                    aram.write_bytes(aram_addr, &src_data)
                }
                _ => {
                    log::warn!(
                        "Unsupported DMA transfer: 0x{:08X} -> 0x{:08X} ({} bytes)",
                        source,
                        dest,
                        length
                    );
                    Ok(())
                }
            };

            // Check transfer result
            match transfer_result {
                Ok(()) => {
                    channel.bytes_transferred = length;
                    // Complete the transfer
                    self.complete_transfer(channel_idx);
                    log::debug!(
                        "DMA transfer completed: channel {}, {} bytes from 0x{:08X} to 0x{:08X}",
                        channel_idx,
                        length,
                        source,
                        dest
                    );
                }
                Err(e) => {
                    log::error!(
                        "DMA transfer failed on channel {}: {} ({} bytes from 0x{:08X} to 0x{:08X})",
                        channel_idx,
                        e,
                        length,
                        source,
                        dest
                    );
                    // Mark channel as inactive on error
                    channel.active.store(false, Ordering::SeqCst);
                }
            }
        }

        Ok(())
    }

    /// Get memory region for an address
    fn get_memory_region(addr: u32) -> MemoryRegion {
        match addr {
            0x80000000..=0x817FFFFF => MemoryRegion::Ram(addr & 0x00FFFFFF),
            0xCC000000..=0xCC1FFFFF => MemoryRegion::VRam(addr & 0x001FFFFF),
            0x80000000..=0x80FFFFFF | 0xC0000000..=0xCFFFFFFF => MemoryRegion::ARam(addr & 0x00FFFFFF),
            _ => MemoryRegion::IO(addr),
        }
    }
}

/// Memory region type for DMA transfers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MemoryRegion {
    Ram(u32),
    VRam(u32),
    ARam(u32),
    IO(u32),
}

impl DmaChannel {
    pub fn set_callback<F: Fn() + Send + Sync + 'static>(&mut self, callback: F) {
        self.callback = Some(Box::new(callback));
    }
}
