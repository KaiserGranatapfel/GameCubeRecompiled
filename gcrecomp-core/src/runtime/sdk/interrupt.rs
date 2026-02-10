/// GameCube interrupt system emulation.
///
/// The GameCube has 32 interrupt sources managed through a mask register.
/// In a static recompiler context, most interrupts are simulated (VI retrace,
/// AI DMA complete, etc.) rather than triggered by real hardware.
pub struct InterruptSystem {
    master_enable: bool,
    mask: u32,
    pending: u32,
    handlers: [Option<u32>; 32], // GC function addresses for each interrupt
}

impl InterruptSystem {
    pub fn new() -> Self {
        Self {
            master_enable: false,
            mask: 0,
            pending: 0,
            handlers: [None; 32],
        }
    }

    pub fn enabled(&self) -> bool {
        self.master_enable
    }

    pub fn set_master_enable(&mut self, enable: bool) {
        self.master_enable = enable;
    }

    pub fn disable_all(&mut self) {
        self.master_enable = false;
        self.mask = 0;
    }

    /// Enable a specific interrupt source.
    pub fn enable_interrupt(&mut self, irq: u8) {
        if (irq as usize) < 32 {
            self.mask |= 1 << irq;
        }
    }

    /// Disable a specific interrupt source.
    pub fn disable_interrupt(&mut self, irq: u8) {
        if (irq as usize) < 32 {
            self.mask &= !(1 << irq);
        }
    }

    /// Register a handler (GC function address) for an interrupt.
    pub fn set_handler(&mut self, irq: u8, handler: u32) -> Option<u32> {
        if (irq as usize) < 32 {
            let old = self.handlers[irq as usize];
            self.handlers[irq as usize] = Some(handler);
            old
        } else {
            None
        }
    }

    /// Raise an interrupt. Returns the handler address if the interrupt is
    /// enabled and has a handler registered.
    pub fn raise(&mut self, irq: u8) -> Option<u32> {
        if (irq as usize) >= 32 {
            return None;
        }
        self.pending |= 1 << irq;
        if self.master_enable && (self.mask & (1 << irq)) != 0 {
            self.handlers[irq as usize]
        } else {
            None
        }
    }

    /// Acknowledge (clear) a pending interrupt.
    pub fn acknowledge(&mut self, irq: u8) {
        if (irq as usize) < 32 {
            self.pending &= !(1 << irq);
        }
    }

    /// Get pending interrupts masked by the enable mask.
    pub fn get_pending_masked(&self) -> u32 {
        self.pending & self.mask
    }
}

impl Default for InterruptSystem {
    fn default() -> Self {
        Self::new()
    }
}
