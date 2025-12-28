//! DSP (Digital Signal Processor) emulation
//!
//! The GameCube DSP is a 16-bit fixed-point processor with 32KB program RAM.
//! It handles audio processing, effects, and mixing.

use anyhow::Result;

/// DSP state
#[derive(Debug)]
pub struct DSP {
    /// Program RAM (32KB)
    program_ram: Vec<u8>,
    /// Data RAM (32KB)
    data_ram: Vec<u8>,
    /// DSP registers
    registers: [u16; 32],
    /// Program counter
    pc: u16,
    /// Running state
    running: bool,
}

impl DSP {
    /// Create a new DSP instance
    pub fn new() -> Self {
        Self {
            program_ram: vec![0; 32 * 1024], // 32KB
            data_ram: vec![0; 32 * 1024],    // 32KB
            registers: [0; 32],
            pc: 0,
            running: false,
        }
    }

    /// Initialize the DSP
    pub fn init(&mut self) -> Result<()> {
        log::info!("DSP initialized");
        self.running = false;
        self.pc = 0;
        Ok(())
    }

    /// Load DSP program
    pub fn load_program(&mut self, data: &[u8]) -> Result<()> {
        if data.len() > self.program_ram.len() {
            anyhow::bail!("DSP program too large: {} bytes", data.len());
        }
        self.program_ram[..data.len()].copy_from_slice(data);
        log::debug!("DSP program loaded: {} bytes", data.len());
        Ok(())
    }

    /// Start DSP execution
    pub fn start(&mut self) {
        self.running = true;
        self.pc = 0;
        log::debug!("DSP started");
    }

    /// Stop DSP execution
    pub fn stop(&mut self) {
        self.running = false;
        log::debug!("DSP stopped");
    }

    /// Process one DSP instruction
    ///
    /// Decodes and executes a single 16-bit DSP instruction.
    /// DSP instructions are similar to PowerPC but simplified for audio processing.
    pub fn step(&mut self) -> Result<()> {
        if !self.running {
            return Ok(());
        }

        // Check bounds
        if self.pc as usize + 2 > self.program_ram.len() {
            self.running = false;
            return Ok(());
        }

        // Read 16-bit instruction (big-endian)
        let instruction = u16::from_be_bytes([
            self.program_ram[self.pc as usize],
            self.program_ram[self.pc as usize + 1],
        ]);

        // Decode instruction opcode (top 6 bits)
        let opcode = (instruction >> 10) & 0x3F;
        
        // Execute instruction based on opcode
        match opcode {
            0x00..=0x0F => {
                // Arithmetic operations (ADD, SUB, MUL, etc.)
                self.execute_arithmetic(instruction)?;
            }
            0x10..=0x1F => {
                // Load/store operations
                self.execute_load_store(instruction)?;
            }
            0x20..=0x2F => {
                // Branch operations
                self.execute_branch(instruction)?;
            }
            0x30..=0x3F => {
                // Special operations (NOP, HALT, etc.)
                self.execute_special(instruction)?;
            }
            _ => {
                log::warn!("Unknown DSP instruction: 0x{:04X} at PC=0x{:04X}", instruction, self.pc);
                self.pc = self.pc.wrapping_add(2);
            }
        }

        Ok(())
    }

    /// Execute arithmetic instruction
    fn execute_arithmetic(&mut self, instruction: u16) -> Result<()> {
        let opcode = (instruction >> 10) & 0x3F;
        let rd = ((instruction >> 5) & 0x1F) as usize; // Destination register
        let rs = (instruction & 0x1F) as usize; // Source register

        if rd >= 32 || rs >= 32 {
            anyhow::bail!("Invalid register index: rd={}, rs={}", rd, rs);
        }

        match opcode {
            0x00 => {
                // ADD: rd = rd + rs
                let result = self.registers[rd].wrapping_add(self.registers[rs]);
                self.registers[rd] = result;
            }
            0x01 => {
                // SUB: rd = rd - rs
                let result = self.registers[rd].wrapping_sub(self.registers[rs]);
                self.registers[rd] = result;
            }
            0x02 => {
                // MUL: rd = rd * rs (16-bit multiply, result in 32-bit, take low 16)
                let result = (self.registers[rd] as u32).wrapping_mul(self.registers[rs] as u32);
                self.registers[rd] = (result & 0xFFFF) as u16;
            }
            0x03 => {
                // AND: rd = rd & rs
                self.registers[rd] &= self.registers[rs];
            }
            0x04 => {
                // OR: rd = rd | rs
                self.registers[rd] |= self.registers[rs];
            }
            0x05 => {
                // XOR: rd = rd ^ rs
                self.registers[rd] ^= self.registers[rs];
            }
            _ => {
                log::debug!("Unimplemented arithmetic opcode: 0x{:02X}", opcode);
            }
        }

        self.pc = self.pc.wrapping_add(2);
        Ok(())
    }

    /// Execute load/store instruction
    fn execute_load_store(&mut self, instruction: u16) -> Result<()> {
        let opcode = (instruction >> 10) & 0x3F;
        let rd = ((instruction >> 5) & 0x1F) as usize;
        let offset = (instruction & 0x1F) as u16; // 5-bit offset

        if rd >= 32 {
            anyhow::bail!("Invalid register index: rd={}", rd);
        }

        match opcode {
            0x10 => {
                // LOAD: rd = data_ram[offset]
                let addr = offset as usize;
                if addr + 1 < self.data_ram.len() {
                    let value = u16::from_be_bytes([self.data_ram[addr], self.data_ram[addr + 1]]);
                    self.registers[rd] = value;
                }
            }
            0x11 => {
                // STORE: data_ram[offset] = rd
                let addr = offset as usize;
                if addr + 1 < self.data_ram.len() {
                    let bytes = self.registers[rd].to_be_bytes();
                    self.data_ram[addr] = bytes[0];
                    self.data_ram[addr + 1] = bytes[1];
                }
            }
            _ => {
                log::debug!("Unimplemented load/store opcode: 0x{:02X}", opcode);
            }
        }

        self.pc = self.pc.wrapping_add(2);
        Ok(())
    }

    /// Execute branch instruction
    fn execute_branch(&mut self, instruction: u16) -> Result<()> {
        let opcode = (instruction >> 10) & 0x3F;
        let condition = (instruction >> 8) & 0x3;
        let target = instruction & 0xFF; // 8-bit target offset

        let should_branch = match condition {
            0 => true, // Always branch
            1 => (self.registers[0] as i16) < 0, // Branch if negative
            2 => (self.registers[0] as i16) == 0, // Branch if zero
            3 => (self.registers[0] as i16) > 0, // Branch if positive
            _ => false,
        };

        if should_branch {
            match opcode {
                0x20 => {
                    // BRANCH: pc = target
                    self.pc = target;
                }
                0x21 => {
                    // BRANCH_LINK: save pc, then branch
                    self.registers[31] = self.pc; // Save return address in r31
                    self.pc = target;
                }
                _ => {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
        } else {
            self.pc = self.pc.wrapping_add(2);
        }

        Ok(())
    }

    /// Execute special instruction
    fn execute_special(&mut self, instruction: u16) -> Result<()> {
        let opcode = (instruction >> 10) & 0x3F;

        match opcode {
            0x30 => {
                // NOP: No operation
                self.pc = self.pc.wrapping_add(2);
            }
            0x31 => {
                // HALT: Stop execution
                self.running = false;
            }
            _ => {
                log::debug!("Unimplemented special opcode: 0x{:02X}", opcode);
                self.pc = self.pc.wrapping_add(2);
            }
        }

        Ok(())
    }

    /// Process audio samples
    ///
    /// Processes input samples through the DSP pipeline and outputs processed samples.
    /// The DSP applies effects, mixing, and other audio processing.
    pub fn process_audio(&mut self, input: &[i16], output: &mut [i16]) -> Result<()> {
        if !self.running {
            // If DSP is not running, output silence
            for sample in output.iter_mut() {
                *sample = 0;
            }
            return Ok(());
        }

        // Process samples through DSP pipeline
        // For now, implement a simple pass-through with optional effects
        let input_len = input.len().min(output.len());
        
        // Read input samples from data RAM or use provided input
        // In a real implementation, input would come from ARAM or be provided via registers
        for i in 0..input_len {
            let input_sample = if i < input.len() {
                input[i]
            } else {
                0i16
            };

            // Apply basic DSP processing
            // In a full implementation, this would:
            // 1. Apply filters (low-pass, high-pass, etc.)
            // 2. Apply effects (reverb, echo, etc.)
            // 3. Mix channels
            // 4. Apply volume/gain
            
            // For now, simple pass-through with optional gain
            // Gain is stored in register 1 (as fixed-point value)
            let gain = self.registers[1] as i16;
            let processed = if gain != 0 {
                // Apply gain (simplified - would use fixed-point math)
                ((input_sample as i32 * gain as i32) >> 8) as i16
            } else {
                input_sample
            };

            // Clamp to 16-bit range
            output[i] = processed.max(-32768).min(32767);
        }

        // Fill remaining output with silence if needed
        for i in input_len..output.len() {
            output[i] = 0;
        }

        Ok(())
    }

    /// Check if DSP is running
    pub fn is_running(&self) -> bool {
        self.running
    }
}

impl Default for DSP {
    fn default() -> Self {
        Self::new()
    }
}

