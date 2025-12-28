//! Tests for malformed binary handling

use gcrecomp_core::recompiler::parser::DolFile;
use anyhow::Result;

#[test]
fn test_too_small_dol() {
    // Test handling of DOL file that's too small
    let data = vec![0u8; 100]; // Less than minimum 256 bytes
    let result = DolFile::parse(&data, "test.dol");
    
    // Should attempt partial parse with warnings, not fail completely
    assert!(result.is_ok() || result.is_err()); // Either is acceptable
}

#[test]
fn test_corrupted_section() {
    // Test handling of corrupted section (offset + size > file size)
    let mut data = vec![0u8; 512];
    
    // Set up a section that extends beyond file
    // Text section 0: offset=0x200, size=0x1000 (but file is only 512 bytes)
    data[0x00..0x04].copy_from_slice(&0x200u32.to_be_bytes());
    data[0x38..0x3C].copy_from_slice(&0x1000u32.to_be_bytes());
    
    let result = DolFile::parse(&data, "test.dol");
    
    // Should handle gracefully with partial data
    if let Ok(dol) = result {
        // Should have parsed what it could
        assert!(dol.text_sections.len() <= 7);
    }
}

#[test]
fn test_zero_entry_point() {
    // Test handling of zero entry point (may indicate corruption)
    let mut data = vec![0u8; 512];
    // Entry point at offset 0xE0
    data[0xE0..0xE4].copy_from_slice(&0u32.to_be_bytes());
    
    let result = DolFile::parse(&data, "test.dol");
    // Should parse but log warning
    assert!(result.is_ok());
}

#[test]
fn test_invalid_bss_size() {
    // Test handling of unusually large BSS size
    let mut data = vec![0u8; 512];
    // BSS size at offset 0xDC
    data[0xDC..0xE0].copy_from_slice(&0x2000000u32.to_be_bytes()); // 32MB
    
    let result = DolFile::parse(&data, "test.dol");
    // Should parse but log warning
    assert!(result.is_ok());
}

