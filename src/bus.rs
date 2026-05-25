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
    joyp_select: u8,
    joyp_direction_pressed: u8,
    joyp_button_pressed: u8,
}

const JOYP_ADDR: u16 = 0xFF00;
const DIV_ADDR: u16 = 0xFF04;
const TIMA_ADDR: u16 = 0xFF05;
const TMA_ADDR: u16 = 0xFF06;
const TAC_ADDR: u16 = 0xFF07;
const IF_ADDR: u16 = 0xFF0F;
const IE_ADDR: u16 = 0xFFFF;
const INTERRUPT_MASK: u8 = 0x1F;
const JOYPAD_INTERRUPT_BIT: u8 = 0b0001_0000;
const TIMER_INTERRUPT_BIT: u8 = 0b0000_0100;
const JOYP_SELECT_MASK: u8 = 0b0011_0000;
const JOYP_LOW_MASK: u8 = 0b0000_1111;
const JOYP_UNUSED_MASK: u8 = 0b1100_0000;
const JOYP_SELECT_DIRECTIONS: u8 = 0b0001_0000;
const JOYP_SELECT_BUTTONS: u8 = 0b0010_0000;

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
            joyp_select: JOYP_SELECT_MASK,
            joyp_direction_pressed: 0,
            joyp_button_pressed: 0,
        }
    }

    /// Read a byte from the 64KB address space.
    #[must_use]
    pub fn read(&self, address: u16) -> u8 {
        match address {
            JOYP_ADDR => self.joyp_value(),
            _ => self.memory[usize::from(address)],
        }
    }

    /// Write a byte into the 64KB address space.
    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            JOYP_ADDR => {
                let previous_low = self.joyp_low_nibble();
                self.joyp_select = value & JOYP_SELECT_MASK;
                let current_low = self.joyp_low_nibble();
                self.request_joypad_interrupt_on_falling_edge(previous_low, current_low);
            }
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

    /// Update both joypad rows atomically.
    ///
    /// Bit mapping for each mask is:
    /// bit0 = Right / A, bit1 = Left / B, bit2 = Up / Select, bit3 = Down / Start.
    /// In both masks, bit 1 means pressed.
    pub fn set_joypad_pressed(&mut self, direction_pressed_mask: u8, button_pressed_mask: u8) {
        let previous_low = self.joyp_low_nibble();
        self.joyp_direction_pressed = direction_pressed_mask & JOYP_LOW_MASK;
        self.joyp_button_pressed = button_pressed_mask & JOYP_LOW_MASK;
        let current_low = self.joyp_low_nibble();
        self.request_joypad_interrupt_on_falling_edge(previous_low, current_low);
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

    #[must_use]
    fn joyp_value(&self) -> u8 {
        JOYP_UNUSED_MASK | self.joyp_select | self.joyp_low_nibble()
    }

    #[must_use]
    fn joyp_low_nibble(&self) -> u8 {
        let mut low = JOYP_LOW_MASK;
        if self.joyp_select & JOYP_SELECT_DIRECTIONS == 0 {
            low &= !self.joyp_direction_pressed & JOYP_LOW_MASK;
        }
        if self.joyp_select & JOYP_SELECT_BUTTONS == 0 {
            low &= !self.joyp_button_pressed & JOYP_LOW_MASK;
        }
        low
    }

    fn request_joypad_interrupt_on_falling_edge(&mut self, previous_low: u8, current_low: u8) {
        if previous_low & !current_low != 0 {
            self.request_interrupt(JOYPAD_INTERRUPT_BIT);
        }
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
    fn test_joyp_default_value() {
        let bus = Bus::new();
        assert_eq!(bus.read(0xFF00), 0xFF);
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

    #[test]
    fn test_joyp_read_reflects_selected_direction_keys() {
        let mut bus = Bus::new();
        bus.write(0xFF00, 0b0010_0000); // Select directions (P14)
        bus.set_joypad_pressed(0b0101, 0b0011); // Right + Up; buttons ignored while unselected

        assert_eq!(bus.read(0xFF00), 0b1110_1010);
    }

    #[test]
    fn test_joyp_read_reflects_selected_button_keys() {
        let mut bus = Bus::new();
        bus.write(0xFF00, 0b0001_0000); // Select buttons (P15)
        bus.set_joypad_pressed(0b0011, 0b1010); // B + Start; directions ignored while unselected

        assert_eq!(bus.read(0xFF00), 0b1101_0101);
    }

    #[test]
    fn test_joyp_read_reflects_both_selected_rows() {
        let mut bus = Bus::new();
        bus.write(0xFF00, 0b0000_0000); // Select both rows
        bus.set_joypad_pressed(0b0001, 0b0100); // Right + Select

        assert_eq!(bus.read(0xFF00), 0b1100_1010);
    }

    #[test]
    fn test_joyp_write_ignores_low_nibble_and_forces_upper_bits() {
        let mut bus = Bus::new();

        bus.write(0xFF00, 0b1010_0110);
        assert_eq!(bus.read(0xFF00), 0b1110_1111);

        bus.set_joypad_pressed(0b0001, 0);
        assert_eq!(bus.read(0xFF00), 0b1110_1110);
    }

    #[test]
    fn test_joyp_low_nibble_write_does_not_request_interrupt_without_select_change() {
        let mut bus = Bus::new();
        bus.write(0xFF00, 0b0010_0000); // Select directions (P14)
        bus.set_joypad_pressed(0b0001, 0); // Right pressed
        bus.write(0xFF0F, 0);

        bus.write(0xFF00, 0b0010_1111); // Same select bits, low nibble ignored

        assert!(!bus.joypad_interrupt_requested());
    }

    #[test]
    fn test_joypad_interrupt_requested_on_selected_press_edge() {
        let mut bus = Bus::new();
        bus.write(0xFF00, 0b0010_0000); // Select directions (P14)
        bus.write(0xFF0F, 0);

        bus.set_joypad_pressed(0b0001, 0); // Right pressed

        assert!(bus.joypad_interrupt_requested());
    }

    #[test]
    fn test_joypad_interrupt_not_requested_on_release() {
        let mut bus = Bus::new();
        bus.write(0xFF00, 0b0010_0000); // Select directions (P14)

        bus.set_joypad_pressed(0b0001, 0);
        assert!(bus.joypad_interrupt_requested());

        bus.write(0xFF0F, 0);
        bus.set_joypad_pressed(0, 0);

        assert!(!bus.joypad_interrupt_requested());
    }

    #[test]
    fn test_joypad_interrupt_not_re_requested_for_idempotent_state() {
        let mut bus = Bus::new();
        bus.write(0xFF00, 0b0010_0000); // Select directions (P14)
        bus.write(0xFF0F, 0);

        bus.set_joypad_pressed(0b0001, 0);
        assert!(bus.joypad_interrupt_requested());

        bus.write(0xFF0F, 0);
        bus.set_joypad_pressed(0b0001, 0);

        assert!(!bus.joypad_interrupt_requested());
    }

    #[test]
    fn test_set_joypad_pressed_atomic_swap_does_not_create_false_edge() {
        let mut bus = Bus::new();
        bus.write(0xFF00, 0b0000_0000); // Select both rows
        bus.set_joypad_pressed(0b0001, 0); // Right pressed in directions row
        bus.write(0xFF0F, 0);

        bus.set_joypad_pressed(0, 0b0001); // Same logical line stays low via buttons row

        assert!(!bus.joypad_interrupt_requested());
    }

    #[test]
    fn test_joypad_interrupt_not_requested_for_unselected_row_press() {
        let mut bus = Bus::new();
        bus.write(0xFF00, 0b0001_0000); // Select buttons (P15)
        bus.write(0xFF0F, 0);

        bus.set_joypad_pressed(0b0001, 0); // Right pressed on unselected row

        assert!(!bus.joypad_interrupt_requested());
    }

    #[test]
    fn test_joypad_interrupt_requested_on_select_change_falling_edge() {
        let mut bus = Bus::new();
        bus.write(0xFF00, 0b0011_0000); // No row selected
        bus.write(0xFF0F, 0);

        bus.set_joypad_pressed(0b0001, 0); // Right pressed while unselected
        assert!(!bus.joypad_interrupt_requested());

        bus.write(0xFF00, 0b0010_0000); // Select directions (P14)

        assert!(bus.joypad_interrupt_requested());
    }

    #[test]
    fn test_joypad_interrupt_not_requested_on_select_change_rising_edge() {
        let mut bus = Bus::new();
        bus.write(0xFF00, 0b0010_0000); // Select directions (P14)
        bus.set_joypad_pressed(0b0001, 0); // Right pressed while selected
        bus.write(0xFF0F, 0);

        bus.write(0xFF00, 0b0011_0000); // Deselect directions -> low nibble rises

        assert!(!bus.joypad_interrupt_requested());
    }

    #[test]
    fn test_joypad_interrupt_not_requested_when_select_switch_keeps_low_state() {
        let mut bus = Bus::new();
        bus.write(0xFF00, 0b0010_0000); // Select directions (P14)
        bus.set_joypad_pressed(0b0001, 0b0001); // Matching bit low in both rows
        bus.write(0xFF0F, 0);

        bus.write(0xFF00, 0b0001_0000); // Switch to buttons (P15)

        assert!(!bus.joypad_interrupt_requested());
    }

    #[test]
    fn test_joypad_interrupt_not_requested_when_select_write_unchanged() {
        let mut bus = Bus::new();
        bus.write(0xFF00, 0b0010_0000); // Select directions (P14)
        bus.set_joypad_pressed(0b0001, 0);
        bus.write(0xFF0F, 0);

        bus.write(0xFF00, 0b0010_0000); // Same select value

        assert!(!bus.joypad_interrupt_requested());
    }
}
