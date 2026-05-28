/// Flat 64KB memory buffer backing the Game Boy address space.
///
/// Uses an inline `[u8; 0x10000]` for zero-indirection O(1) access.
///
/// The array is created on the stack inside `new()`. For most use cases
/// (including tests) this is fine, but if stack pressure becomes an issue
/// the field can be changed to `Box<[u8; 0x10000]>` with minimal churn.
use crate::cartridge::Cartridge;

#[derive(Debug)]
pub struct Bus {
    memory: [u8; 0x10000],
    cartridge: Option<Cartridge>,
    div_cycles: u16,
    div_value: u8,
    tima_cycles: u16,
    last_tac: u8,
    joyp_select: u8,
    joyp_direction_pressed: u8,
    joyp_button_pressed: u8,
    dma_active: bool,
    dma_src_high: u8,
    dma_index: u8,
}

const JOYP_ADDR: u16 = 0xFF00;
const DIV_ADDR: u16 = 0xFF04;
const TIMA_ADDR: u16 = 0xFF05;
const TMA_ADDR: u16 = 0xFF06;
const TAC_ADDR: u16 = 0xFF07;
const IF_ADDR: u16 = 0xFF0F;
const IE_ADDR: u16 = 0xFFFF;
const ECHO_RAM_START: u16 = 0xE000;
const ECHO_RAM_END: u16 = 0xFDFF;
const ECHO_RAM_OFFSET: u16 = 0x2000;
const UNUSABLE_START: u16 = 0xFEA0;
const UNUSABLE_END: u16 = 0xFEFF;
// Temporary scaffold policy for the unusable FEA0-FEFF range.
// Hardware behavior varies by model and timing; refine once PPU/MMIO timing is modeled.
const UNUSABLE_READ_VALUE: u8 = 0xFF;
const INTERRUPT_MASK: u8 = 0x1F;
const JOYPAD_INTERRUPT_BIT: u8 = 0b0001_0000;
const TIMER_INTERRUPT_BIT: u8 = 0b0000_0100;
const JOYP_SELECT_MASK: u8 = 0b0011_0000;
const JOYP_LOW_MASK: u8 = 0b0000_1111;
const JOYP_UNUSED_MASK: u8 = 0b1100_0000;
const JOYP_SELECT_DIRECTIONS: u8 = 0b0001_0000;
const JOYP_SELECT_BUTTONS: u8 = 0b0010_0000;
const DMA_ADDR: u16 = 0xFF46;
const OAM_START: u16 = 0xFE00;
const OAM_LEN: u8 = 160;
const HRAM_START: u16 = 0xFF80;
const HRAM_END: u16 = 0xFFFE;
const ROM_END: u16 = 0x7FFF;
const EXTERNAL_RAM_START: u16 = 0xA000;
const EXTERNAL_RAM_END: u16 = 0xBFFF;

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
            cartridge: None,
            div_cycles: 0,
            div_value: 0,
            tima_cycles: 0,
            last_tac: 0,
            joyp_select: JOYP_SELECT_MASK,
            joyp_direction_pressed: 0,
            joyp_button_pressed: 0,
            dma_active: false,
            dma_src_high: 0,
            dma_index: 0,
        }
    }

    /// Load a cartridge from raw ROM bytes.
    ///
    /// Parses the ROM header and configures MBC1 banking state. Bank 0
    /// is not copied into `memory` — ROM reads are delegated to the
    /// cartridge from this point forward.
    pub fn load_cartridge(&mut self, data: &[u8]) {
        self.cartridge = Some(Cartridge::from_bytes(data));
    }

    /// Whether a cartridge has been loaded.
    #[must_use]
    pub fn has_cartridge(&self) -> bool {
        self.cartridge.is_some()
    }

    /// Read a byte from the 64KB address space.
    #[must_use]
    pub fn read(&self, address: u16) -> u8 {
        if self.dma_active && !Self::is_dma_bus_allowed(address) {
            return 0xFF;
        }
        match address {
            0x0000..=ROM_END => {
                if let Some(ref cart) = self.cartridge {
                    cart.read_rom(address)
                } else {
                    self.memory[usize::from(address)]
                }
            }
            EXTERNAL_RAM_START..=EXTERNAL_RAM_END => {
                if let Some(ref cart) = self.cartridge {
                    cart.read_ram(address)
                } else {
                    0xFF
                }
            }
            JOYP_ADDR => self.joyp_value(),
            UNUSABLE_START..=UNUSABLE_END => UNUSABLE_READ_VALUE,
            ECHO_RAM_START..=ECHO_RAM_END => {
                self.memory[usize::from(Self::mirror_echo_address(address))]
            }
            _ => self.memory[usize::from(address)],
        }
    }

    /// Write a byte into the 64KB address space.
    pub fn write(&mut self, address: u16, value: u8) {
        if address == DMA_ADDR {
            self.dma_active = true;
            self.dma_src_high = value;
            self.dma_index = 0;
            self.memory[usize::from(DMA_ADDR)] = value;
            return;
        }
        if self.dma_active && !Self::is_dma_bus_allowed(address) {
            return;
        }
        match address {
            0x0000..=ROM_END => {
                if let Some(ref mut cart) = self.cartridge {
                    cart.write_register(address, value);
                } else {
                    self.memory[usize::from(address)] = value;
                }
            }
            EXTERNAL_RAM_START..=EXTERNAL_RAM_END => {
                if let Some(ref mut cart) = self.cartridge {
                    cart.write_ram(address, value);
                }
            }
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
            UNUSABLE_START..=UNUSABLE_END => {}
            ECHO_RAM_START..=ECHO_RAM_END => {
                self.memory[usize::from(Self::mirror_echo_address(address))] = value;
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

    /// Tick timer registers and DMA transfer by instruction cycles.
    pub fn tick_timers(&mut self, cycles: u8) {
        self.tick_dma(cycles);

        let cycles = u16::from(cycles);

        self.div_cycles = self.div_cycles.wrapping_add(cycles);
        while self.div_cycles >= 256 {
            self.div_cycles -= 256;
            self.div_value = self.div_value.wrapping_add(1);
            self.memory[usize::from(DIV_ADDR)] = self.div_value;
        }

        let tac = self.memory[usize::from(TAC_ADDR)];
        if tac & 0b0000_0100 == 0 {
            return;
        }

        self.tima_cycles = self.tima_cycles.wrapping_add(cycles);
        let period = timer_period(tac);

        while self.tima_cycles >= period {
            self.tima_cycles -= period;

            let tima_value = self.memory[usize::from(TIMA_ADDR)];
            if tima_value == 0xFF {
                let reload_value = self.memory[usize::from(TMA_ADDR)];
                self.memory[usize::from(TIMA_ADDR)] = reload_value;
                self.request_interrupt(TIMER_INTERRUPT_BIT);
            } else {
                self.memory[usize::from(TIMA_ADDR)] = tima_value.wrapping_add(1);
            }
        }
    }

    #[must_use]
    pub fn pending_interrupts(&self) -> u8 {
        (self.memory[usize::from(IE_ADDR)] & self.memory[usize::from(IF_ADDR)]) & INTERRUPT_MASK
    }

    #[must_use]
    pub fn requested_interrupts(&self) -> u8 {
        self.memory[usize::from(IF_ADDR)] & INTERRUPT_MASK
    }

    #[must_use]
    pub fn joypad_interrupt_requested(&self) -> bool {
        self.requested_interrupts() & JOYPAD_INTERRUPT_BIT != 0
    }

    pub fn clear_interrupt(&mut self, mask: u8) {
        let iflags = self.memory[usize::from(IF_ADDR)];
        self.memory[usize::from(IF_ADDR)] = iflags & !(mask & INTERRUPT_MASK);
    }

    pub fn request_interrupt(&mut self, mask: u8) {
        let iflags = self.memory[usize::from(IF_ADDR)];
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

    #[must_use]
    fn is_dma_bus_allowed(address: u16) -> bool {
        (HRAM_START..=HRAM_END).contains(&address) || address == IE_ADDR
    }

    fn tick_dma(&mut self, cycles: u8) {
        if !self.dma_active {
            return;
        }
        for _ in 0..cycles {
            if self.dma_index >= OAM_LEN {
                self.dma_active = false;
                return;
            }
            let src = (u16::from(self.dma_src_high) << 8) | u16::from(self.dma_index);
            let byte = self.read_for_dma(src);
            self.memory[usize::from(OAM_START + u16::from(self.dma_index))] = byte;
            self.dma_index += 1;
        }
        if self.dma_index >= OAM_LEN {
            self.dma_active = false;
        }
    }

    /// Read a byte for DMA transfer, bypassing DMA bus restrictions.
    ///
    /// Delegates to the cartridge for ROM-space reads when a cartridge
    /// is loaded, so DMA can correctly copy from switchable ROM banks.
    #[must_use]
    fn read_for_dma(&self, address: u16) -> u8 {
        if let Some(ref cart) = self.cartridge {
            if address <= ROM_END {
                return cart.read_rom(address);
            }
            if (EXTERNAL_RAM_START..=EXTERNAL_RAM_END).contains(&address) {
                return cart.read_ram(address);
            }
        }
        self.memory[usize::from(address)]
    }

    #[must_use]
    fn mirror_echo_address(address: u16) -> u16 {
        debug_assert!((ECHO_RAM_START..=ECHO_RAM_END).contains(&address));
        address - ECHO_RAM_OFFSET
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
    fn test_echo_ram_mirrors_wram_reads_and_writes() {
        let mut bus = Bus::new();

        bus.write(0xC123, 0x42);
        assert_eq!(bus.read(0xE123), 0x42);

        bus.write(0xE123, 0x99);
        assert_eq!(bus.read(0xC123), 0x99);
    }

    #[test]
    fn test_echo_ram_mirror_boundary_addresses() {
        let mut bus = Bus::new();

        bus.write(0xE000, 0x12);
        bus.write(0xFDFF, 0x34);

        assert_eq!(bus.read(0xC000), 0x12);
        assert_eq!(bus.read(0xDDFF), 0x34);

        bus.write(0xC000, 0x56);
        bus.write(0xDDFF, 0x78);

        assert_eq!(bus.read(0xE000), 0x56);
        assert_eq!(bus.read(0xFDFF), 0x78);
    }

    #[test]
    fn test_echo_ram_fenceposts_do_not_bleed_into_neighbors() {
        let mut bus = Bus::new();

        bus.write(0xDFFF, 0x11);
        bus.write(0xE000, 0x22);
        bus.write(0xFDFF, 0x33);
        bus.write(0xFE00, 0x44);

        assert_eq!(bus.read(0xDFFF), 0x11);
        assert_eq!(bus.read(0xC000), 0x22);
        assert_eq!(bus.read(0xE000), 0x22);
        assert_eq!(bus.read(0xDDFF), 0x33);
        assert_eq!(bus.read(0xFDFF), 0x33);
        assert_eq!(bus.read(0xFE00), 0x44);
        assert_eq!(bus.read(0xDE00), 0x00);
    }

    #[test]
    fn test_unusable_area_read_policy_and_write_ignored() {
        let mut bus = Bus::new();

        assert_eq!(bus.read(0xFEA0), UNUSABLE_READ_VALUE);
        assert_eq!(bus.read(0xFEFF), UNUSABLE_READ_VALUE);

        bus.write(0xFEA0, 0x12);
        bus.write(0xFEFF, 0x34);

        assert_eq!(bus.read(0xFEA0), UNUSABLE_READ_VALUE);
        assert_eq!(bus.read(0xFEFF), UNUSABLE_READ_VALUE);
    }

    #[test]
    fn test_unusable_area_boundaries_do_not_affect_neighbors() {
        let mut bus = Bus::new();

        let joyp_before = bus.read(0xFF00);
        bus.write(0xFE9F, 0x66);
        assert_eq!(bus.read(0xFEA0), UNUSABLE_READ_VALUE);

        bus.write(0xFEA1, 0x12);
        bus.write(0xFEFF, 0x34);

        assert_eq!(bus.read(0xFE9F), 0x66);
        assert_eq!(bus.read(0xFEA1), UNUSABLE_READ_VALUE);
        assert_eq!(bus.read(0xFEFF), UNUSABLE_READ_VALUE);
        assert_eq!(bus.read(0xFF00), joyp_before);
    }

    #[test]
    fn test_vram_round_trip_scaffolding() {
        let mut bus = Bus::new();

        bus.write(0x8000, 0x11);
        bus.write(0x9FFF, 0x22);

        assert_eq!(bus.read(0x8000), 0x11);
        assert_eq!(bus.read(0x9FFF), 0x22);
    }

    #[test]
    fn test_wram_round_trip_scaffolding() {
        let mut bus = Bus::new();

        bus.write(0xC000, 0x33);
        bus.write(0xDFFF, 0x44);

        assert_eq!(bus.read(0xC000), 0x33);
        assert_eq!(bus.read(0xDFFF), 0x44);
    }

    #[test]
    fn test_oam_round_trip_scaffolding() {
        let mut bus = Bus::new();

        bus.write(0xFE00, 0x55);
        bus.write(0xFE9F, 0x66);

        assert_eq!(bus.read(0xFE00), 0x55);
        assert_eq!(bus.read(0xFE9F), 0x66);
    }

    #[test]
    fn test_hram_round_trip_scaffolding() {
        let mut bus = Bus::new();

        bus.write(0xFF80, 0x77);
        bus.write(0xFFFE, 0x88);

        assert_eq!(bus.read(0xFF80), 0x77);
        assert_eq!(bus.read(0xFFFE), 0x88);
    }

    #[test]
    fn test_serial_and_lcd_register_round_trip_scaffolding() {
        let mut bus = Bus::new();

        // LY (0xFF44) intentionally excluded; future PPU timing should own its semantics.
        let writes = [
            (0xFF01, 0xA1),
            (0xFF02, 0x81),
            (0xFF40, 0x91),
            (0xFF41, 0x85),
            (0xFF42, 0x12),
            (0xFF43, 0x34),
            (0xFF45, 0x78),
            (0xFF4A, 0x9A),
            (0xFF4B, 0xBC),
        ];

        for (address, value) in writes {
            bus.write(address, value);
            assert_eq!(bus.read(address), value);
        }
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

    #[test]
    fn test_oam_dma_transfers_160_bytes() {
        let mut bus = Bus::new();
        for i in 0..160u16 {
            bus.memory[0x0100 + usize::from(i)] = (i & 0xFF) as u8;
        }

        bus.write(DMA_ADDR, 0x01);
        bus.tick_timers(160);

        for i in 0..160u16 {
            assert_eq!(bus.read(0xFE00 + i), (i & 0xFF) as u8);
        }
        assert!(!bus.dma_active);
    }

    #[test]
    fn test_oam_dma_transfers_in_chunks() {
        let mut bus = Bus::new();
        for i in 0..160u16 {
            bus.memory[0x0100 + usize::from(i)] = (i & 0xFF) as u8;
        }

        bus.write(DMA_ADDR, 0x01);
        bus.tick_timers(50);

        for i in 0..50u16 {
            assert_eq!(bus.memory[0xFE00 + usize::from(i)], (i & 0xFF) as u8);
        }
        assert_eq!(bus.memory[0xFE00 + 50], 0x00);
        assert!(bus.dma_active);

        bus.tick_timers(110);
        assert!(!bus.dma_active);
    }

    #[test]
    fn test_oam_dma_retrigger_aborts_previous() {
        let mut bus = Bus::new();
        for i in 0..160u16 {
            bus.memory[0x0100 + usize::from(i)] = 0xAA;
            bus.memory[0x0200 + usize::from(i)] = 0xBB;
        }

        bus.write(DMA_ADDR, 0x01);
        bus.tick_timers(50);

        bus.write(DMA_ADDR, 0x02);
        assert_eq!(bus.dma_index, 0);

        bus.tick_timers(160);
        for i in 0..160u16 {
            assert_eq!(bus.read(0xFE00 + i), 0xBB);
        }
    }

    #[test]
    fn test_oam_dma_bus_restriction_blocks_non_hram_reads() {
        let mut bus = Bus::new();
        bus.memory[0x0100] = 0x42;

        bus.write(DMA_ADDR, 0x01);
        assert!(bus.dma_active);

        assert_eq!(bus.read(0x0100), 0xFF);
        assert_eq!(bus.read(0x8000), 0xFF);
        assert_eq!(bus.read(0xC000), 0xFF);
        assert_eq!(bus.read(0xFE00), 0xFF);
        assert_eq!(bus.read(0xFF00), 0xFF);
    }

    #[test]
    fn test_oam_dma_bus_restriction_allows_hram_reads() {
        let mut bus = Bus::new();
        bus.write(0xFF80, 0x42);
        bus.write(0xFFFF, 0x11);

        bus.write(DMA_ADDR, 0x01);
        assert!(bus.dma_active);

        assert_eq!(bus.read(0xFF80), 0x42);
        assert_eq!(bus.read(0xFFFE), 0x00);
        assert_eq!(bus.read(0xFFFF), 0x11);
    }

    #[test]
    fn test_oam_dma_bus_restriction_blocks_non_hram_writes() {
        let mut bus = Bus::new();
        bus.write(DMA_ADDR, 0x01);
        assert!(bus.dma_active);

        bus.write(0x8000, 0x42);
        bus.write(0xC000, 0x42);
        bus.write(0xFE00, 0x42);

        assert_eq!(bus.memory[0x8000], 0x00);
        assert_eq!(bus.memory[0xC000], 0x00);
        assert_eq!(bus.memory[0xFE00], 0x00);
    }

    #[test]
    fn test_oam_dma_bus_restriction_allows_hram_writes() {
        let mut bus = Bus::new();

        bus.write(DMA_ADDR, 0x01);
        assert!(bus.dma_active);

        bus.write(0xFF80, 0x42);
        assert_eq!(bus.read(0xFF80), 0x42);
    }

    #[test]
    fn test_oam_dma_zero_source() {
        let mut bus = Bus::new();
        for i in 0..160u16 {
            bus.memory[usize::from(i)] = (i & 0xFF) as u8;
        }

        bus.write(DMA_ADDR, 0x00);
        bus.tick_timers(160);

        for i in 0..160u16 {
            assert_eq!(bus.read(0xFE00 + i), (i & 0xFF) as u8);
        }
    }

    #[test]
    fn test_oam_dma_does_not_complete_without_enough_cycles() {
        let mut bus = Bus::new();
        bus.write(DMA_ADDR, 0x01);
        bus.tick_timers(159);

        assert!(bus.dma_active);
        assert_eq!(bus.dma_index, 159);

        bus.tick_timers(1);
        assert!(!bus.dma_active);
        assert_eq!(bus.dma_index, 160);
    }

    #[test]
    fn test_oam_dma_does_not_corrupt_timer_state() {
        let mut bus = Bus::new();
        bus.write(TMA_ADDR, 0x42);
        bus.write(TIMA_ADDR, 0x00);
        bus.write(TAC_ADDR, 0b0000_0101); // enabled, period 16

        bus.write(DMA_ADDR, 0x01);
        bus.tick_timers(16);

        assert_eq!(bus.memory[usize::from(TIMA_ADDR)], 0x01);
        assert_eq!(bus.memory[usize::from(TMA_ADDR)], 0x42);
    }

    #[test]
    fn test_oam_dma_does_not_corrupt_interrupt_flags() {
        let mut bus = Bus::new();
        bus.write(IF_ADDR, 0b0000_0100); // timer interrupt pending
        bus.write(IE_ADDR, 0b0000_0101); // timer + joypad enabled

        bus.write(DMA_ADDR, 0x01);
        bus.tick_timers(1);

        assert_eq!(bus.pending_interrupts(), 0b0000_0100);
        assert_eq!(bus.requested_interrupts(), 0b0000_0100);
    }

    #[test]
    fn test_oam_dma_read_back() {
        let mut bus = Bus::new();
        bus.write(DMA_ADDR, 0xC1);
        // During active DMA, bus.read() returns 0xFF for non-HRAM; verify memory directly.
        assert_eq!(bus.memory[usize::from(DMA_ADDR)], 0xC1);
    }

    fn make_bus_with_mbc1_cart(rom_banks: usize) -> Bus {
        assert!(
            rom_banks.is_power_of_two(),
            "rom_banks must be a power of 2"
        );
        let rom_size = 0x4000 * rom_banks;
        let mut data = vec![0u8; rom_size];
        data[0x0147] = 0x01; // MBC1
        // Header byte N: total size = 32KB * 2^N, so N = log2(rom_banks / 2).
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        {
            data[0x0148] = ((rom_banks / 2) as u32).trailing_zeros() as u8;
        }
        data[0x0149] = 0;
        let mut bus = Bus::new();
        bus.load_cartridge(&data);
        bus
    }

    #[test]
    fn test_bus_has_cartridge_flag() {
        let mut bus = Bus::new();
        assert!(!bus.has_cartridge());
        bus.load_cartridge(&vec![0u8; 0x8000]);
        assert!(bus.has_cartridge());
    }

    #[test]
    fn test_bus_rom_read_delegates_to_cartridge() {
        let mut data = vec![0u8; 0x8000]; // 32KB = 2 banks
        data[0x0147] = 0x01; // MBC1
        data[0x0148] = 0; // 32KB
        data[0x0149] = 0;
        data[0x5234] = 0xAB; // Marker in bank 1
        let mut bus = Bus::new();
        bus.load_cartridge(&data);

        // Bank 0: header type is at offset 0x0147.
        assert_eq!(bus.read(0x0147), 0x01);
        // Bank 1 (switchable, default = bank 1).
        assert_eq!(bus.read(0x5234), 0xAB);
    }

    #[test]
    fn test_bus_rom_write_goes_to_mbc_register() {
        let mut bus = make_bus_with_mbc1_cart(4);
        // Write to ROM bank select register.
        bus.write(0x2000, 0x02);

        // The memory array for ROM space should NOT be updated.
        assert_eq!(bus.memory[0x2000], 0x00);
        // But the cartridge bank should have switched.
        assert_eq!(bus.read(0x4100), 0x00); // Bank 2, offset 0x100
    }

    #[test]
    fn test_bus_external_ram_round_trip() {
        let mut data = vec![0u8; 0x8000]; // 32KB = 2 banks
        data[0x0147] = 0x02; // MBC1+RAM
        data[0x0148] = 0; // 32KB
        data[0x0149] = 2; // 8KB RAM
        let mut bus = Bus::new();
        bus.load_cartridge(&data);

        // RAM is disabled by default.
        assert_eq!(bus.read(0xA000), 0xFF);

        // Enable RAM.
        bus.write(0x0000, 0x0A);
        bus.write(0xA000, 0x42);
        assert_eq!(bus.read(0xA000), 0x42);
    }

    #[test]
    fn test_bus_external_ram_returns_ff_without_cartridge() {
        let bus = Bus::new();
        assert_eq!(bus.read(0xA000), 0xFF);
        assert_eq!(bus.read(0xBFFF), 0xFF);
    }

    #[test]
    fn test_bus_rom_write_without_cartridge_still_writes_to_memory() {
        let mut bus = Bus::new();
        bus.write(0x1234, 0xAB);
        assert_eq!(bus.memory[0x1234], 0xAB);
        assert_eq!(bus.read(0x1234), 0xAB);
    }

    #[test]
    fn test_bus_dma_reads_through_cartridge() {
        let mut data = vec![0u8; 0x8000]; // 32KB = 2 banks
        data[0x0147] = 0x01; // MBC1
        data[0x0148] = 0; // 32KB
        data[0x0149] = 0;
        // Put a recognizable pattern in bank 1 starting at offset 0x4000.
        for i in 0..160usize {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            {
                data[0x4000 + i] = (i & 0xFF) as u8;
            }
        }
        let mut bus = Bus::new();
        bus.load_cartridge(&data);

        // DMA source 0x40: src = (0x40 << 8) | index = 0x4000 + index.
        bus.write(DMA_ADDR, 0x40);
        bus.tick_timers(160);

        for i in 0..160u16 {
            assert_eq!(
                bus.read(0xFE00 + i),
                (i & 0xFF) as u8,
                "DMA byte {i} mismatch"
            );
        }
    }
}
