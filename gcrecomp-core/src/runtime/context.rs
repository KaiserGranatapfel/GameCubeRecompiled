// CPU context
#[derive(Debug, Clone)]
pub struct CpuContext {
    pub gpr: [u32; 32], // General Purpose Registers (r0-r31)
    pub pc: u32,        // Program Counter
    pub lr: u32,        // Link Register
    pub ctr: u32,       // Count Register
    pub cr: u32,        // Condition Register
    pub xer: u32,       // Fixed-Point Exception Register
    pub fpscr: u32,     // Floating-Point Status and Control Register
    pub fpr: [f64; 32], // Floating-Point Registers
    pub msr: u32,       // Machine State Register
}

impl CpuContext {
    pub fn new() -> Self {
        Self {
            gpr: [0; 32],
            pc: 0,
            lr: 0,
            ctr: 0,
            cr: 0,
            xer: 0,
            fpscr: 0,
            fpr: [0.0; 32],
            msr: 0,
        }
    }

    pub fn get_register(&self, reg: u8) -> u32 {
        if reg < 32 {
            self.gpr[reg as usize]
        } else {
            0
        }
    }

    pub fn set_register(&mut self, reg: u8, value: u32) {
        if reg < 32 {
            self.gpr[reg as usize] = value;
        }
    }

    pub fn get_cr_field(&self, field: u8) -> u8 {
        if field < 8 {
            ((self.cr >> (4 * field)) & 0xF) as u8
        } else {
            0
        }
    }

    pub fn set_cr_field(&mut self, field: u8, value: u8) {
        if field < 8 {
            let mask = !(0xF << (4 * field));
            self.cr = (self.cr & mask) | ((value as u32 & 0xF) << (4 * field));
        }
    }

    pub fn get_fpr(&self, reg: u8) -> f64 {
        if reg < 32 {
            self.fpr[reg as usize]
        } else {
            0.0
        }
    }

    pub fn set_fpr(&mut self, reg: u8, value: f64) {
        if reg < 32 {
            self.fpr[reg as usize] = value;
        }
    }
}

impl Default for CpuContext {
    fn default() -> Self {
        Self::new()
    }
}
