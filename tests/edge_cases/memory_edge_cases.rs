//! Tests for memory access edge cases

use gcrecomp_runtime::memory::ram::Ram;
use anyhow::Result;

#[test]
fn test_out_of_bounds_read() {
    let ram = Ram::new();
    
    // Test read beyond RAM limit
    let result = ram.read_u32(0x81800000); // Beyond 24MB
    assert!(result.is_err());
    
    // Test read in valid range
    let result = ram.read_u32(0x80000000);
    assert!(result.is_ok());
}

#[test]
fn test_unaligned_access() {
    let mut ram = Ram::new();
    
    // GameCube allows unaligned accesses
    // Test unaligned read
    let result = ram.read_u32(0x80000001); // Not 4-byte aligned
    assert!(result.is_ok()); // Should handle gracefully
    
    // Test unaligned write
    let result = ram.write_u32(0x80000001, 0x12345678);
    assert!(result.is_ok());
}

#[test]
fn test_memory_mapped_io() {
    let ram = Ram::new();
    
    // Test read from memory-mapped I/O region
    let result = ram.read_u8(0xCC000000); // VRAM region
    // Should handle gracefully (may return 0 or error)
    let _ = result;
}

#[test]
fn test_boundary_access() {
    let mut ram = Ram::new();
    
    // Test access at exact boundary
    let boundary = 0x817FFFFC; // Last valid 32-bit address
    let result = ram.read_u32(boundary);
    assert!(result.is_ok());
    
    // Test access just beyond boundary
    let result = ram.read_u32(0x81800000);
    assert!(result.is_err());
}

