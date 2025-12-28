//! Unusual Code Pattern Detection
//!
//! This module detects unusual code patterns that require special handling:
//! - Self-modifying code
//! - Jump tables
//! - Indirect calls
//! - Exception handlers
//! - Interrupt handlers

use crate::recompiler::decoder::DecodedInstruction;
use anyhow::Result;
use std::collections::HashMap;

/// Detected unusual code pattern
#[derive(Debug, Clone)]
pub struct DetectedPattern {
    /// Pattern type
    pub pattern_type: PatternType,
    /// Address where pattern was detected
    pub address: u32,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Type of unusual pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternType {
    /// Self-modifying code detected
    SelfModifyingCode,
    /// Jump table detected
    JumpTable,
    /// Indirect call pattern
    IndirectCall,
    /// Exception handler
    ExceptionHandler,
    /// Interrupt handler
    InterruptHandler,
    /// Unusual memory access pattern
    UnusualMemoryAccess,
}

/// Pattern detector
pub struct PatternDetector {
    /// Detected patterns
    patterns: Vec<DetectedPattern>,
}

impl Default for PatternDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl PatternDetector {
    /// Create a new pattern detector
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
        }
    }

    /// Detect all unusual patterns in instruction sequence
    pub fn detect_patterns(&mut self, instructions: &[DecodedInstruction]) -> Result<()> {
        self.patterns.clear();
        
        // Detect self-modifying code
        self.detect_self_modifying_code(instructions)?;
        
        // Detect jump tables
        self.detect_jump_tables(instructions)?;
        
        // Detect indirect calls
        self.detect_indirect_calls(instructions)?;
        
        // Detect exception handlers
        self.detect_exception_handlers(instructions)?;
        
        // Detect interrupt handlers
        self.detect_interrupt_handlers(instructions)?;
        
        log::info!("Detected {} unusual patterns", self.patterns.len());
        
        Ok(())
    }

    /// Detect self-modifying code
    /// Self-modifying code writes to executable memory regions
    fn detect_self_modifying_code(&mut self, instructions: &[DecodedInstruction]) -> Result<()> {
        // Look for store instructions to executable memory regions (0x80000000-0x81800000)
        for inst in instructions {
            if inst.instruction.instruction_type == crate::recompiler::decoder::InstructionType::Store {
                // Check if storing to executable region
                if let Some(crate::recompiler::decoder::Operand::Address(addr)) = 
                    inst.instruction.operands.get(1) {
                    if *addr >= 0x80000000 && *addr < 0x81800000 {
                        // Potential self-modifying code
                        self.patterns.push(DetectedPattern {
                            pattern_type: PatternType::SelfModifyingCode,
                            address: inst.address,
                            confidence: 0.7,
                            metadata: {
                                let mut m = HashMap::new();
                                m.insert("target_address".to_string(), format!("0x{:08X}", addr));
                                m
                            },
                        });
                    }
                }
            }
        }
        Ok(())
    }

    /// Detect jump tables
    /// Jump tables typically use mtctr + bctrl or indirect branches
    fn detect_jump_tables(&mut self, instructions: &[DecodedInstruction]) -> Result<()> {
        for (i, inst) in instructions.iter().enumerate() {
            // Look for mtctr followed by bctrl (common jump table pattern)
            if i + 1 < instructions.len() {
                let next = &instructions[i + 1];
                
                // Check for mtctr (mtspr with CTR)
                if inst.instruction.instruction_type == crate::recompiler::decoder::InstructionType::System {
                    // Check if next instruction is bctrl
                    if next.instruction.instruction_type == crate::recompiler::decoder::InstructionType::Branch {
                        // Check if it's bctrl (opcode 19, sub-opcode 528)
                        if next.opcode == 19 {
                            // Potential jump table
                            self.patterns.push(DetectedPattern {
                                pattern_type: PatternType::JumpTable,
                                address: inst.address,
                                confidence: 0.8,
                                metadata: HashMap::new(),
                            });
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Detect indirect calls
    /// Indirect calls use register-based branch instructions
    fn detect_indirect_calls(&mut self, instructions: &[DecodedInstruction]) -> Result<()> {
        for inst in instructions {
            if inst.instruction.instruction_type == crate::recompiler::decoder::InstructionType::Branch {
                // Check for indirect branch (bctrl, bclrl)
                if inst.opcode == 19 || inst.opcode == 16 {
                    // Check if it's a call (link bit set)
                    let link = (inst.raw & 0x1) != 0;
                    if link {
                        // Indirect call detected
                        self.patterns.push(DetectedPattern {
                            pattern_type: PatternType::IndirectCall,
                            address: inst.address,
                            confidence: 0.9,
                            metadata: HashMap::new(),
                        });
                    }
                }
            }
        }
        Ok(())
    }

    /// Detect exception handlers
    /// Exception handlers typically start at specific addresses and use specific patterns
    fn detect_exception_handlers(&mut self, instructions: &[DecodedInstruction]) -> Result<()> {
        // GameCube exception vectors are at specific addresses
        const EXCEPTION_VECTORS: &[u32] = &[
            0x00000100, // System Reset
            0x00000200, // Machine Check
            0x00000300, // DSI
            0x00000400, // ISI
            0x00000500, // External Interrupt
            0x00000600, // Alignment
            0x00000700, // Program
            0x00000800, // Floating-Point Unavailable
            0x00000900, // Decrementer
            0x00000C00, // System Call
            0x00000D00, // Trace
            0x00000E00, // Performance Monitor
        ];
        
        for inst in instructions {
            if EXCEPTION_VECTORS.contains(&inst.address) {
                self.patterns.push(DetectedPattern {
                    pattern_type: PatternType::ExceptionHandler,
                    address: inst.address,
                    confidence: 1.0,
                    metadata: HashMap::new(),
                });
            }
        }
        Ok(())
    }

    /// Detect interrupt handlers
    /// Similar to exception handlers but for hardware interrupts
    fn detect_interrupt_handlers(&mut self, instructions: &[DecodedInstruction]) -> Result<()> {
        // Interrupt handlers often have specific patterns:
        // - Save context (stwu, mflr, etc.)
        // - Handle interrupt
        // - Restore context (lwz, mtlr, etc.)
        for (i, inst) in instructions.iter().enumerate() {
            if i + 5 < instructions.len() {
                // Look for interrupt handler prologue pattern
                let prologue = &instructions[i..i+5];
                let has_save = prologue.iter().any(|i| {
                    i.instruction.instruction_type == crate::recompiler::decoder::InstructionType::Store
                });
                let has_mflr = prologue.iter().any(|i| {
                    i.instruction.instruction_type == crate::recompiler::decoder::InstructionType::Move &&
                    i.opcode == 31 // mflr
                });
                
                if has_save && has_mflr {
                    // Potential interrupt handler
                    self.patterns.push(DetectedPattern {
                        pattern_type: PatternType::InterruptHandler,
                        address: inst.address,
                        confidence: 0.6,
                        metadata: HashMap::new(),
                    });
                }
            }
        }
        Ok(())
    }

    /// Get all detected patterns
    pub fn get_patterns(&self) -> &[DetectedPattern] {
        &self.patterns
    }

    /// Check if an address has a detected pattern
    pub fn has_pattern_at(&self, address: u32) -> Option<&DetectedPattern> {
        self.patterns.iter().find(|p| p.address == address)
    }

    /// Get patterns of a specific type
    pub fn get_patterns_of_type(&self, pattern_type: PatternType) -> Vec<&DetectedPattern> {
        self.patterns.iter()
            .filter(|p| p.pattern_type == pattern_type)
            .collect()
    }
}

/// Reconstruct jump table from detected pattern
pub fn reconstruct_jump_table(
    instructions: &[DecodedInstruction],
    pattern: &DetectedPattern,
) -> Result<Vec<u32>> {
    // Find the mtctr instruction
    let mtctr_inst = instructions.iter()
        .find(|i| i.address == pattern.address)
        .ok_or_else(|| anyhow::anyhow!("Instruction not found"))?;
    
    // Extract the register containing the index
    // Then look for the table in memory
    // This is a simplified version - full implementation would analyze memory
    
    Ok(Vec::new())
}

