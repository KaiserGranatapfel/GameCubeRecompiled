use std::net::SocketAddr;

/// Bind to localhost only for security.
pub fn bind_address() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], 8080))
}

/// Maximum upload size: 5 GB.
/// Used by the Rust dispatcher for the first-line-of-defense manual check.
pub const MAX_UPLOAD_SIZE: usize = 5 * 1024 * 1024 * 1024;
