/// Flat 64KB memory buffer backing the Game Boy address space.
///
/// Uses an inline `[u8; 0x10000]` for zero-indirection O(1) access.
///
/// The array is created on the stack inside `new()`. For most use cases
/// (including tests) this is fine, but if stack pressure becomes an issue
/// the field can be changed to `Box<[u8; 0x10000]>` with minimal churn.
#[derive(Debug)]
pub struct Bus {
    memory: [u8; 0x10000],
    div_cycles: u16,
    div_value: u8,
    tima_cycles: u16,
    last_tac: u8,
}

const DIV_ADDR: u16 = 0xFF04;
const TIMA_ADDR: u16 = 0xFF05;
const TMA_ADDR: u16 = 0xFF06;
const TAC_ADDR: u16 = 0xFF07;
const IF_ADDR: u16 = 0xFF0F;
const IE_ADDR: u16 = 0xFFFF;
const INTERRUPT_MASK: u8 = 0x1F;
const JOYPAD_INTERRUPT_BIT: u8 = 0b0001_0000;
const TIMER_INTERRUPT_BIT: u8 = 0b0000_0100;

impl Default for Bus {
    fn default() -> Self {
        Self::new()
    }
}

impl Bus {
    /// Create a new bus with all bytes initialised to zero.
    #[must_use]
    #[allow(clippy::large_stack_arrays)]
    pub fn new() -> Self {
        Bus {
            memory: [0; 0x10000],
            div_cycles: 0,
            div_value: 0,
            tima_cycles: 0,
            last_tac: 0,
        }
    }

    /// Read a byte from the 64KB address space.
    #[must_use]
    pub fn read(&self, address: u16) -> u8 {
        self.memory[usize::from(address)]
    }

    /// Write a byte into the 64KB address space.
    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            DIV_ADDR => {
                // Writing any value to DIV resets it to 0.
                self.div_cycles = 0;
                self.div_value = 0;
                self.memory[usize::from(DIV_ADDR)] = 0;
            }
            TAC_ADDR => {
                let normalized = value & 0b0000_0111;
                if normalized != self.last_tac {
                    self.tima_cycles = 0;
                    self.last_tac = normalized;
                }
                self.memory[usize::from(TAC_ADDR)] = value;
            }
            _ => {
                self.memory[usize::from(address)] = value;
            }
        }
    }

    /// Tick timer registers by instruction cycles.
    pub fn tick_timers(&mut self, cycles: u8) {
        let cycles = u16::from(cycles);

        self.div_cycles = self.div_cycles.wrapping_add(cycles);
        while self.div_cycles >= 256 {
            self.div_cycles -= 256;
            self.div_value = self.div_value.wrapping_add(1);
            self.memory[usize::from(DIV_ADDR)] = self.div_value;
        }

        let tac = self.read(TAC_ADDR);
        if tac & 0b0000_0100 == 0 {
            return;
        }

        self.tima_cycles = self.tima_cycles.wrapping_add(cycles);
        let period = timer_period(tac);

        while self.tima_cycles >= period {
            self.tima_cycles -= period;

            let tima_value = self.read(TIMA_ADDR);
            if tima_value == 0xFF {
                let reload_value = self.read(TMA_ADDR);
                self.memory[usize::from(TIMA_ADDR)] = reload_value;
                self.request_interrupt(TIMER_INTERRUPT_BIT);
            } else {
                self.memory[usize::from(TIMA_ADDR)] = tima_value.wrapping_add(1);
            }
        }
    }

    #[must_use]
    pub fn pending_interrupts(&self) -> u8 {
        (self.read(IE_ADDR) & self.read(IF_ADDR)) & INTERRUPT_MASK
    }

    #[must_use]
    pub fn requested_interrupts(&self) -> u8 {
        self.read(IF_ADDR) & INTERRUPT_MASK
    }

    #[must_use]
    pub fn joypad_interrupt_requested(&self) -> bool {
        self.requested_interrupts() & JOYPAD_INTERRUPT_BIT != 0
    }

    pub fn clear_interrupt(&mut self, mask: u8) {
        let iflags = self.read(IF_ADDR);
        self.memory[usize::from(IF_ADDR)] = iflags & !(mask & INTERRUPT_MASK);
    }

    pub fn request_interrupt(&mut self, mask: u8) {
        let iflags = self.read(IF_ADDR);
        self.memory[usize::from(IF_ADDR)] = iflags | (mask & INTERRUPT_MASK);
    }
}

const fn timer_period(tac: u8) -> u16 {
    match tac & 0b0000_0011 {
        0b00 => 1024,
        0b01 => 16,
        0b10 => 64,
        0b11 => 256,
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bus_read_write() {
        let mut bus = Bus::new();
        bus.write(0x1234, 0xAB);
        assert_eq!(bus.read(0x1234), 0xAB);
    }

    #[test]
    fn test_bus_initially_zero() {
        let bus = Bus::new();
        assert_eq!(bus.read(0x0000), 0x00);
        assert_eq!(bus.read(0xFFFF), 0x00);
    }

    #[test]
    fn test_requested_interrupts_masks_upper_bits() {
        let mut bus = Bus::new();
        bus.write(0xFF0F, 0b1110_0101);
        assert_eq!(bus.requested_interrupts(), 0b0000_0101);
    }

    #[test]
    fn test_joypad_interrupt_requested() {
        let mut bus = Bus::new();
        assert!(!bus.joypad_interrupt_requested());

        bus.write(0xFF0F, 0b0001_0000);
        assert!(bus.joypad_interrupt_requested());

        bus.write(0xFF0F, 0b0000_0100);
        assert!(!bus.joypad_interrupt_requested());
    }
}
