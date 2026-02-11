use mlua::{Lua, Table};
use std::io::Read;
use std::path::Path;
use std::process::Command;

use crate::error::IntoAnyhow;

/// Maximum upload size: 5 GB.
const MAX_UPLOAD_SIZE: usize = 5 * 1024 * 1024 * 1024;

/// Valid compilation targets.
const VALID_TARGETS: &[(&str, &str)] = &[
    ("x86_64-linux", "x86_64 Linux"),
    ("x86_64-windows", "x86_64 Windows"),
    ("aarch64-linux", "AArch64 Linux"),
    ("aarch64-macos", "AArch64 macOS"),
];

pub fn register(lua: &Lua, gcrecomp: &Table) -> anyhow::Result<()> {
    let web_table = lua.create_table().into_anyhow()?;

    // gcrecomp.web.validate_dol(bytes) -> bool
    let validate_dol_fn = lua
        .create_function(|_, data: mlua::String| {
            let bytes = data.as_bytes();
            Ok(validate_dol_magic(&bytes))
        })
        .into_anyhow()?;

    // gcrecomp.web.extract_dol_from_zip(bytes) -> bytes|nil, error|nil
    let extract_zip_fn = lua
        .create_function(|lua, data: mlua::String| {
            let bytes = data.as_bytes();
            match extract_dol_from_zip(&bytes) {
                Ok(dol_bytes) => {
                    let s = lua.create_string(&dol_bytes)?;
                    Ok((Some(s), mlua::Value::Nil))
                }
                Err(msg) => {
                    let err = lua.create_string(&msg)?;
                    Ok((None, mlua::Value::String(err)))
                }
            }
        })
        .into_anyhow()?;

    // gcrecomp.web.extract_dol_from_disc(bytes) -> bytes|nil, error|nil
    // Parses a GameCube ISO/GCM disc image and extracts the main DOL.
    let extract_disc_fn = lua
        .create_function(|lua, data: mlua::String| {
            let bytes = data.as_bytes();
            match extract_dol_from_gcm_iso(&bytes) {
                Ok(dol_bytes) => {
                    let s = lua.create_string(&dol_bytes)?;
                    Ok((Some(s), mlua::Value::Nil))
                }
                Err(msg) => {
                    let err = lua.create_string(&msg)?;
                    Ok((None, mlua::Value::String(err)))
                }
            }
        })
        .into_anyhow()?;

    // gcrecomp.web.extract_dol_from_rvz(bytes) -> bytes|nil, error|nil
    // Converts RVZ to ISO via dolphin-tool, then extracts the DOL.
    // Auto-downloads dolphin-tool into tools/ if not found.
    let extract_rvz_fn = lua
        .create_function(|lua, data: mlua::String| {
            let bytes = data.as_bytes();
            match extract_dol_from_rvz(&bytes) {
                Ok(dol_bytes) => {
                    let s = lua.create_string(&dol_bytes)?;
                    Ok((Some(s), mlua::Value::Nil))
                }
                Err(msg) => {
                    let err = lua.create_string(&msg)?;
                    Ok((None, mlua::Value::String(err)))
                }
            }
        })
        .into_anyhow()?;

    // gcrecomp.web.extract_files_from_disc(bytes) -> file_count|nil, error|nil
    // Extracts all files from a GameCube disc image FST and writes a GCFS archive.
    let extract_files_fn = lua
        .create_function(|lua, data: mlua::String| {
            let bytes = data.as_bytes();
            match extract_and_archive_disc_files(&bytes) {
                Ok(count) => Ok((Some(count), mlua::Value::Nil)),
                Err(msg) => {
                    let err = lua.create_string(&msg)?;
                    Ok((None, mlua::Value::String(err)))
                }
            }
        })
        .into_anyhow()?;

    // gcrecomp.web.extract_dol_and_files_from_rvz(bytes) -> dol_bytes|nil, error|nil
    // Converts RVZ to ISO once, then extracts both DOL and filesystem files.
    let extract_rvz_combined_fn = lua
        .create_function(|lua, data: mlua::String| {
            let bytes = data.as_bytes();
            match extract_dol_and_files_from_rvz(&bytes) {
                Ok(dol_bytes) => {
                    let s = lua.create_string(&dol_bytes)?;
                    Ok((Some(s), mlua::Value::Nil))
                }
                Err(msg) => {
                    let err = lua.create_string(&msg)?;
                    Ok((None, mlua::Value::String(err)))
                }
            }
        })
        .into_anyhow()?;

    // gcrecomp.web.compile_game(title, target) -> filename|nil, error|nil
    let compile_game_fn = lua
        .create_function(|lua, (title, target): (String, String)| {
            match compile_game(&title, &target) {
                Ok(filename) => {
                    let s = lua.create_string(&filename)?;
                    Ok((Some(s), mlua::Value::Nil))
                }
                Err(e) => {
                    let err = lua.create_string(e.to_string())?;
                    Ok((None, mlua::Value::String(err)))
                }
            }
        })
        .into_anyhow()?;

    // gcrecomp.web.save_dol(bytes, path) -> true
    let save_dol_fn = lua
        .create_function(|_, (data, path): (mlua::String, String)| {
            let bytes = data.as_bytes();
            let p = Path::new(&path);
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent).map_err(mlua::Error::external)?;
            }
            std::fs::write(p, &*bytes).map_err(mlua::Error::external)?;
            Ok(true)
        })
        .into_anyhow()?;

    // gcrecomp.web.update_status(stage, msg) -> nil
    // No-op placeholder; overridden per-request in the pipeline task.
    let update_status_fn = lua
        .create_function(|_, (_stage, _msg): (String, String)| Ok(()))
        .into_anyhow()?;

    // gcrecomp.web.valid_targets() -> table of {id, name}
    let valid_targets_fn = lua
        .create_function(|lua, ()| {
            let tbl = lua.create_table()?;
            for (i, &(id, name)) in VALID_TARGETS.iter().enumerate() {
                let entry = lua.create_table()?;
                entry.set("id", id)?;
                entry.set("name", name)?;
                tbl.set(i + 1, entry)?;
            }
            Ok(tbl)
        })
        .into_anyhow()?;

    // gcrecomp.web.max_upload_size() -> integer
    let max_upload_size_fn = lua
        .create_function(|_, ()| Ok(MAX_UPLOAD_SIZE))
        .into_anyhow()?;

    web_table
        .set("validate_dol", validate_dol_fn)
        .into_anyhow()?;
    web_table
        .set("extract_dol_from_zip", extract_zip_fn)
        .into_anyhow()?;
    web_table
        .set("extract_dol_from_disc", extract_disc_fn)
        .into_anyhow()?;
    web_table
        .set("extract_dol_from_rvz", extract_rvz_fn)
        .into_anyhow()?;
    web_table
        .set("extract_files_from_disc", extract_files_fn)
        .into_anyhow()?;
    web_table
        .set("extract_dol_and_files_from_rvz", extract_rvz_combined_fn)
        .into_anyhow()?;
    web_table
        .set("compile_game", compile_game_fn)
        .into_anyhow()?;
    web_table.set("save_dol", save_dol_fn).into_anyhow()?;
    web_table
        .set("update_status", update_status_fn)
        .into_anyhow()?;
    web_table
        .set("valid_targets", valid_targets_fn)
        .into_anyhow()?;
    web_table
        .set("max_upload_size", max_upload_size_fn)
        .into_anyhow()?;

    gcrecomp.set("web", web_table).into_anyhow()?;
    Ok(())
}

// ===========================================================================
// DOL validation
// ===========================================================================

/// Validate that uploaded data looks like a DOL file.
fn validate_dol_magic(data: &[u8]) -> bool {
    if data.len() < 0x100 {
        return false;
    }
    let first_offset = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
    first_offset >= 0x100 && first_offset % 4 == 0
}

// ===========================================================================
// GameCube disc image (ISO / GCM) → DOL extraction
// ===========================================================================

/// GameCube disc magic at offset 0x1C.
const GC_DISC_MAGIC: u32 = 0xC2339F3D;

/// Extract main.dol from a raw GameCube ISO/GCM disc image.
fn extract_dol_from_gcm_iso(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < 0x440 {
        return Err("File too small to be a GameCube disc image.".to_string());
    }

    let magic = read_u32_be(data, 0x1C);
    if magic != GC_DISC_MAGIC {
        return Err(format!(
            "Not a valid GameCube disc image (magic 0x{:08X}, expected 0x{:08X}).",
            magic, GC_DISC_MAGIC
        ));
    }

    // DOL offset is stored at 0x420 in the boot block
    let dol_offset = read_u32_be(data, 0x420) as usize;
    if dol_offset == 0 {
        return Err("Disc image has no DOL (offset is 0).".to_string());
    }
    if dol_offset + 0x100 > data.len() {
        return Err(format!(
            "DOL offset 0x{:X} is past end of image ({} bytes).",
            dol_offset,
            data.len()
        ));
    }

    let dol_size = compute_dol_size(&data[dol_offset..])?;
    if dol_offset + dol_size > data.len() {
        return Err(format!(
            "DOL (0x{:X} + {} bytes) extends past end of image.",
            dol_offset, dol_size
        ));
    }

    log::info!(
        "Extracted DOL: offset=0x{:X}, size={} bytes",
        dol_offset,
        dol_size
    );
    Ok(data[dol_offset..dol_offset + dol_size].to_vec())
}

/// Compute the total file size of a DOL from its header.
/// DOL header: 7 text sections + 11 data sections, each with (file_offset, size).
fn compute_dol_size(dol: &[u8]) -> Result<usize, String> {
    if dol.len() < 0x100 {
        return Err("DOL header is too small.".to_string());
    }

    let mut max_end: usize = 0x100; // Minimum: the header itself

    // Text sections: offsets at 0x00 (7 × u32), sizes at 0x90 (7 × u32)
    for i in 0..7 {
        let off = read_u32_be(dol, i * 4) as usize;
        let size = read_u32_be(dol, 0x90 + i * 4) as usize;
        if off > 0 && size > 0 {
            max_end = max_end.max(off + size);
        }
    }

    // Data sections: offsets at 0x1C (11 × u32), sizes at 0xAC (11 × u32)
    for i in 0..11 {
        let off = read_u32_be(dol, 0x1C + i * 4) as usize;
        let size = read_u32_be(dol, 0xAC + i * 4) as usize;
        if off > 0 && size > 0 {
            max_end = max_end.max(off + size);
        }
    }

    Ok(max_end)
}

fn read_u32_be(data: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

// ===========================================================================
// RVZ → DOL extraction (via dolphin-tool)
// ===========================================================================

/// Convert RVZ to ISO via dolphin-tool, returning the raw ISO data.
fn convert_rvz_to_iso(rvz_data: &[u8]) -> Result<Vec<u8>, String> {
    let tool_path = find_or_download_dolphin_tool()?;

    let tmp_dir = std::env::temp_dir();
    let tmp_rvz = tmp_dir.join("gcrecomp_input.rvz");
    let tmp_iso = tmp_dir.join("gcrecomp_output.iso");

    std::fs::write(&tmp_rvz, rvz_data)
        .map_err(|e| format!("Failed to write temp RVZ: {}", e))?;

    log::info!("Converting RVZ to ISO using {}...", tool_path);
    let output = Command::new(&tool_path)
        .args([
            "convert",
            "-i",
            tmp_rvz.to_str().unwrap_or(""),
            "-o",
            tmp_iso.to_str().unwrap_or(""),
            "-f",
            "iso",
        ])
        .output()
        .map_err(|e| {
            let _ = std::fs::remove_file(&tmp_rvz);
            format!("Failed to run dolphin-tool: {}", e)
        })?;

    let _ = std::fs::remove_file(&tmp_rvz);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let _ = std::fs::remove_file(&tmp_iso);
        return Err(format!("dolphin-tool convert failed: {}", stderr));
    }

    let iso_data = std::fs::read(&tmp_iso).map_err(|e| {
        let _ = std::fs::remove_file(&tmp_iso);
        format!("Failed to read converted ISO: {}", e)
    })?;
    let _ = std::fs::remove_file(&tmp_iso);

    Ok(iso_data)
}

/// Extract main.dol from an RVZ file by converting to ISO with dolphin-tool.
fn extract_dol_from_rvz(rvz_data: &[u8]) -> Result<Vec<u8>, String> {
    let iso_data = convert_rvz_to_iso(rvz_data)?;
    extract_dol_from_gcm_iso(&iso_data)
}

/// Convert RVZ to ISO once, then extract both DOL and filesystem files.
/// This avoids converting twice when the caller needs both.
fn extract_dol_and_files_from_rvz(rvz_data: &[u8]) -> Result<Vec<u8>, String> {
    let iso_data = convert_rvz_to_iso(rvz_data)?;

    // Extract filesystem files (non-fatal)
    if let Err(e) = extract_and_archive_disc_files(&iso_data) {
        log::warn!("FST extraction from RVZ failed (continuing): {}", e);
    }

    extract_dol_from_gcm_iso(&iso_data)
}

// ===========================================================================
// Disc filesystem extraction → GCFS archive
// ===========================================================================

/// Extract all files from a disc image's FST and write a GCFS archive to game/assets.bin.
/// Returns the number of files extracted.
fn extract_and_archive_disc_files(disc_data: &[u8]) -> Result<usize, String> {
    let files = super::disc_fs::extract_all_files(disc_data)?;
    if files.is_empty() {
        log::info!("No files found in disc filesystem.");
        return Ok(0);
    }

    let count = files.len();
    let archive = super::disc_fs::build_archive(&files)?;

    std::fs::create_dir_all("game").map_err(|e| format!("Failed to create game/: {}", e))?;
    std::fs::write("game/assets.bin", &archive)
        .map_err(|e| format!("Failed to write game/assets.bin: {}", e))?;

    log::info!(
        "Wrote GCFS archive: {} files, {} bytes to game/assets.bin",
        count,
        archive.len()
    );
    Ok(count)
}

/// Find dolphin-tool in PATH or local tools/ directory.
/// If not found anywhere, download the Arch package and extract it locally.
fn find_or_download_dolphin_tool() -> Result<String, String> {
    // 1. Check PATH
    if let Ok(output) = Command::new("which").arg("dolphin-tool").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                log::info!("Found dolphin-tool in PATH: {}", path);
                return Ok(path);
            }
        }
    }

    // 2. Check local tools/ directory
    let local_path = Path::new("tools/dolphin-tool");
    if local_path.exists() {
        log::info!("Found local dolphin-tool: {}", local_path.display());
        return Ok(local_path.to_string_lossy().to_string());
    }

    // 3. Not found — download it
    log::info!("dolphin-tool not found, downloading...");
    download_dolphin_tool()?;

    if local_path.exists() {
        return Ok(local_path.to_string_lossy().to_string());
    }

    Err(
        "Failed to obtain dolphin-tool. Install manually: sudo pacman -S dolphin-emu-tool"
            .to_string(),
    )
}

/// Download the dolphin-emu-tool Arch package and extract just the binary
/// into the project's tools/ directory. No root access required.
fn download_dolphin_tool() -> Result<(), String> {
    std::fs::create_dir_all("tools").map_err(|e| format!("Failed to create tools/: {}", e))?;

    let tmp_pkg = std::env::temp_dir().join("dolphin-emu-tool.pkg.tar.zst");
    let tmp_pkg_str = tmp_pkg.to_string_lossy().to_string();

    // Download from Arch repos
    log::info!("Downloading dolphin-emu-tool package...");
    let status = Command::new("curl")
        .args([
            "-L",
            "--fail",
            "-o",
            &tmp_pkg_str,
            "https://archlinux.org/packages/extra/x86_64/dolphin-emu-tool/download/",
        ])
        .status()
        .map_err(|e| format!("Failed to run curl: {}", e))?;

    if !status.success() {
        let _ = std::fs::remove_file(&tmp_pkg);
        return Err("Failed to download dolphin-emu-tool package from Arch repos.".to_string());
    }

    // Extract just the dolphin-tool binary
    log::info!("Extracting dolphin-tool binary...");
    let status = Command::new("tar")
        .args([
            "-xf",
            &tmp_pkg_str,
            "--strip-components=2",
            "-C",
            "tools/",
            "usr/bin/dolphin-tool",
        ])
        .status()
        .map_err(|e| {
            let _ = std::fs::remove_file(&tmp_pkg);
            format!("Failed to run tar: {}", e)
        })?;

    let _ = std::fs::remove_file(&tmp_pkg);

    if !status.success() {
        return Err("Failed to extract dolphin-tool from package.".to_string());
    }

    // Ensure executable
    let _ = Command::new("chmod")
        .args(["+x", "tools/dolphin-tool"])
        .status();

    log::info!("dolphin-tool installed to tools/dolphin-tool");
    Ok(())
}

// ===========================================================================
// ZIP extraction (supports .dol, .iso, .gcm, .rvz inside zips)
// ===========================================================================

/// Extract a DOL from a zip archive.
///
/// Strategy:
/// 1. Look for .dol files by extension.
/// 2. Look for disc images (.iso, .gcm) by extension → extract DOL from disc.
/// 3. Look for .rvz files by extension → convert via dolphin-tool → extract DOL.
/// 4. Check every file for DOL magic bytes.
/// 5. If nothing found, list entries in the error.
fn extract_dol_from_zip(zip_data: &[u8]) -> Result<Vec<u8>, String> {
    let cursor = std::io::Cursor::new(zip_data);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|e| format!("Invalid zip file: {}", e))?;

    let entry_count = archive.len();
    let mut entry_names: Vec<String> = Vec::with_capacity(entry_count);

    // Collect entry metadata
    let mut entries: Vec<(usize, String, bool)> = Vec::with_capacity(entry_count);
    for i in 0..entry_count {
        let file = archive
            .by_index(i)
            .map_err(|e| format!("Zip read error at index {}: {}", i, e))?;
        let name = file.name().to_string();
        let is_dir = file.is_dir();
        if is_dir {
            entry_names.push(format!("{}  (dir)", name));
        } else {
            entry_names.push(name.clone());
        }
        entries.push((i, name, is_dir));
    }

    // Pass 1: .dol by extension
    for &(i, ref name, is_dir) in &entries {
        if is_dir {
            continue;
        }
        let basename = name.rsplit('/').next().unwrap_or(name);
        if basename.to_lowercase().ends_with(".dol") {
            let mut file = archive
                .by_index(i)
                .map_err(|e| format!("Zip read error: {}", e))?;
            return read_zip_entry(&mut file);
        }
    }

    // Pass 2: .iso / .gcm disc images → parse for DOL
    for &(i, ref name, is_dir) in &entries {
        if is_dir {
            continue;
        }
        let lower = name.to_lowercase();
        let basename = lower.rsplit('/').next().unwrap_or(&lower);
        if basename.ends_with(".iso") || basename.ends_with(".gcm") {
            let mut file = archive
                .by_index(i)
                .map_err(|e| format!("Zip read error: {}", e))?;
            let disc_data = read_zip_entry(&mut file)?;
            return extract_dol_from_gcm_iso(&disc_data);
        }
    }

    // Pass 3: .rvz → convert via dolphin-tool
    for &(i, ref name, is_dir) in &entries {
        if is_dir {
            continue;
        }
        let lower = name.to_lowercase();
        let basename = lower.rsplit('/').next().unwrap_or(&lower);
        if basename.ends_with(".rvz") {
            let mut file = archive
                .by_index(i)
                .map_err(|e| format!("Zip read error: {}", e))?;
            let rvz_data = read_zip_entry(&mut file)?;
            return extract_dol_from_rvz(&rvz_data);
        }
    }

    // Pass 4: DOL magic check on every non-directory file
    for &(i, _, is_dir) in &entries {
        if is_dir {
            continue;
        }
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Zip read error: {}", e))?;
        if file.size() < 0x100 {
            continue;
        }
        let buf = read_zip_entry_bytes(&mut file)?;
        if validate_dol_magic(&buf) {
            return Ok(buf);
        }
    }

    // Nothing found
    let listing = if entry_names.is_empty() {
        "  (archive is empty)".to_string()
    } else {
        entry_names
            .iter()
            .map(|n| format!("  - {}", n))
            .collect::<Vec<_>>()
            .join("\n")
    };
    Err(format!(
        "No DOL or disc image found inside the zip archive.\nEntries found:\n{}",
        listing
    ))
}

/// Read a zip entry into a Vec<u8>, enforcing the size limit.
fn read_zip_entry(file: &mut zip::read::ZipFile) -> Result<Vec<u8>, String> {
    if file.size() > MAX_UPLOAD_SIZE as u64 {
        return Err(format!(
            "File '{}' inside zip is too large ({} bytes).",
            file.name(),
            file.size()
        ));
    }
    read_zip_entry_bytes(file)
}

fn read_zip_entry_bytes(file: &mut zip::read::ZipFile) -> Result<Vec<u8>, String> {
    if file.size() > MAX_UPLOAD_SIZE as u64 {
        return Err(format!(
            "File '{}' inside zip is too large ({} bytes).",
            file.name(),
            file.size()
        ));
    }
    let mut buf = Vec::with_capacity(file.size() as usize);
    file.read_to_end(&mut buf)
        .map_err(|e| format!("Failed to read '{}' from zip: {}", file.name(), e))?;
    Ok(buf)
}

// ===========================================================================
// Compile game
// ===========================================================================

/// Compile the game crate and produce a named executable.
fn compile_game(game_title: &str, target: &str) -> anyhow::Result<String> {
    std::fs::create_dir_all("output")?;
    if Path::new("output/recompiled.rs").exists() {
        std::fs::copy("output/recompiled.rs", "game/src/recompiled.rs")?;
    }

    let target_triple = match target {
        "x86_64-linux" => "x86_64-unknown-linux-gnu",
        "x86_64-windows" => "x86_64-pc-windows-gnu",
        "aarch64-linux" => "aarch64-unknown-linux-gnu",
        "aarch64-macos" => "aarch64-apple-darwin",
        _ => "x86_64-unknown-linux-gnu",
    };

    let output = Command::new("cargo")
        .args([
            "build",
            "--release",
            "-p",
            "game",
            "--target",
            target_triple,
        ])
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to invoke cargo: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let short_err = stderr
            .lines()
            .filter(|l| l.contains("error") || l.contains("Error"))
            .take(10)
            .collect::<Vec<_>>()
            .join("\n");
        let msg = if short_err.is_empty() {
            stderr.to_string()
        } else {
            short_err
        };
        return Err(anyhow::anyhow!("Compilation failed:\n{}", msg));
    }

    let ext = if target.contains("windows") {
        ".exe"
    } else {
        ""
    };
    let src_binary = format!("target/{}/release/game{}", target_triple, ext);

    let safe_title: String = game_title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let dst_name = format!("{}{}", safe_title, ext);
    let dst_path = format!("output/{}", dst_name);

    std::fs::copy(&src_binary, &dst_path).map_err(|e| {
        anyhow::anyhow!(
            "Failed to copy binary from {} to {}: {}",
            src_binary,
            dst_path,
            e
        )
    })?;

    // Ensure the binary is executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&dst_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dst_path, perms)?;
    }

    log::info!("Game binary written to {}", dst_path);
    Ok(dst_name)
}
