/// Arena allocator matching the GameCube OS memory model.
///
/// The GameCube arena sits between the end of the loaded DOL and the top of MEM1.
/// `lo` grows upward, `hi` grows downward. The two cursors must never cross.
///
/// Address space: 0x80000000..0x817FFFFF (24 MB MEM1)
/// Default arena: lo starts at ~0x80400000 (after typical DOL), hi starts at 0x817FFFFF.
pub struct ArenaAllocator {
    lo: u32,
    hi: u32,
    initial_lo: u32,
    initial_hi: u32,
}

impl ArenaAllocator {
    /// Default arena boundaries (assumes DOL ends around 0x80400000).
    const DEFAULT_LO: u32 = 0x8040_0000;
    const DEFAULT_HI: u32 = 0x817F_FFFF;

    pub fn new() -> Self {
        Self {
            lo: Self::DEFAULT_LO,
            hi: Self::DEFAULT_HI,
            initial_lo: Self::DEFAULT_LO,
            initial_hi: Self::DEFAULT_HI,
        }
    }

    pub fn reset(&mut self) {
        self.lo = self.initial_lo;
        self.hi = self.initial_hi;
    }

    /// Allocate from the low end (grows upward). Returns GC address.
    pub fn alloc_lo(&mut self, size: u32, align: u32) -> u32 {
        let align = if align == 0 { 32 } else { align };
        // Align upward
        let aligned = (self.lo + align - 1) & !(align - 1);
        let end = aligned + size;
        if end > self.hi {
            log::warn!(
                "ArenaAllocator: lo alloc of {} bytes overflows (lo=0x{:08X}, hi=0x{:08X})",
                size,
                self.lo,
                self.hi
            );
            return 0;
        }
        self.lo = end;
        aligned
    }

    /// Allocate from the high end (grows downward). Returns GC address.
    pub fn alloc_hi(&mut self, size: u32, align: u32) -> u32 {
        let align = if align == 0 { 32 } else { align };
        // Align downward
        let end = self.hi.wrapping_sub(size);
        let aligned = end & !(align - 1);
        if aligned < self.lo {
            log::warn!(
                "ArenaAllocator: hi alloc of {} bytes overflows (lo=0x{:08X}, hi=0x{:08X})",
                size,
                self.lo,
                self.hi
            );
            return 0;
        }
        self.hi = aligned;
        aligned
    }

    pub fn lo_cursor(&self) -> u32 {
        self.lo
    }

    pub fn hi_cursor(&self) -> u32 {
        self.hi
    }

    pub fn set_lo_cursor(&mut self, addr: u32) {
        self.lo = addr;
    }

    pub fn set_hi_cursor(&mut self, addr: u32) {
        self.hi = addr;
    }

    /// Set the initial boundaries (e.g. after loading a DOL, the lo start should
    /// be the end of the last loaded section).
    pub fn set_bounds(&mut self, lo: u32, hi: u32) {
        self.initial_lo = lo;
        self.initial_hi = hi;
        self.lo = lo;
        self.hi = hi;
    }
}

impl Default for ArenaAllocator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alloc_lo() {
        let mut arena = ArenaAllocator::new();
        let addr = arena.alloc_lo(256, 32);
        assert_eq!(addr, ArenaAllocator::DEFAULT_LO);
        assert_eq!(arena.lo_cursor(), ArenaAllocator::DEFAULT_LO + 256);
    }

    #[test]
    fn test_alloc_hi() {
        let mut arena = ArenaAllocator::new();
        let addr = arena.alloc_hi(256, 32);
        let expected = (ArenaAllocator::DEFAULT_HI - 256) & !31;
        assert_eq!(addr, expected);
    }

    #[test]
    fn test_arena_overflow() {
        let mut arena = ArenaAllocator::new();
        arena.set_bounds(0x8040_0000, 0x8040_0100);
        let addr = arena.alloc_lo(512, 32);
        assert_eq!(addr, 0); // Should fail
    }
}
