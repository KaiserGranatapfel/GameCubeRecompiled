//! Memory Manager
//!
//! This module provides memory management for the GameCube recompiler runtime.
//! It handles address translation, memory reads/writes, and bulk operations.
//!
//! # Memory Map
//! GameCube uses a flat memory model with the following regions:
//! - **0x80000000 - 0x817FFFFF**: Main RAM (24MB)
//! - **0xCC000000 - 0xCFFFFFFF**: ARAM (16MB audio RAM)
//! - **0x80000000 - 0x807FFFFF**: Locked cache (8MB, overlaps with main RAM)
//!
//! # Memory Optimizations
//! - All hot-path functions use `#[inline(always)]` for address translation
//! - Read/write functions use `#[inline]` for performance
//! - Explicit type annotations to reduce compiler inference overhead
//! - Bulk operations use optimized copy_from_slice for non-overlapping ranges
//!
//! # Address Translation
//! GameCube uses physical addresses directly. Main RAM is mapped at 0x80000000,
//! so we subtract this base address to get the RAM offset.

use anyhow::{Context, Result};

/// Memory manager for GameCube memory operations.
///
/// # Memory Layout
/// - `ram`: 24MB byte array (heap allocation required for large size)
///
/// # Address Translation
/// GameCube main RAM is mapped to virtual addresses 0x80000000-0x817FFFFF.
/// Physical addresses are computed by subtracting the base address (0x80000000).
#[derive(Debug)]
pub struct MemoryManager {
    /// Main RAM (24MB)
    ram: Vec<u8>,
    /// I/O registers (hardware register space: 0xCC000000-0xCC00FFFF)
    io_regs: Vec<u8>,
}

impl MemoryManager {
    /// Create a new memory manager with 24MB of RAM.
    ///
    /// # Returns
    /// `MemoryManager` - Initialized memory manager with all bytes set to 0
    ///
    /// # Examples
    /// ```rust
    /// let mut memory = MemoryManager::new();
    /// ```
    #[inline] // Constructor - simple, may be inlined
    pub fn new() -> Self {
        // 24MB RAM model
        const RAM_SIZE: usize = 24usize * 1024usize * 1024usize; // 24MB
        const IO_SIZE: usize = 0x10000usize; // 64KB I/O register space
        Self {
            ram: vec![0u8; RAM_SIZE],
            io_regs: vec![0u8; IO_SIZE],
        }
    }

    /// Translate a virtual address to a physical RAM offset.
    ///
    /// # Algorithm
    /// GameCube uses a flat memory model with physical addresses.
    /// Main RAM is at 0x80000000 - 0x817FFFFF (24MB).
    /// Physical offset = virtual_address - 0x80000000
    ///
    /// # Arguments
    /// * `address` - 32-bit virtual address
    ///
    /// # Returns
    /// `Option<usize>` - Physical RAM offset if address is in main RAM, None otherwise
    ///
    /// # Examples
    /// ```rust
    /// let offset = memory.translate_address(0x80000000);
    /// assert_eq!(offset, Some(0));
    /// ```
    #[inline(always)] // Hot path - always inline for performance
    fn translate_address(&self, address: u32) -> Option<usize> {
        match address {
            // Main RAM: 0x80000000 - 0x817FFFFF (cached)
            0x80000000..=0x817FFFFF => Some((address.wrapping_sub(0x80000000u32)) as usize),
            // Uncached RAM mirror: 0xC0000000 - 0xC17FFFFF → same physical RAM
            0xC0000000..=0xC17FFFFF => Some((address.wrapping_sub(0xC0000000u32)) as usize),
            _ => None,
        }
    }

    /// Resolve an address to its backing buffer and offset, covering both main RAM
    /// and the hardware I/O register window (0xCC000000-0xCC00FFFF). This is what
    /// makes recompiled writes to VI/DSP/DI/etc. registers actually land somewhere.
    #[inline(always)]
    fn region(&self, address: u32) -> Option<(&[u8], usize)> {
        if let Some(off) = self.translate_address(address) {
            Some((&self.ram, off))
        } else if (0xCC000000..=0xCC00FFFF).contains(&address) {
            Some((&self.io_regs, (address - 0xCC000000) as usize))
        } else {
            None
        }
    }

    #[inline(always)]
    fn region_mut(&mut self, address: u32) -> Option<(&mut [u8], usize)> {
        if let Some(off) = self.translate_address(address) {
            Some((&mut self.ram, off))
        } else if (0xCC000000..=0xCC00FFFF).contains(&address) {
            Some((&mut self.io_regs, (address - 0xCC000000) as usize))
        } else {
            None
        }
    }

    /// Read a byte from I/O register space (0xCC000000-0xCC00FFFF).
    #[inline]
    pub fn read_io_u8(&self, address: u32) -> Result<u8> {
        let offset = (address.wrapping_sub(0xCC000000u32)) as usize;
        if offset < self.io_regs.len() {
            Ok(self.io_regs[offset])
        } else {
            anyhow::bail!("I/O register read out of bounds: 0x{:08X}", address);
        }
    }

    /// Write a byte to I/O register space.
    #[inline]
    pub fn write_io_u8(&mut self, address: u32, value: u8) -> Result<()> {
        let offset = (address.wrapping_sub(0xCC000000u32)) as usize;
        if offset < self.io_regs.len() {
            self.io_regs[offset] = value;
            Ok(())
        } else {
            anyhow::bail!("I/O register write out of bounds: 0x{:08X}", address);
        }
    }

    /// Read a 32-bit value from I/O register space.
    #[inline]
    pub fn read_io_u32(&self, address: u32) -> Result<u32> {
        let offset = (address.wrapping_sub(0xCC000000u32)) as usize;
        if offset + 3 < self.io_regs.len() {
            let bytes: [u8; 4] = [
                self.io_regs[offset],
                self.io_regs[offset + 1],
                self.io_regs[offset + 2],
                self.io_regs[offset + 3],
            ];
            Ok(u32::from_be_bytes(bytes))
        } else {
            anyhow::bail!("I/O register read out of bounds: 0x{:08X}", address);
        }
    }

    /// Write a 32-bit value to I/O register space.
    #[inline]
    pub fn write_io_u32(&mut self, address: u32, value: u32) -> Result<()> {
        let offset = (address.wrapping_sub(0xCC000000u32)) as usize;
        if offset + 3 < self.io_regs.len() {
            let bytes = value.to_be_bytes();
            self.io_regs[offset..offset + 4].copy_from_slice(&bytes);
            Ok(())
        } else {
            anyhow::bail!("I/O register write out of bounds: 0x{:08X}", address);
        }
    }

    /// Get raw RAM reference for direct access (e.g. texture decoding).
    pub fn ram_slice(&self) -> &[u8] {
        &self.ram
    }

    /// Read a single byte from memory.
    ///
    /// # Arguments
    /// * `address` - 32-bit virtual address
    ///
    /// # Returns
    /// `Result<u8>` - Byte value at address, or error if invalid/out of bounds
    ///
    /// # Errors
    /// Returns error if address is not in main RAM or out of bounds
    ///
    /// # Examples
    /// ```rust
    /// let value = memory.read_u8(0x80000000)?;
    /// ```
    #[inline] // Hot path - may be inlined
    pub fn read_u8(&self, address: u32) -> Result<u8> {
        let (buf, off) = self.region(address).context("Invalid memory address")?;
        buf.get(off).copied().context("Memory read out of bounds")
    }

    /// Read a 16-bit word (big-endian) from memory.
    ///
    /// # Arguments
    /// * `address` - 32-bit virtual address (must be aligned, but we don't enforce it)
    ///
    /// # Returns
    /// `Result<u16>` - 16-bit value at address, or error if invalid/out of bounds
    ///
    /// # Errors
    /// Returns error if address+1 is out of bounds
    ///
    /// # Examples
    /// ```rust
    /// let value = memory.read_u16(0x80000000)?;
    /// ```
    #[inline] // Hot path - may be inlined
    pub fn read_u16(&self, address: u32) -> Result<u16> {
        let (buf, off) = self.region(address).context("Invalid memory address")?;
        if off + 2 > buf.len() {
            anyhow::bail!("Memory read out of bounds");
        }
        Ok(u16::from_be_bytes([buf[off], buf[off + 1]]))
    }

    /// Read a 32-bit word (big-endian) from memory.
    ///
    /// # Arguments
    /// * `address` - 32-bit virtual address (must be aligned, but we don't enforce it)
    ///
    /// # Returns
    /// `Result<u32>` - 32-bit value at address, or error if invalid/out of bounds
    ///
    /// # Errors
    /// Returns error if address+3 is out of bounds
    ///
    /// # Examples
    /// ```rust
    /// let value = memory.read_u32(0x80000000)?;
    /// ```
    #[inline] // Hot path - may be inlined
    pub fn read_u32(&self, address: u32) -> Result<u32> {
        let (buf, off) = self.region(address).context("Invalid memory address")?;
        if off + 4 > buf.len() {
            anyhow::bail!("Memory read out of bounds");
        }
        Ok(u32::from_be_bytes([
            buf[off],
            buf[off + 1],
            buf[off + 2],
            buf[off + 3],
        ]))
    }

    /// Read a 64-bit word (big-endian) from memory.
    ///
    /// # Arguments
    /// * `address` - 32-bit virtual address (must be aligned, but we don't enforce it)
    ///
    /// # Returns
    /// `Result<u64>` - 64-bit value at address, or error if invalid/out of bounds
    ///
    /// # Errors
    /// Returns error if address+7 is out of bounds
    ///
    /// # Examples
    /// ```rust
    /// let value = memory.read_u64(0x80000000)?;
    /// ```
    #[inline] // Hot path - may be inlined
    pub fn read_u64(&self, address: u32) -> Result<u64> {
        let (buf, off) = self.region(address).context("Invalid memory address")?;
        if off + 8 > buf.len() {
            anyhow::bail!("Memory read out of bounds");
        }
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&buf[off..off + 8]);
        Ok(u64::from_be_bytes(bytes))
    }

    /// Write a single byte to memory.
    ///
    /// # Arguments
    /// * `address` - 32-bit virtual address
    /// * `value` - Byte value to write
    ///
    /// # Returns
    /// `Result<()>` - Success, or error if invalid/out of bounds
    ///
    /// # Errors
    /// Returns error if address is not in main RAM or out of bounds
    ///
    /// # Examples
    /// ```rust
    /// memory.write_u8(0x80000000, 0x42)?;
    /// ```
    #[inline] // Hot path - may be inlined
    pub fn write_u8(&mut self, address: u32, value: u8) -> Result<()> {
        let (buf, off) = self.region_mut(address).context("Invalid memory address")?;
        *buf.get_mut(off).context("Memory write out of bounds")? = value;
        Ok(())
    }

    /// Write a 16-bit word (big-endian) to memory.
    ///
    /// # Arguments
    /// * `address` - 32-bit virtual address (must be aligned, but we don't enforce it)
    /// * `value` - 16-bit value to write
    ///
    /// # Returns
    /// `Result<()>` - Success, or error if invalid/out of bounds
    ///
    /// # Errors
    /// Returns error if address+1 is out of bounds
    ///
    /// # Examples
    /// ```rust
    /// memory.write_u16(0x80000000, 0x1234)?;
    /// ```
    #[inline] // Hot path - may be inlined
    pub fn write_u16(&mut self, address: u32, value: u16) -> Result<()> {
        let (buf, off) = self.region_mut(address).context("Invalid memory address")?;
        if off + 2 > buf.len() {
            anyhow::bail!("Memory write out of bounds");
        }
        buf[off..off + 2].copy_from_slice(&value.to_be_bytes());
        Ok(())
    }

    /// Write a 32-bit word (big-endian) to memory.
    ///
    /// # Arguments
    /// * `address` - 32-bit virtual address (must be aligned, but we don't enforce it)
    /// * `value` - 32-bit value to write
    ///
    /// # Returns
    /// `Result<()>` - Success, or error if invalid/out of bounds
    ///
    /// # Errors
    /// Returns error if address+3 is out of bounds
    ///
    /// # Examples
    /// ```rust
    /// memory.write_u32(0x80000000, 0x12345678)?;
    /// ```
    #[inline] // Hot path - may be inlined
    pub fn write_u32(&mut self, address: u32, value: u32) -> Result<()> {
        let (buf, off) = self.region_mut(address).context("Invalid memory address")?;
        if off + 4 > buf.len() {
            anyhow::bail!("Memory write out of bounds");
        }
        buf[off..off + 4].copy_from_slice(&value.to_be_bytes());
        Ok(())
    }

    /// Write a 64-bit word (big-endian) to memory.
    ///
    /// # Arguments
    /// * `address` - 32-bit virtual address (must be aligned, but we don't enforce it)
    /// * `value` - 64-bit value to write
    ///
    /// # Returns
    /// `Result<()>` - Success, or error if invalid/out of bounds
    ///
    /// # Errors
    /// Returns error if address+7 is out of bounds
    ///
    /// # Examples
    /// ```rust
    /// memory.write_u64(0x80000000, 0x1234567890ABCDEF)?;
    /// ```
    #[inline] // Hot path - may be inlined
    pub fn write_u64(&mut self, address: u32, value: u64) -> Result<()> {
        let (buf, off) = self.region_mut(address).context("Invalid memory address")?;
        if off + 8 > buf.len() {
            anyhow::bail!("Memory write out of bounds");
        }
        buf[off..off + 8].copy_from_slice(&value.to_be_bytes());
        Ok(())
    }

    /// Read multiple bytes from memory.
    ///
    /// # Arguments
    /// * `address` - 32-bit virtual address
    /// * `len` - Number of bytes to read
    ///
    /// # Returns
    /// `Result<Vec<u8>>` - Byte vector, or error if invalid/out of bounds
    ///
    /// # Errors
    /// Returns error if address+len is out of bounds
    ///
    /// # Examples
    /// ```rust
    /// let data = memory.read_bytes(0x80000000, 1024)?;
    /// ```
    #[inline] // May be inlined for small lengths
    pub fn read_bytes(&self, address: u32, len: usize) -> Result<Vec<u8>> {
        let offset: usize = self
            .translate_address(address)
            .context("Invalid memory address")?;
        if offset.wrapping_add(len) > self.ram.len() {
            anyhow::bail!("Memory read out of bounds");
        }
        Ok(self.ram[offset..offset.wrapping_add(len)].to_vec())
    }

    /// Write multiple bytes to memory.
    ///
    /// # Arguments
    /// * `address` - 32-bit virtual address
    /// * `data` - Byte slice to write
    ///
    /// # Returns
    /// `Result<()>` - Success, or error if invalid/out of bounds
    ///
    /// # Errors
    /// Returns error if address+data.len() is out of bounds
    ///
    /// # Examples
    /// ```rust
    /// memory.write_bytes(0x80000000, &[0x42, 0x43, 0x44])?;
    /// ```
    #[inline] // May be inlined for small lengths
    pub fn write_bytes(&mut self, address: u32, data: &[u8]) -> Result<()> {
        let offset: usize = self
            .translate_address(address)
            .context("Invalid memory address")?;
        if offset.wrapping_add(data.len()) > self.ram.len() {
            anyhow::bail!("Memory write out of bounds");
        }
        self.ram[offset..offset.wrapping_add(data.len())].copy_from_slice(data);
        Ok(())
    }

    /// Load a section of data into memory (convenience wrapper for write_bytes).
    ///
    /// # Arguments
    /// * `address` - 32-bit virtual address
    /// * `data` - Byte slice to write
    ///
    /// # Returns
    /// `Result<()>` - Success, or error if invalid/out of bounds
    ///
    /// # Examples
    /// ```rust
    /// memory.load_section(0x80000000, &section_data)?;
    /// ```
    #[inline] // Simple wrapper - may be inlined
    pub fn load_section(&mut self, address: u32, data: &[u8]) -> Result<()> {
        self.write_bytes(address, data)
    }

    /// Optimized bulk memory copy.
    ///
    /// # Algorithm
    /// Copies `len` bytes from `src` to `dest`. Uses optimized `copy_from_slice`
    /// for non-overlapping ranges. For overlapping ranges, uses temporary buffer
    /// to ensure correct copy semantics.
    ///
    /// # Arguments
    /// * `dest` - Destination address
    /// * `src` - Source address
    /// * `len` - Number of bytes to copy
    ///
    /// # Returns
    /// `Result<()>` - Success, or error if invalid/out of bounds
    ///
    /// # Errors
    /// Returns error if either address is invalid or copy would go out of bounds
    ///
    /// # Examples
    /// ```rust
    /// memory.bulk_copy(0x80001000, 0x80000000, 1024)?;
    /// ```
    #[inline] // May be inlined for small lengths
    pub fn bulk_copy(&mut self, dest: u32, src: u32, len: usize) -> Result<()> {
        let dest_offset: usize = self
            .translate_address(dest)
            .context("Invalid destination address")?;
        let src_offset: usize = self
            .translate_address(src)
            .context("Invalid source address")?;

        if dest_offset.wrapping_add(len) > self.ram.len()
            || src_offset.wrapping_add(len) > self.ram.len()
        {
            anyhow::bail!("Bulk copy out of bounds");
        }

        // Always use temporary buffer to avoid borrow checker issues with overlapping slices
        let temp: Vec<u8> = self.ram[src_offset..src_offset.wrapping_add(len)].to_vec();
        self.ram[dest_offset..dest_offset.wrapping_add(len)].copy_from_slice(&temp);

        Ok(())
    }

    /// Get a read-only slice of memory.
    ///
    /// # Safety
    /// This function is safe but returns a reference to internal memory.
    /// The caller must ensure the slice is not used after the MemoryManager is dropped.
    ///
    /// # Arguments
    /// * `address` - 32-bit virtual address
    /// * `len` - Length of slice
    ///
    /// # Returns
    /// `Result<&[u8]>` - Byte slice, or error if invalid/out of bounds
    ///
    /// # Errors
    /// Returns error if address+len is out of bounds
    ///
    /// # Examples
    /// ```rust
    /// let slice = memory.get_slice(0x80000000, 1024)?;
    /// ```
    #[inline] // May be inlined for small lengths
    pub fn get_slice(&self, address: u32, len: usize) -> Result<&[u8]> {
        let offset: usize = self
            .translate_address(address)
            .context("Invalid memory address")?;
        if offset.wrapping_add(len) > self.ram.len() {
            anyhow::bail!("Memory slice out of bounds");
        }
        Ok(&self.ram[offset..offset.wrapping_add(len)])
    }
}

impl Default for MemoryManager {
    #[inline] // Simple default implementation
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ram_roundtrip() {
        let mut m = MemoryManager::new();
        m.write_u32(0x8000_1000, 0xDEAD_BEEF).unwrap();
        assert_eq!(m.read_u32(0x8000_1000).unwrap(), 0xDEAD_BEEF);
        // Uncached mirror sees the same RAM.
        assert_eq!(m.read_u32(0xC000_1000).unwrap(), 0xDEAD_BEEF);
    }

    #[test]
    fn hardware_registers_route_to_io_space() {
        // 0xCC002000 is the VI register block. Before the region() fix these
        // writes returned Err and were dropped; now they persist and read back.
        let mut m = MemoryManager::new();
        m.write_u32(0xCC00_201C, 0x0123_4567).unwrap(); // VI_TFBL (XFB address)
        assert_eq!(m.read_u32(0xCC00_201C).unwrap(), 0x0123_4567);
        m.write_u16(0xCC00_2002, 0xABCD).unwrap();
        assert_eq!(m.read_u16(0xCC00_2002).unwrap(), 0xABCD);
        m.write_u8(0xCC00_3000, 0x42).unwrap(); // DSP region
        assert_eq!(m.read_u8(0xCC00_3000).unwrap(), 0x42);
    }
}
