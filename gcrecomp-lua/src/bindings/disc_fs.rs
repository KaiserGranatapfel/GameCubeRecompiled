//! GameCube disc filesystem (FST) parser and GCFS archive builder.
//!
//! Extracts all files from a GameCube disc image's File System Table,
//! then packs them into a compressed GCFS archive for embedding.

use std::io::Write;

/// A file extracted from the GameCube disc filesystem.
pub struct DiscFile {
    pub path: String,
    pub data: Vec<u8>,
}

/// Read a big-endian u32 from a byte slice at the given offset.
fn read_u32_be(data: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

/// Extract all files from a GameCube disc image's FST.
///
/// The FST location is stored at disc offset 0x424 (FST offset) and
/// 0x428 (FST size). Each FST entry is 12 bytes:
///   - Byte 0: flags (0 = file, 1 = directory)
///   - Bytes 1-3: offset into string table (24-bit)
///   - Bytes 4-7: for files: data offset; for dirs: parent index
///   - Bytes 8-11: for files: data size; for dirs: next entry index (exclusive)
pub fn extract_all_files(disc_data: &[u8]) -> Result<Vec<DiscFile>, String> {
    if disc_data.len() < 0x440 {
        return Err("Disc image too small to contain FST pointers.".to_string());
    }

    let fst_offset = read_u32_be(disc_data, 0x424) as usize;
    if fst_offset == 0 || fst_offset >= disc_data.len() {
        return Err(format!(
            "Invalid FST offset: 0x{:X} (disc size: {} bytes).",
            fst_offset,
            disc_data.len()
        ));
    }

    // Root entry (index 0) — bytes 8-11 hold the total entry count
    if fst_offset + 12 > disc_data.len() {
        return Err("FST root entry extends past end of disc.".to_string());
    }
    let entry_count = read_u32_be(disc_data, fst_offset + 8) as usize;
    if entry_count == 0 {
        return Err("FST has zero entries.".to_string());
    }

    let entries_end = fst_offset + entry_count * 12;
    if entries_end > disc_data.len() {
        return Err(format!(
            "FST entries extend past end of disc (need {} bytes at 0x{:X}).",
            entry_count * 12,
            fst_offset
        ));
    }

    // String table starts immediately after all entries
    let string_table_offset = entries_end;

    // Read a null-terminated string from the string table
    let read_string = |name_offset: usize| -> String {
        let start = string_table_offset + name_offset;
        if start >= disc_data.len() {
            return String::new();
        }
        let mut end = start;
        while end < disc_data.len() && disc_data[end] != 0 {
            end += 1;
        }
        String::from_utf8_lossy(&disc_data[start..end]).into_owned()
    };

    let mut files = Vec::new();
    // Directory path stack: (dir_name, end_index)
    let mut dir_stack: Vec<(String, usize)> = Vec::new();

    // Skip root entry (index 0), walk from index 1
    let mut i = 1;
    while i < entry_count {
        let entry_off = fst_offset + i * 12;
        let flags = disc_data[entry_off];
        let name_offset = ((disc_data[entry_off + 1] as usize) << 16)
            | ((disc_data[entry_off + 2] as usize) << 8)
            | (disc_data[entry_off + 3] as usize);
        let offset_or_parent = read_u32_be(disc_data, entry_off + 4);
        let size_or_next = read_u32_be(disc_data, entry_off + 8);

        let name = read_string(name_offset);

        // Pop directories whose range we've passed
        while let Some(&(_, end)) = dir_stack.last() {
            if i >= end {
                dir_stack.pop();
            } else {
                break;
            }
        }

        if flags == 1 {
            // Directory entry
            let next_index = size_or_next as usize;
            dir_stack.push((name, next_index));
        } else {
            // File entry
            let data_offset = offset_or_parent as usize;
            let data_size = size_or_next as usize;

            // Build full path from directory stack
            let mut path = String::new();
            for (dir_name, _) in &dir_stack {
                path.push_str(dir_name);
                path.push('/');
            }
            path.push_str(&name);

            // Bounds check and extract data
            if data_offset + data_size <= disc_data.len() {
                files.push(DiscFile {
                    path,
                    data: disc_data[data_offset..data_offset + data_size].to_vec(),
                });
            } else {
                log::warn!(
                    "Skipping file '{}': data at 0x{:X}+{} extends past disc end.",
                    path,
                    data_offset,
                    data_size
                );
            }
        }

        i += 1;
    }

    log::info!("Extracted {} files from disc filesystem.", files.len());
    Ok(files)
}

/// Build a GCFS archive from extracted disc files.
///
/// Format (all integers little-endian):
/// ```text
/// Header:  magic b"GCFS" | version u32 | file_count u32 | toc_offset u64
/// Data:    [zstd-compressed file data, concatenated]
/// TOC:     For each file: path_len u16 | path bytes | data_offset u64 | compressed_size u64 | decompressed_size u64
/// ```
pub fn build_archive(files: &[DiscFile]) -> Result<Vec<u8>, String> {
    let mut archive = Vec::new();

    // Write header placeholder (will patch toc_offset later)
    archive.extend_from_slice(b"GCFS"); // magic
    archive.extend_from_slice(&1u32.to_le_bytes()); // version
    archive.extend_from_slice(&(files.len() as u32).to_le_bytes()); // file_count
    archive.extend_from_slice(&0u64.to_le_bytes()); // toc_offset placeholder
    let header_size = archive.len(); // 20 bytes

    // Compress and write file data, collecting TOC entries
    struct TocEntry {
        path: String,
        data_offset: u64,
        compressed_size: u64,
        decompressed_size: u64,
    }
    let mut toc_entries = Vec::with_capacity(files.len());

    for file in files {
        let data_offset = archive.len() as u64;
        let decompressed_size = file.data.len() as u64;

        let compressed = zstd::encode_all(file.data.as_slice(), 3)
            .map_err(|e| format!("zstd compression failed for '{}': {}", file.path, e))?;
        let compressed_size = compressed.len() as u64;

        archive
            .write_all(&compressed)
            .map_err(|e| format!("Write failed: {}", e))?;

        toc_entries.push(TocEntry {
            path: file.path.clone(),
            data_offset,
            compressed_size,
            decompressed_size,
        });
    }

    // Patch toc_offset in the header
    let toc_offset = archive.len() as u64;
    archive[header_size - 8..header_size].copy_from_slice(&toc_offset.to_le_bytes());

    // Write TOC
    for entry in &toc_entries {
        let path_bytes = entry.path.as_bytes();
        let path_len = path_bytes.len() as u16;
        archive
            .write_all(&path_len.to_le_bytes())
            .map_err(|e| format!("Write failed: {}", e))?;
        archive
            .write_all(path_bytes)
            .map_err(|e| format!("Write failed: {}", e))?;
        archive
            .write_all(&entry.data_offset.to_le_bytes())
            .map_err(|e| format!("Write failed: {}", e))?;
        archive
            .write_all(&entry.compressed_size.to_le_bytes())
            .map_err(|e| format!("Write failed: {}", e))?;
        archive
            .write_all(&entry.decompressed_size.to_le_bytes())
            .map_err(|e| format!("Write failed: {}", e))?;
    }

    log::info!(
        "Built GCFS archive: {} files, {} bytes total.",
        files.len(),
        archive.len()
    );
    Ok(archive)
}
