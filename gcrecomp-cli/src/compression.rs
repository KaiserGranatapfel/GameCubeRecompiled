//! Compression Support
//!
//! This module provides compression utilities for output files.

use anyhow::Result;
use std::path::Path;

/// Compression algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionAlgorithm {
    /// Gzip compression
    Gzip,
    /// Zstd compression
    Zstd,
    /// LZ4 compression
    Lz4,
    /// No compression
    None,
}

/// Compression level (0-9 for gzip, 0-22 for zstd).
#[derive(Debug, Clone, Copy)]
pub struct CompressionLevel(u8);

impl CompressionLevel {
    pub fn new(level: u8, algorithm: CompressionAlgorithm) -> Self {
        let max_level = match algorithm {
            CompressionAlgorithm::Gzip => 9,
            CompressionAlgorithm::Zstd => 22,
            CompressionAlgorithm::Lz4 => 16,
            CompressionAlgorithm::None => 0,
        };
        Self(level.min(max_level))
    }
}

/// Compress a file.
pub fn compress_file(
    input_path: &Path,
    output_path: &Path,
    algorithm: CompressionAlgorithm,
    level: CompressionLevel,
) -> Result<()> {
    match algorithm {
        CompressionAlgorithm::Gzip => {
            use flate2::write::GzEncoder;
            use flate2::Compression;
            use std::fs::File;
            use std::io::Write;

            let input = std::fs::read(input_path)?;
            let output = File::create(output_path)?;
            let mut encoder = GzEncoder::new(output, Compression::new(level.0 as u32));
            encoder.write_all(&input)?;
            encoder.finish()?;
        }
        CompressionAlgorithm::Zstd => {
            let input = std::fs::read(input_path)?;
            let compressed = zstd::encode_all(&input[..], level.0 as i32)?;
            std::fs::write(output_path, compressed)?;
        }
        CompressionAlgorithm::Lz4 => {
            // LZ4 compression would be implemented here
            return Err(anyhow::anyhow!("LZ4 compression not yet implemented"));
        }
        CompressionAlgorithm::None => {
            std::fs::copy(input_path, output_path)?;
        }
    }
    Ok(())
}

/// Decompress a file.
pub fn decompress_file(input_path: &Path, output_path: &Path) -> Result<()> {
    // Auto-detect compression format and decompress
    let input = std::fs::read(input_path)?;

    // Try gzip first
    if let Ok(decompressed) = decompress_gzip(&input) {
        std::fs::write(output_path, decompressed)?;
        return Ok(());
    }

    // Try zstd
    if let Ok(decompressed) = zstd::decode_all(&input[..]) {
        std::fs::write(output_path, decompressed)?;
        return Ok(());
    }

    // No compression detected, copy as-is
    std::fs::copy(input_path, output_path)?;
    Ok(())
}

fn decompress_gzip(data: &[u8]) -> Result<Vec<u8>> {
    use flate2::read::GzDecoder;
    use std::io::Read;
    let mut decoder = GzDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}
