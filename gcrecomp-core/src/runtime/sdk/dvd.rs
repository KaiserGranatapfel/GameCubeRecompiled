//! Virtual DVD filesystem for GameCube disc asset access.
//!
//! Parses a GCFS archive (built by `disc_fs::build_archive`) and provides
//! `DVDOpen` / `DVDRead` / `DVDClose` / `DVDGetLength` emulation so
//! recompiled games can load assets at runtime.

use std::collections::HashMap;

use crate::runtime::memory::MemoryManager;

/// Table-of-contents entry parsed from the GCFS archive.
struct TocEntry {
    /// Byte offset of compressed data within the archive.
    data_offset: usize,
    /// Size of the zstd-compressed data.
    compressed_size: usize,
    /// Size after decompression.
    decompressed_size: usize,
}

/// State for a currently open file handle.
struct OpenFile {
    path: String,
    length: u32,
}

/// Virtual filesystem backed by an embedded GCFS archive.
pub struct VirtualFilesystem {
    /// Raw archive bytes (a `&'static [u8]` from `include_bytes!`).
    archive: &'static [u8],
    /// Path → TOC entry mapping.
    toc: HashMap<String, TocEntry>,
    /// Lazily decompressed file cache.
    file_cache: HashMap<String, Vec<u8>>,
    /// Open file handles: handle_id → OpenFile.
    open_files: HashMap<u32, OpenFile>,
    /// Next handle ID to assign (starts at 1; 0 means failure).
    next_handle: u32,
}

impl VirtualFilesystem {
    /// Parse a GCFS archive and build the TOC index.
    ///
    /// GCFS header layout (all little-endian):
    /// ```text
    /// [0..4]   magic b"GCFS"
    /// [4..8]   version u32
    /// [8..12]  file_count u32
    /// [12..20] toc_offset u64
    /// ```
    pub fn new(archive: &'static [u8]) -> Result<Self, String> {
        if archive.is_empty() {
            return Ok(Self {
                archive,
                toc: HashMap::new(),
                file_cache: HashMap::new(),
                open_files: HashMap::new(),
                next_handle: 1,
            });
        }

        if archive.len() < 20 {
            return Err("GCFS archive too small for header.".to_string());
        }

        if &archive[0..4] != b"GCFS" {
            return Err("Invalid GCFS magic.".to_string());
        }

        let _version = u32::from_le_bytes([archive[4], archive[5], archive[6], archive[7]]);
        let file_count =
            u32::from_le_bytes([archive[8], archive[9], archive[10], archive[11]]) as usize;
        let toc_offset = u64::from_le_bytes([
            archive[12],
            archive[13],
            archive[14],
            archive[15],
            archive[16],
            archive[17],
            archive[18],
            archive[19],
        ]) as usize;

        if toc_offset > archive.len() {
            return Err(format!(
                "GCFS TOC offset {} exceeds archive size {}.",
                toc_offset,
                archive.len()
            ));
        }

        let mut toc = HashMap::with_capacity(file_count);
        let mut pos = toc_offset;

        for _ in 0..file_count {
            if pos + 2 > archive.len() {
                return Err("GCFS TOC truncated (path_len).".to_string());
            }
            let path_len = u16::from_le_bytes([archive[pos], archive[pos + 1]]) as usize;
            pos += 2;

            if pos + path_len > archive.len() {
                return Err("GCFS TOC truncated (path).".to_string());
            }
            let path = String::from_utf8_lossy(&archive[pos..pos + path_len]).into_owned();
            pos += path_len;

            if pos + 24 > archive.len() {
                return Err("GCFS TOC truncated (offsets).".to_string());
            }
            let data_offset = read_u64_le(archive, pos) as usize;
            pos += 8;
            let compressed_size = read_u64_le(archive, pos) as usize;
            pos += 8;
            let decompressed_size = read_u64_le(archive, pos) as usize;
            pos += 8;

            toc.insert(
                path,
                TocEntry {
                    data_offset,
                    compressed_size,
                    decompressed_size,
                },
            );
        }

        log::info!(
            "DVD filesystem initialized: loaded {} files from GCFS archive.",
            toc.len()
        );

        Ok(Self {
            archive,
            toc,
            file_cache: HashMap::new(),
            open_files: HashMap::new(),
            next_handle: 1,
        })
    }

    /// Open a file by path. Returns a handle (>0) or 0 on failure.
    ///
    /// GameCube games use paths like `/banner.bnr` or `audio/stream.adp`.
    /// We normalize by stripping a leading `/` if present.
    pub fn dvd_open(&mut self, path: &str) -> u32 {
        let normalized = path.strip_prefix('/').unwrap_or(path);

        // Try exact match first, then case-insensitive
        let found = if self.toc.contains_key(normalized) {
            Some(normalized.to_string())
        } else {
            let lower = normalized.to_lowercase();
            self.toc
                .keys()
                .find(|k| k.to_lowercase() == lower)
                .cloned()
        };

        match found {
            Some(key) => {
                let entry = &self.toc[&key];
                let length = entry.decompressed_size as u32;
                let handle = self.next_handle;
                self.next_handle += 1;
                self.open_files.insert(
                    handle,
                    OpenFile {
                        path: key,
                        length,
                    },
                );
                log::debug!("DVDOpen('{}') -> handle {}", path, handle);
                handle
            }
            None => {
                log::warn!("DVDOpen('{}') -> file not found", path);
                0
            }
        }
    }

    /// Close a file handle. Returns true if the handle was valid.
    pub fn dvd_close(&mut self, handle: u32) -> bool {
        let removed = self.open_files.remove(&handle).is_some();
        if removed {
            log::debug!("DVDClose(handle={}) -> ok", handle);
        } else {
            log::warn!("DVDClose(handle={}) -> invalid handle", handle);
        }
        removed
    }

    /// Get the decompressed file length for an open handle.
    pub fn dvd_get_length(&self, handle: u32) -> u32 {
        self.open_files
            .get(&handle)
            .map(|f| f.length)
            .unwrap_or(0)
    }

    /// Read data from an open file into GameCube memory.
    ///
    /// Decompresses the file on first access and caches the result.
    /// Copies `length` bytes starting at `offset` in the file to `gc_addr` in memory.
    /// Returns the number of bytes actually read.
    pub fn dvd_read(
        &mut self,
        handle: u32,
        memory: &mut MemoryManager,
        gc_addr: u32,
        length: u32,
        offset: u32,
    ) -> Result<u32, String> {
        let file_info = self
            .open_files
            .get(&handle)
            .ok_or_else(|| format!("DVDRead: invalid handle {}", handle))?;

        let path = file_info.path.clone();

        // Decompress on first access
        if !self.file_cache.contains_key(&path) {
            let toc_entry = self
                .toc
                .get(&path)
                .ok_or_else(|| format!("DVDRead: TOC entry missing for '{}'", path))?;

            let compressed_end = toc_entry.data_offset + toc_entry.compressed_size;
            if compressed_end > self.archive.len() {
                return Err(format!(
                    "DVDRead: compressed data for '{}' out of bounds.",
                    path
                ));
            }

            let compressed =
                &self.archive[toc_entry.data_offset..compressed_end];
            let decompressed = zstd::decode_all(compressed).map_err(|e| {
                format!("DVDRead: zstd decompression failed for '{}': {}", path, e)
            })?;

            log::debug!(
                "DVDRead: decompressed '{}' ({} -> {} bytes)",
                path,
                toc_entry.compressed_size,
                decompressed.len()
            );
            self.file_cache.insert(path.clone(), decompressed);
        }

        let file_data = &self.file_cache[&path];
        let start = offset as usize;
        let end = (start + length as usize).min(file_data.len());
        if start >= file_data.len() {
            return Ok(0);
        }

        let slice = &file_data[start..end];
        memory
            .write_bytes(gc_addr, slice)
            .map_err(|e| format!("DVDRead: memory write failed at 0x{:08X}: {}", gc_addr, e))?;

        Ok(slice.len() as u32)
    }
}

fn read_u64_le(data: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
        data[offset + 4],
        data[offset + 5],
        data[offset + 6],
        data[offset + 7],
    ])
}
