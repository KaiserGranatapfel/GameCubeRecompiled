//! Struct Detection
//!
//! This module provides struct layout inference from memory access patterns.

use std::collections::HashMap;

/// Detected struct field.
#[derive(Debug, Clone)]
pub struct StructField {
    /// Field offset in bytes
    pub offset: usize,
    /// Field size in bytes
    pub size: usize,
    /// Field type (if inferred)
    pub field_type: Option<String>,
}

/// Detected struct layout.
#[derive(Debug, Clone)]
pub struct StructLayout {
    /// Struct name
    pub name: String,
    /// Base address (if known)
    pub base_address: Option<u32>,
    /// Fields in the struct
    pub fields: Vec<StructField>,
    /// Total struct size
    pub size: usize,
}

/// Struct detector that infers struct layouts from memory access patterns.
pub struct StructDetector {
    /// Detected structs
    structs: HashMap<String, StructLayout>,
}

impl StructDetector {
    /// Create a new struct detector.
    pub fn new() -> Self {
        Self {
            structs: HashMap::new(),
        }
    }

    /// Detect structs from memory access patterns.
    ///
    /// # Arguments
    /// * `accesses` - List of memory accesses (address, size, type)
    ///
    /// # Returns
    /// Detected struct layouts
    pub fn detect_structs(&mut self, accesses: &[(u32, usize, &str)]) -> Vec<StructLayout> {
        // Group accesses by base address (struct instance)
        let mut struct_instances: HashMap<u32, Vec<(u32, usize, &str)>> = HashMap::new();

        for (address, size, access_type) in accesses {
            // Find base address (round down to likely struct boundary)
            let base = (*address / 16) * 16; // Assume 16-byte alignment
            struct_instances.entry(base).or_insert_with(Vec::new).push((
                *address,
                *size,
                access_type,
            ));
        }

        // Infer struct layouts from access patterns
        let mut detected = Vec::new();
        for (base, accesses) in struct_instances {
            if let Some(layout) = self.infer_struct_layout(base, &accesses) {
                detected.push(layout);
            }
        }

        detected
    }

    /// Infer struct layout from access patterns.
    fn infer_struct_layout(
        &self,
        base_address: u32,
        accesses: &[(u32, usize, &str)],
    ) -> Option<StructLayout> {
        // Sort accesses by offset
        let mut fields = Vec::new();
        for (address, size, _) in accesses {
            let offset = (*address - base_address) as usize;
            fields.push(StructField {
                offset,
                size: *size,
                field_type: None,
            });
        }

        // Calculate total size
        let size = fields.iter().map(|f| f.offset + f.size).max().unwrap_or(0);

        Some(StructLayout {
            name: format!("Struct_0x{:08X}", base_address),
            base_address: Some(base_address),
            fields,
            size,
        })
    }

    /// Get detected structs.
    pub fn structs(&self) -> &HashMap<String, StructLayout> {
        &self.structs
    }
}

impl Default for StructDetector {
    fn default() -> Self {
        Self::new()
    }
}
