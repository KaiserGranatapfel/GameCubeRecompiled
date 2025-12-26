//! Main RAM Simulation (24MB)
//!
//! This module provides the main RAM implementation for the GameCube recompiler runtime.
//! The GameCube has 24MB of main RAM, mapped to addresses 0x80000000-0x817FFFFF.
//!
//! # Memory Optimizations
//! - Removed redundant `size` field (can be derived from `data.len()`)
//! - All read/write functions use `#[inline]` for hot-path optimization
//! - Explicit bounds checks with early returns for better branch prediction
//!
//! # Address Translation
//! GameCube uses 24-bit addressing for RAM (addresses 0x00000000-0x00FFFFFF).
//! The upper 8 bits are masked off to get the physical RAM offset.

use anyhow::Result;

/// Main RAM implementation (24MB).
///
/// # Memory Layout
/// - `data`: 24MB byte array (heap allocation required for large size)
/// - No redundant `size` field (derived from `data.len()`)
///
/// # Address Space
/// GameCube RAM is mapped to:
/// - Physical addresses: 0x00000000-0x00FFFFFF (24MB)
/// - Virtual addresses: 0x80000000-0x817FFFFF (24MB)
#[derive(Debug)]
pub struct Ram {
    /// RAM data (24MB)
    data: Vec<u8>,
}

impl Ram {
    /// Create a new RAM instance with 24MB of memory.
    ///
    /// # Returns
    /// `Ram` - Initialized RAM instance with all bytes set to 0
    ///
    /// # Examples
    /// ```rust
    /// let ram = Ram::new();
    /// ```
    #[inline] // Constructor - simple, may be inlined
    pub fn new() -> Self {
        const RAM_SIZE: usize = 24usize * 1024usize * 1024usize; // 24MB
        Self {
            data: vec![0u8; RAM_SIZE],
        }
    }
    
    /// Read a single byte from RAM.
    ///
    /// # Arguments
    /// * `address` - 32-bit address (24-bit addressing, upper 8 bits masked)
    ///
    /// # Returns
    /// `Result<u8>` - Byte value at address, or error if out of bounds
    ///
    /// # Errors
    /// Returns error if address is out of bounds (>= 24MB)
    ///
    /// # Examples
    /// ```rust
    /// let value = ram.read_u8(0x80000000)?;
    /// ```
    #[inline(always)] // Hot path - always inline for performance
    pub fn read_u8(&self, address: u32) -> Result<u8> {
        let addr: usize = (address & 0x00FFFFFFu32) as usize; // 24-bit addressing
        if addr < self.data.len() {
            Ok(self.data[addr])
        } else {
            anyhow::bail!("RAM read out of bounds: 0x{:08X}", address);
        }
    }
    
    /// Read a 16-bit word (big-endian) from RAM.
    ///
    /// # Arguments
    /// * `address` - 32-bit address (must be aligned, but we don't enforce it)
    ///
    /// # Returns
    /// `Result<u16>` - 16-bit value at address, or error if out of bounds
    ///
    /// # Errors
    /// Returns error if address+1 is out of bounds
    ///
    /// # Examples
    /// ```rust
    /// let value = ram.read_u16(0x80000000)?;
    /// ```
    #[inline] // Hot path - may be inlined
    pub fn read_u16(&self, address: u32) -> Result<u16> {
        let low: u8 = self.read_u8(address)?;
        let high: u8 = self.read_u8(address.wrapping_add(1u32))?;
        Ok(u16::from_be_bytes([high, low]))
    }
    
    /// Read a 32-bit word (big-endian) from RAM.
    ///
    /// # Arguments
    /// * `address` - 32-bit address (must be aligned, but we don't enforce it)
    ///
    /// # Returns
    /// `Result<u32>` - 32-bit value at address, or error if out of bounds
    ///
    /// # Errors
    /// Returns error if address+3 is out of bounds
    ///
    /// # Examples
    /// ```rust
    /// let value = ram.read_u32(0x80000000)?;
    /// ```
    #[inline] // Hot path - may be inlined
    pub fn read_u32(&self, address: u32) -> Result<u32> {
        let bytes: [u8; 4] = [
            self.read_u8(address)?,
            self.read_u8(address.wrapping_add(1u32))?,
            self.read_u8(address.wrapping_add(2u32))?,
            self.read_u8(address.wrapping_add(3u32))?,
        ];
        Ok(u32::from_be_bytes(bytes))
    }
    
    /// Write a single byte to RAM.
    ///
    /// # Arguments
    /// * `address` - 32-bit address (24-bit addressing, upper 8 bits masked)
    /// * `value` - Byte value to write
    ///
    /// # Returns
    /// `Result<()>` - Success, or error if out of bounds
    ///
    /// # Errors
    /// Returns error if address is out of bounds (>= 24MB)
    ///
    /// # Examples
    /// ```rust
    /// ram.write_u8(0x80000000, 0x42)?;
    /// ```
    #[inline(always)] // Hot path - always inline for performance
    pub fn write_u8(&mut self, address: u32, value: u8) -> Result<()> {
        let addr: usize = (address & 0x00FFFFFFu32) as usize;
        if addr < self.data.len() {
            self.data[addr] = value;
            Ok(())
        } else {
            anyhow::bail!("RAM write out of bounds: 0x{:08X}", address);
        }
    }
    
    /// Write a 16-bit word (big-endian) to RAM.
    ///
    /// # Arguments
    /// * `address` - 32-bit address (must be aligned, but we don't enforce it)
    /// * `value` - 16-bit value to write
    ///
    /// # Returns
    /// `Result<()>` - Success, or error if out of bounds
    ///
    /// # Errors
    /// Returns error if address+1 is out of bounds
    ///
    /// # Examples
    /// ```rust
    /// ram.write_u16(0x80000000, 0x1234)?;
    /// ```
    #[inline] // Hot path - may be inlined
    pub fn write_u16(&mut self, address: u32, value: u16) -> Result<()> {
        let bytes: [u8; 2] = value.to_be_bytes();
        self.write_u8(address, bytes[0])?;
        self.write_u8(address.wrapping_add(1u32), bytes[1])?;
        Ok(())
    }
    
    /// Write a 32-bit word (big-endian) to RAM.
    ///
    /// # Arguments
    /// * `address` - 32-bit address (must be aligned, but we don't enforce it)
    /// * `value` - 32-bit value to write
    ///
    /// # Returns
    /// `Result<()>` - Success, or error if out of bounds
    ///
    /// # Errors
    /// Returns error if address+3 is out of bounds
    ///
    /// # Examples
    /// ```rust
    /// ram.write_u32(0x80000000, 0x12345678)?;
    /// ```
    #[inline] // Hot path - may be inlined
    pub fn write_u32(&mut self, address: u32, value: u32) -> Result<()> {
        let bytes: [u8; 4] = value.to_be_bytes();
        self.write_u8(address, bytes[0])?;
        self.write_u8(address.wrapping_add(1u32), bytes[1])?;
        self.write_u8(address.wrapping_add(2u32), bytes[2])?;
        self.write_u8(address.wrapping_add(3u32), bytes[3])?;
        Ok(())
    }
    
    /// Read multiple bytes from RAM.
    ///
    /// # Arguments
    /// * `address` - 32-bit address (24-bit addressing, upper 8 bits masked)
    /// * `len` - Number of bytes to read
    ///
    /// # Returns
    /// `Result<Vec<u8>>` - Byte vector, or error if out of bounds
    ///
    /// # Errors
    /// Returns error if address+len is out of bounds
    ///
    /// # Examples
    /// ```rust
    /// let data = ram.read_bytes(0x80000000, 1024)?;
    /// ```
    #[inline] // May be inlined for small lengths
    pub fn read_bytes(&self, address: u32, len: usize) -> Result<Vec<u8>> {
        let addr: usize = (address & 0x00FFFFFFu32) as usize;
        if addr.wrapping_add(len) <= self.data.len() {
            Ok(self.data[addr..addr.wrapping_add(len)].to_vec())
        } else {
            anyhow::bail!("RAM read out of bounds: 0x{:08X} len {}", address, len);
        }
    }
    
    /// Write multiple bytes to RAM.
    ///
    /// # Arguments
    /// * `address` - 32-bit address (24-bit addressing, upper 8 bits masked)
    /// * `data` - Byte slice to write
    ///
    /// # Returns
    /// `Result<()>` - Success, or error if out of bounds
    ///
    /// # Errors
    /// Returns error if address+data.len() is out of bounds
    ///
    /// # Examples
    /// ```rust
    /// ram.write_bytes(0x80000000, &[0x42, 0x43, 0x44])?;
    /// ```
    #[inline] // May be inlined for small lengths
    pub fn write_bytes(&mut self, address: u32, data: &[u8]) -> Result<()> {
        let addr: usize = (address & 0x00FFFFFFu32) as usize;
        if addr.wrapping_add(data.len()) <= self.data.len() {
            self.data[addr..addr.wrapping_add(data.len())].copy_from_slice(data);
            Ok(())
        } else {
            anyhow::bail!("RAM write out of bounds: 0x{:08X} len {}", address, data.len());
        }
    }
}

impl Default for Ram {
    #[inline] // Simple default implementation
    fn default() -> Self {
        Self::new()
    }
}
