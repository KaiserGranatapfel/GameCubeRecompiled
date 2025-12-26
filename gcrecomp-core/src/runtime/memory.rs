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
        Self {
            ram: vec![0u8; RAM_SIZE],
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
        // GameCube uses a flat memory model with physical addresses
        // Main RAM is at 0x80000000 - 0x817FFFFF
        if address >= 0x80000000u32 && address < 0x81800000u32 {
            Some((address.wrapping_sub(0x80000000u32)) as usize)
        } else {
            None
        }
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
        let offset: usize = self
            .translate_address(address)
            .context("Invalid memory address")?;
        Ok(self.ram[offset])
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
        let offset: usize = self
            .translate_address(address)
            .context("Invalid memory address")?;
        if offset.wrapping_add(1usize) >= self.ram.len() {
            anyhow::bail!("Memory read out of bounds");
        }
        let bytes: [u8; 2] = [self.ram[offset], self.ram[offset.wrapping_add(1usize)]];
        Ok(u16::from_be_bytes(bytes))
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
        let offset: usize = self
            .translate_address(address)
            .context("Invalid memory address")?;
        if offset.wrapping_add(3usize) >= self.ram.len() {
            anyhow::bail!("Memory read out of bounds");
        }
        let bytes: [u8; 4] = [
            self.ram[offset],
            self.ram[offset.wrapping_add(1usize)],
            self.ram[offset.wrapping_add(2usize)],
            self.ram[offset.wrapping_add(3usize)],
        ];
        Ok(u32::from_be_bytes(bytes))
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
        let offset: usize = self
            .translate_address(address)
            .context("Invalid memory address")?;
        if offset.wrapping_add(7usize) >= self.ram.len() {
            anyhow::bail!("Memory read out of bounds");
        }
        let bytes: [u8; 8] = [
            self.ram[offset],
            self.ram[offset.wrapping_add(1usize)],
            self.ram[offset.wrapping_add(2usize)],
            self.ram[offset.wrapping_add(3usize)],
            self.ram[offset.wrapping_add(4usize)],
            self.ram[offset.wrapping_add(5usize)],
            self.ram[offset.wrapping_add(6usize)],
            self.ram[offset.wrapping_add(7usize)],
        ];
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
        let offset: usize = self
            .translate_address(address)
            .context("Invalid memory address")?;
        self.ram[offset] = value;
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
        let offset: usize = self
            .translate_address(address)
            .context("Invalid memory address")?;
        if offset.wrapping_add(1usize) >= self.ram.len() {
            anyhow::bail!("Memory write out of bounds");
        }
        let bytes: [u8; 2] = value.to_be_bytes();
        self.ram[offset] = bytes[0];
        self.ram[offset.wrapping_add(1usize)] = bytes[1];
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
        let offset: usize = self
            .translate_address(address)
            .context("Invalid memory address")?;
        if offset.wrapping_add(3usize) >= self.ram.len() {
            anyhow::bail!("Memory write out of bounds");
        }
        let bytes: [u8; 4] = value.to_be_bytes();
        self.ram[offset] = bytes[0];
        self.ram[offset.wrapping_add(1usize)] = bytes[1];
        self.ram[offset.wrapping_add(2usize)] = bytes[2];
        self.ram[offset.wrapping_add(3usize)] = bytes[3];
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
        let offset: usize = self
            .translate_address(address)
            .context("Invalid memory address")?;
        if offset.wrapping_add(7usize) >= self.ram.len() {
            anyhow::bail!("Memory write out of bounds");
        }
        let bytes: [u8; 8] = value.to_be_bytes();
        for (i, byte) in bytes.iter().enumerate() {
            self.ram[offset.wrapping_add(i)] = *byte;
        }
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
        let dest_offset: usize = self.translate_address(dest)
            .context("Invalid destination address")?;
        let src_offset: usize = self.translate_address(src)
            .context("Invalid source address")?;
        
        if dest_offset.wrapping_add(len) > self.ram.len() || src_offset.wrapping_add(len) > self.ram.len() {
            anyhow::bail!("Bulk copy out of bounds");
        }
        
        // Use optimized copy if ranges don't overlap
        if dest_offset < src_offset || dest_offset >= src_offset.wrapping_add(len) {
            self.ram[dest_offset..dest_offset.wrapping_add(len)]
                .copy_from_slice(&self.ram[src_offset..src_offset.wrapping_add(len)]);
        } else {
            // Handle overlapping ranges (copy backwards or use temporary buffer)
            let temp: Vec<u8> = self.ram[src_offset..src_offset.wrapping_add(len)].to_vec();
            self.ram[dest_offset..dest_offset.wrapping_add(len)].copy_from_slice(&temp);
        }
        
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
        let offset: usize = self.translate_address(address)
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
