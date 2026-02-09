use std::net::SocketAddr;
use std::path::Path;

/// Bind to localhost only for security.
pub fn bind_address() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], 8080))
}

/// Maximum upload size: 64 MB (largest GameCube DOL/ISO sections).
pub const MAX_UPLOAD_SIZE: usize = 64 * 1024 * 1024;

/// Validate that the uploaded file looks like a DOL.
pub fn validate_dol_magic(data: &[u8]) -> bool {
    // DOL files have specific section header layout starting at offset 0
    // Text section offsets start at 0x00, data section offsets at 0x1C
    // A valid DOL should be at least 0x100 bytes (header size)
    if data.len() < 0x100 {
        return false;
    }
    // Check that the first text section offset is reasonable (non-zero, aligned)
    let first_offset = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
    first_offset >= 0x100 && first_offset % 4 == 0
}

/// Sanitize output path to prevent directory traversal.
#[allow(dead_code)]
pub fn sanitize_path(path: &str) -> Option<&str> {
    let p = Path::new(path);
    // Reject absolute paths and path traversal
    if p.is_absolute() || path.contains("..") {
        return None;
    }
    Some(path)
}
