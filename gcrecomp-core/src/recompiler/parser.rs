//! DOL File Parser
//!
//! This module provides parsing for GameCube DOL (Dolphin) executable files.
//! The DOL format is the standard executable format for GameCube games.
//!
//! # DOL File Format
//! The DOL file format consists of:
//! - **Text sections**: Executable code sections (up to 7 sections)
//! - **Data sections**: Data sections (up to 11 sections)
//! - **BSS section**: Uninitialized data section (address and size)
//! - **Entry point**: Program entry point address
//!
//! # Memory Optimizations
//! - Uses const generics for fixed-size arrays (text/data section arrays)
//! - Pre-allocates vectors with known capacity
//! - Efficient byte reading with explicit buffer management

use anyhow::{Context, Result};
use std::io::{Cursor, Read};

/// DOL file structure.
///
/// Represents a parsed GameCube DOL executable file with all sections loaded.
#[derive(Debug, Clone)]
pub struct DolFile {
    /// Text (executable) sections
    pub text_sections: Vec<Section>,
    /// Data sections
    pub data_sections: Vec<Section>,
    /// BSS section address (uninitialized data)
    pub bss_address: u32,
    /// BSS section size
    pub bss_size: u32,
    /// Program entry point address
    pub entry_point: u32,
    /// File path (for reference)
    pub path: String,
}

/// DOL file section.
///
/// Represents a single section (text or data) in a DOL file.
#[derive(Debug, Clone)]
pub struct Section {
    /// Section offset in DOL file
    pub offset: u32,
    /// Section load address in memory
    pub address: u32,
    /// Section size in bytes
    pub size: u32,
    /// Section data
    pub data: Vec<u8>,
    /// Whether this section is executable (text section)
    pub executable: bool,
}

impl DolFile {
    /// Parse a DOL file from byte data.
    ///
    /// # Algorithm
    /// 1. Read text section offsets, addresses, and sizes (7 sections)
    /// 2. Read data section offsets, addresses, and sizes (11 sections)
    /// 3. Read BSS address and size
    /// 4. Read entry point
    /// 5. Load section data from file
    ///
    /// # Arguments
    /// * `data` - DOL file byte data
    /// * `path` - File path (for reference)
    ///
    /// # Returns
    /// `Result<DolFile>` - Parsed DOL file structure
    ///
    /// # Errors
    /// Returns error if DOL file is malformed or too small
    ///
    /// # Examples
    /// ```rust
    /// let dol_data = std::fs::read("game.dol")?;
    /// let dol_file = DolFile::parse(&dol_data, "game.dol")?;
    /// ```
    #[inline(never)] // Large function - don't inline
    pub fn parse(data: &[u8], path: &str) -> Result<Self> {
        const MIN_DOL_SIZE: usize = 0x100usize;
        if data.len() < MIN_DOL_SIZE {
            anyhow::bail!(
                "DOL file too small: {} bytes (minimum {} bytes)",
                data.len(),
                MIN_DOL_SIZE
            );
        }

        let mut cursor: Cursor<&[u8]> = Cursor::new(data);

        // Read text section offsets (7 sections, 4 bytes each)
        const NUM_TEXT_SECTIONS: usize = 7usize;
        let mut text_offsets: [u32; NUM_TEXT_SECTIONS] = [0u32; NUM_TEXT_SECTIONS];
        for offset in text_offsets.iter_mut() {
            *offset = read_u32_be(&mut cursor)?;
        }

        // Read data section offsets (11 sections, 4 bytes each)
        const NUM_DATA_SECTIONS: usize = 11usize;
        let mut data_offsets: [u32; NUM_DATA_SECTIONS] = [0u32; NUM_DATA_SECTIONS];
        for offset in data_offsets.iter_mut() {
            *offset = read_u32_be(&mut cursor)?;
        }

        // Read text section addresses (7 sections, 4 bytes each)
        let mut text_addresses: [u32; NUM_TEXT_SECTIONS] = [0u32; NUM_TEXT_SECTIONS];
        for addr in text_addresses.iter_mut() {
            *addr = read_u32_be(&mut cursor)?;
        }

        // Read data section addresses (11 sections, 4 bytes each)
        let mut data_addresses: [u32; NUM_DATA_SECTIONS] = [0u32; NUM_DATA_SECTIONS];
        for addr in data_addresses.iter_mut() {
            *addr = read_u32_be(&mut cursor)?;
        }

        // Read text section sizes (7 sections, 4 bytes each)
        let mut text_sizes: [u32; NUM_TEXT_SECTIONS] = [0u32; NUM_TEXT_SECTIONS];
        for size in text_sizes.iter_mut() {
            *size = read_u32_be(&mut cursor)?;
        }

        // Read data section sizes (11 sections, 4 bytes each)
        let mut data_sizes: [u32; NUM_DATA_SECTIONS] = [0u32; NUM_DATA_SECTIONS];
        for size in data_sizes.iter_mut() {
            *size = read_u32_be(&mut cursor)?;
        }

        // Read BSS address and size (at offset 0xD8)
        const BSS_OFFSET: u64 = 0xD8u64;
        cursor.set_position(BSS_OFFSET);
        let bss_address: u32 = read_u32_be(&mut cursor)?;
        let bss_size: u32 = read_u32_be(&mut cursor)?;

        // Read entry point (at offset 0xE0)
        const ENTRY_POINT_OFFSET: u64 = 0xE0u64;
        cursor.set_position(ENTRY_POINT_OFFSET);
        let entry_point: u32 = read_u32_be(&mut cursor)?;

        // Parse text sections
        let mut text_sections: Vec<Section> = Vec::with_capacity(NUM_TEXT_SECTIONS);
        for i in 0usize..NUM_TEXT_SECTIONS {
            if text_offsets[i] != 0u32 && text_sizes[i] != 0u32 {
                let offset: usize = text_offsets[i] as usize;
                let size: usize = text_sizes[i] as usize;

                if offset.wrapping_add(size) > data.len() {
                    anyhow::bail!(
                        "Text section {} extends beyond file: offset {}, size {}",
                        i,
                        offset,
                        size
                    );
                }

                let section_data: Vec<u8> = data[offset..offset.wrapping_add(size)].to_vec();
                text_sections.push(Section {
                    offset: text_offsets[i],
                    address: text_addresses[i],
                    size: text_sizes[i],
                    data: section_data,
                    executable: true,
                });
            }
        }

        // Parse data sections
        let mut data_sections: Vec<Section> = Vec::with_capacity(NUM_DATA_SECTIONS);
        for i in 0usize..NUM_DATA_SECTIONS {
            if data_offsets[i] != 0u32 && data_sizes[i] != 0u32 {
                let offset: usize = data_offsets[i] as usize;
                let size: usize = data_sizes[i] as usize;

                if offset.wrapping_add(size) > data.len() {
                    anyhow::bail!(
                        "Data section {} extends beyond file: offset {}, size {}",
                        i,
                        offset,
                        size
                    );
                }

                let section_data: Vec<u8> = data[offset..offset.wrapping_add(size)].to_vec();
                data_sections.push(Section {
                    offset: data_offsets[i],
                    address: data_addresses[i],
                    size: data_sizes[i],
                    data: section_data,
                    executable: false,
                });
            }
        }

        Ok(Self {
            text_sections,
            data_sections,
            bss_address,
            bss_size,
            entry_point,
            path: path.to_string(),
        })
    }

    /// Get all sections (text and data combined).
    ///
    /// # Returns
    /// `Vec<Section>` - All sections from the DOL file
    ///
    /// # Examples
    /// ```rust
    /// let all_sections = dol_file.get_all_sections();
    /// ```
    #[inline] // Simple function - may be inlined
    pub fn get_all_sections(&self) -> Vec<Section> {
        let mut all: Vec<Section> =
            Vec::with_capacity(self.text_sections.len() + self.data_sections.len());
        all.extend_from_slice(&self.text_sections);
        all.extend_from_slice(&self.data_sections);
        all
    }
}

/// Read a big-endian u32 from a cursor.
///
/// # Arguments
/// * `cursor` - Cursor to read from
///
/// # Returns
/// `Result<u32>` - Read u32 value, or error if read fails
#[inline] // Hot path - may be inlined
fn read_u32_be(cursor: &mut Cursor<&[u8]>) -> Result<u32> {
    const U32_SIZE: usize = 4usize;
    let mut buf: [u8; U32_SIZE] = [0u8; U32_SIZE];
    cursor
        .read_exact(&mut buf)
        .context("Failed to read u32 from DOL file")?;
    Ok(u32::from_be_bytes(buf))
}
