//! Game Boy cartridge with MBC1 mapper support.
//!
//! Handles ROM header parsing, bank switching for ROM and external RAM,
//! and the MBC1 control register interface.

/// ROM header offsets.
const HEADER_TYPE_OFFSET: usize = 0x0147;
const HEADER_ROM_SIZE_OFFSET: usize = 0x0148;
const HEADER_RAM_SIZE_OFFSET: usize = 0x0149;

/// MBC1 control register address ranges.
const RAM_ENABLE_START: u16 = 0x0000;
const RAM_ENABLE_END: u16 = 0x1FFF;
const ROM_BANK_LOW_START: u16 = 0x2000;
const ROM_BANK_LOW_END: u16 = 0x3FFF;
const BANK_UPPER_START: u16 = 0x4000;
const BANK_UPPER_END: u16 = 0x5FFF;
const MODE_SELECT_START: u16 = 0x6000;
const MODE_SELECT_END: u16 = 0x7FFF;

/// ROM and RAM bank sizes.
const ROM_BANK_SIZE: usize = 0x4000; // 16KB
const RAM_BANK_SIZE: usize = 0x2000; // 8KB

/// Cartridge type byte values.
const CART_TYPE_ROM_ONLY: u8 = 0x00;
const CART_TYPE_MBC1: u8 = 0x01;
const CART_TYPE_MBC1_RAM: u8 = 0x02;
const CART_TYPE_MBC1_RAM_BATTERY: u8 = 0x03;

/// MBC1 bank register mask (5-bit).
const BANK_LOW_MASK: u8 = 0x1F;
/// Upper 2-bit register mask.
const BANK_UPPER_MASK: u8 = 0x03;
/// RAM enable signature.
const RAM_ENABLE_SIGNATURE: u8 = 0x0A;

/// External RAM address window.
const EXTERNAL_RAM_START: u16 = 0xA000;

#[derive(Debug)]
pub struct Cartridge {
    rom: Vec<u8>,
    ram: Vec<u8>,
    rom_bank_count: usize,
    rom_bank: u8,
    bank_upper: u8,
    mode: u8,
    ram_enabled: bool,
    // Retained for future use: battery detection, save states, and logging.
    #[allow(dead_code)]
    cart_type: u8,
}

impl Cartridge {
    /// Load a cartridge from raw ROM bytes (e.g. from a `.gb` file).
    ///
    /// Parses the header to determine mapper type, ROM size, and RAM size.
    /// Does not validate the header checksum (bytes 0x0134..=0x014C → 0x014D).
    ///
    /// # Panics
    ///
    /// Panics if the ROM is too small to contain a valid header, if the
    /// cartridge type is unsupported, or if the ROM data is smaller than
    /// the declared size.
    #[must_use]
    pub fn from_bytes(data: &[u8]) -> Self {
        assert!(
            data.len() > HEADER_RAM_SIZE_OFFSET,
            "ROM too small to contain header"
        );

        let cart_type = data[HEADER_TYPE_OFFSET];
        assert!(
            matches!(
                cart_type,
                CART_TYPE_ROM_ONLY
                    | CART_TYPE_MBC1
                    | CART_TYPE_MBC1_RAM
                    | CART_TYPE_MBC1_RAM_BATTERY
            ),
            "Unsupported cartridge type: 0x{cart_type:02X}"
        );

        let rom_size = Self::parse_rom_size(data[HEADER_ROM_SIZE_OFFSET]);
        assert!(
            data.len() >= rom_size,
            "ROM data ({} bytes) smaller than declared size ({rom_size} bytes)",
            data.len()
        );

        let ext_ram_size = Self::parse_ram_size(data[HEADER_RAM_SIZE_OFFSET]);

        let mut rom = vec![0u8; rom_size];
        rom[..data.len().min(rom_size)].copy_from_slice(&data[..data.len().min(rom_size)]);
        let rom_bank_count = rom_size / ROM_BANK_SIZE;

        Cartridge {
            rom,
            ram: vec![0u8; ext_ram_size],
            rom_bank_count,
            rom_bank: 1,
            bank_upper: 0,
            mode: 0,
            ram_enabled: false,
            cart_type,
        }
    }

    /// Read a byte from the ROM address space (`0x0000..=0x7FFF`).
    ///
    /// Bank 0 (`0x0000..=0x3FFF`) is always mapped to ROM bank 0.
    /// The switchable window (`0x4000..=0x7FFF`) maps to the bank
    /// selected by the `rom_bank` and `bank_upper` registers.
    /// Bank numbers wrap around to fit the actual ROM size.
    #[must_use]
    pub fn read_rom(&self, address: u16) -> u8 {
        let addr = usize::from(address);
        if address <= 0x3FFF {
            // Bank 0: always fixed.
            self.rom[addr]
        } else {
            // Switchable bank window.
            let bank = self.current_rom_bank();
            let offset = (bank % self.rom_bank_count) * ROM_BANK_SIZE + (addr - 0x4000);
            self.rom[offset]
        }
    }

    /// Handle a write to the ROM address space (`0x0000..=0x7FFF`).
    ///
    /// These writes configure MBC1 control registers and never reach ROM.
    pub fn write_register(&mut self, address: u16, value: u8) {
        match address {
            RAM_ENABLE_START..=RAM_ENABLE_END => {
                self.ram_enabled = value == RAM_ENABLE_SIGNATURE;
            }
            ROM_BANK_LOW_START..=ROM_BANK_LOW_END => {
                let bank = value & BANK_LOW_MASK;
                // MBC1 quirk: writing 0 maps to bank 1 (bank 0 is only
                // accessible in the fixed 0x0000..=0x3FFF window).
                self.rom_bank = if bank == 0 { 1 } else { bank };
            }
            BANK_UPPER_START..=BANK_UPPER_END => {
                self.bank_upper = value & BANK_UPPER_MASK;
            }
            MODE_SELECT_START..=MODE_SELECT_END => {
                self.mode = value & 1;
            }
            _ => {}
        }
    }

    /// Read a byte from external RAM (`0xA000..=0xBFFF`).
    ///
    /// Returns `0xFF` if RAM is disabled, unaddressable, or absent.
    #[must_use]
    pub fn read_ram(&self, address: u16) -> u8 {
        if !self.ram_enabled || self.ram.is_empty() {
            return 0xFF;
        }
        let bank = self.current_ram_bank();
        let offset = bank * RAM_BANK_SIZE + usize::from(address - EXTERNAL_RAM_START);
        if offset < self.ram.len() {
            self.ram[offset]
        } else {
            0xFF
        }
    }

    /// Write a byte to external RAM (`0xA000..=0xBFFF`).
    ///
    /// Writes are silently dropped if RAM is disabled or absent.
    pub fn write_ram(&mut self, address: u16, value: u8) {
        if !self.ram_enabled || self.ram.is_empty() {
            return;
        }
        let bank = self.current_ram_bank();
        let offset = bank * RAM_BANK_SIZE + usize::from(address - EXTERNAL_RAM_START);
        if offset < self.ram.len() {
            self.ram[offset] = value;
        }
    }

    /// Whether external RAM is currently enabled.
    #[must_use]
    pub fn ram_enabled(&self) -> bool {
        self.ram_enabled
    }

    /// Whether this cartridge has a non-empty external RAM region.
    #[must_use]
    pub fn has_ram(&self) -> bool {
        !self.ram.is_empty()
    }

    /// Compute the ROM bank index for the switchable window (`0x4000..=0x7FFF`).
    ///
    /// In MBC1 mode 0 (ROM banking), `bank_upper` provides the upper 2 bits
    /// of the ROM bank number. In mode 1 (RAM banking), `bank_upper` selects
    /// the RAM bank instead and does not affect ROM.
    fn current_rom_bank(&self) -> usize {
        let upper = if self.mode == 0 { self.bank_upper } else { 0 };
        let bank = (u16::from(upper) << 5) | u16::from(self.rom_bank);
        usize::from(bank)
    }

    /// Compute the RAM bank index (`0..=3`).
    ///
    /// In MBC1 mode 1 (RAM banking), `bank_upper` selects the RAM bank.
    /// In mode 0, only bank 0 is addressable.
    fn current_ram_bank(&self) -> usize {
        if self.mode == 1 {
            usize::from(self.bank_upper)
        } else {
            0
        }
    }

    /// Derive ROM size in bytes from the header byte.
    ///
    /// Header value `N` means `32KB * 2^N` bytes (minimum 32KB for bank 0 + bank 1).
    fn parse_rom_size(header_byte: u8) -> usize {
        ROM_BANK_SIZE * 2 * 2_usize.pow(u32::from(header_byte))
    }

    /// Derive external RAM size in bytes from the header byte.
    ///
    /// On real MBC1 hardware the upper register is only 2 bits, so only
    /// 4 RAM banks (32KB) are addressable regardless of header value.
    fn parse_ram_size(header_byte: u8) -> usize {
        match header_byte {
            1 => 2048,                  // 2KB
            2 | 5 => 8 * RAM_BANK_SIZE, // 8KB (one bank) / 64KB (excess unaddressable)
            3 => 4 * RAM_BANK_SIZE,     // 32KB (four banks)
            4 => 16 * RAM_BANK_SIZE,    // 128KB (sixteen banks)
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cart_with_rom(rom_size: usize, cart_type: u8) -> Cartridge {
        let mut data = vec![0u8; rom_size];
        // Set cartridge type in header.
        data[HEADER_TYPE_OFFSET] = cart_type;
        // ROM size header byte: N where total size = 32KB * 2^N.
        // Total banks = rom_size / (ROM_BANK_SIZE * 2), header = log2(banks).
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        {
            let n = (rom_size / (ROM_BANK_SIZE * 2)).trailing_zeros() as u8;
            data[HEADER_ROM_SIZE_OFFSET] = n;
        }
        // RAM size: 0 = no RAM.
        data[HEADER_RAM_SIZE_OFFSET] = 0;
        Cartridge::from_bytes(&data)
    }

    fn make_cart_with_ram(cart_type: u8, ram_size_header: u8) -> Cartridge {
        let rom_size = ROM_BANK_SIZE * 2; // 32KB = 2 banks
        let mut data = vec![0u8; rom_size];
        data[HEADER_TYPE_OFFSET] = cart_type;
        data[HEADER_ROM_SIZE_OFFSET] = 0; // 32KB
        data[HEADER_RAM_SIZE_OFFSET] = ram_size_header;
        Cartridge::from_bytes(&data)
    }

    #[test]
    fn test_rom_only_cart_parses_correctly() {
        let cart = make_cart_with_rom(ROM_BANK_SIZE * 2, CART_TYPE_ROM_ONLY);
        assert!(!cart.ram_enabled);
        assert!(!cart.has_ram());
    }

    #[test]
    fn test_mbc1_cart_parses_correctly() {
        let cart = make_cart_with_rom(ROM_BANK_SIZE * 4, CART_TYPE_MBC1);
        assert!(!cart.ram_enabled);
        assert!(!cart.has_ram());
    }

    #[test]
    fn test_mbc1_ram_cart_has_ram() {
        let cart = make_cart_with_ram(CART_TYPE_MBC1_RAM, 2); // 8KB
        assert!(cart.has_ram());
        assert!(!cart.ram_enabled);
    }

    #[test]
    fn test_read_rom_bank0_returns_first_16kb() {
        let mut data = vec![0u8; ROM_BANK_SIZE * 2]; // 32KB = 2 banks
        // Stamp a marker in bank 0.
        data[0x1234] = 0xAB;
        data[HEADER_TYPE_OFFSET] = CART_TYPE_MBC1;
        data[HEADER_ROM_SIZE_OFFSET] = 0; // 32KB
        data[HEADER_RAM_SIZE_OFFSET] = 0;
        let cart = Cartridge::from_bytes(&data);

        assert_eq!(cart.read_rom(0x1234), 0xAB);
        assert_eq!(cart.read_rom(0x0000), 0x00);
    }

    #[test]
    fn test_read_rom_switchable_bank_defaults_to_bank1() {
        let mut data = vec![0u8; ROM_BANK_SIZE * 4]; // 64KB = 4 banks
        // Stamp markers in bank 1 and bank 2.
        data[ROM_BANK_SIZE + 0x100] = 0xB1;
        data[ROM_BANK_SIZE * 2 + 0x100] = 0xB2;
        data[HEADER_TYPE_OFFSET] = CART_TYPE_MBC1;
        data[HEADER_ROM_SIZE_OFFSET] = 1; // 64KB
        data[HEADER_RAM_SIZE_OFFSET] = 0;
        let cart = Cartridge::from_bytes(&data);

        // Default rom_bank is 1, so 0x4100 reads from bank 1.
        assert_eq!(cart.read_rom(0x4100), 0xB1);
    }

    #[test]
    fn test_write_rom_bank_select_switches_bank() {
        let mut data = vec![0u8; ROM_BANK_SIZE * 4]; // 64KB = 4 banks
        data[ROM_BANK_SIZE + 0x100] = 0xB1;
        data[ROM_BANK_SIZE * 2 + 0x100] = 0xB2;
        data[ROM_BANK_SIZE * 3 + 0x100] = 0xB3;
        data[HEADER_TYPE_OFFSET] = CART_TYPE_MBC1;
        data[HEADER_ROM_SIZE_OFFSET] = 1; // 64KB
        data[HEADER_RAM_SIZE_OFFSET] = 0;
        let mut cart = Cartridge::from_bytes(&data);

        // Select bank 2.
        cart.write_register(0x2000, 2);
        assert_eq!(cart.read_rom(0x4100), 0xB2);

        // Select bank 3.
        cart.write_register(0x2000, 3);
        assert_eq!(cart.read_rom(0x4100), 0xB3);
    }

    #[test]
    fn test_write_zero_rom_bank_selects_bank1() {
        let mut data = vec![0u8; ROM_BANK_SIZE * 2]; // 32KB = 2 banks
        data[ROM_BANK_SIZE + 0x100] = 0xB1;
        data[HEADER_TYPE_OFFSET] = CART_TYPE_MBC1;
        data[HEADER_ROM_SIZE_OFFSET] = 0; // 32KB
        data[HEADER_RAM_SIZE_OFFSET] = 0;
        let mut cart = Cartridge::from_bytes(&data);

        // Writing 0 should map to bank 1 (MBC1 quirk).
        cart.write_register(0x2000, 0);
        assert_eq!(cart.read_rom(0x4100), 0xB1);
    }

    #[test]
    fn test_upper_bits_extend_rom_bank() {
        // 8 banks = 128KB, header byte = 2.
        let mut data = vec![0u8; ROM_BANK_SIZE * 8];
        data[ROM_BANK_SIZE * 6 + 0x100] = 0xB6; // Bank 6 marker
        data[HEADER_TYPE_OFFSET] = CART_TYPE_MBC1;
        data[HEADER_ROM_SIZE_OFFSET] = 2; // 128KB = 8 banks
        data[HEADER_RAM_SIZE_OFFSET] = 0;
        let mut cart = Cartridge::from_bytes(&data);

        // bank = (0 << 5) | 6 = 6
        cart.write_register(0x4000, 0x00);
        cart.write_register(0x2000, 6);
        assert_eq!(cart.read_rom(0x4100), 0xB6);

        // bank = (1 << 5) | 6 = 38 -> wraps: 38 % 8 = 6, so reads bank 6 again.
        cart.write_register(0x4000, 0x01);
        assert_eq!(cart.read_rom(0x4100), 0xB6);
    }

    #[test]
    fn test_ram_enable_and_disable() {
        let mut cart = make_cart_with_ram(CART_TYPE_MBC1_RAM, 2); // 8KB

        // RAM is disabled by default.
        assert_eq!(cart.read_ram(0xA000), 0xFF);

        // Enable RAM.
        cart.write_register(0x0000, RAM_ENABLE_SIGNATURE);
        assert!(cart.ram_enabled());

        // Write and read back.
        cart.write_ram(0xA000, 0x42);
        assert_eq!(cart.read_ram(0xA000), 0x42);

        // Disable RAM.
        cart.write_register(0x0000, 0x00);
        assert!(!cart.ram_enabled());
        assert_eq!(cart.read_ram(0xA000), 0xFF);
    }

    #[test]
    fn test_ram_write_ignored_when_disabled() {
        let mut cart = make_cart_with_ram(CART_TYPE_MBC1_RAM, 2);

        // Write while disabled — should be ignored.
        cart.write_ram(0xA000, 0x42);

        // Enable and verify nothing was written.
        cart.write_register(0x0000, RAM_ENABLE_SIGNATURE);
        assert_eq!(cart.read_ram(0xA000), 0x00);
    }

    #[test]
    fn test_mode1_selects_ram_bank() {
        let mut cart = make_cart_with_ram(CART_TYPE_MBC1_RAM, 2); // 8KB = 1 bank
        cart.write_register(0x0000, RAM_ENABLE_SIGNATURE);

        // Mode 0: RAM bank is always 0.
        cart.write_register(0x6000, 0x00);
        cart.write_ram(0xA000, 0x11);
        assert_eq!(cart.read_ram(0xA000), 0x11);

        // Mode 1: upper bits select RAM bank (bank 0 in this case).
        cart.write_register(0x6000, 0x01);
        cart.write_register(0x4000, 0x00);
        // Still bank 0, data should be accessible.
        assert_eq!(cart.read_ram(0xA000), 0x11);
    }

    #[test]
    fn test_mode1_does_not_affect_rom_bank() {
        // Need 64 banks (1MB) so that (1 << 5) | bank doesn't wrap to same bank.
        let mut data = vec![0u8; ROM_BANK_SIZE * 64];
        data[(ROM_BANK_SIZE * 3) + 0x100] = 0xB3; // Bank 3 marker
        data[(ROM_BANK_SIZE * 35) + 0x100] = 0x23; // Bank 35 marker
        data[HEADER_TYPE_OFFSET] = CART_TYPE_MBC1;
        data[HEADER_ROM_SIZE_OFFSET] = 5; // 1MB = 64 banks
        data[HEADER_RAM_SIZE_OFFSET] = 0;
        let mut cart = Cartridge::from_bytes(&data);

        // Set upper bits to 1 and rom_bank to 3.
        cart.write_register(0x4000, 0x01);
        cart.write_register(0x2000, 0x03);

        // Mode 0: upper bits contribute to ROM bank.
        cart.write_register(0x6000, 0x00);
        // bank = (1 << 5) | 3 = 35
        assert_eq!(cart.read_rom(0x4100), 0x23);

        // Mode 1: upper bits do NOT affect ROM bank.
        cart.write_register(0x6000, 0x01);
        // bank = (0 << 5) | 3 = 3
        assert_eq!(cart.read_rom(0x4100), 0xB3);
    }

    #[test]
    fn test_read_ram_disabled_returns_ff() {
        let cart = make_cart_with_ram(CART_TYPE_MBC1_RAM, 2);
        assert_eq!(cart.read_ram(0xA000), 0xFF);
        assert_eq!(cart.read_ram(0xBFFF), 0xFF);
    }

    #[test]
    fn test_external_ram_out_of_bounds_returns_ff() {
        let mut cart = make_cart_with_ram(CART_TYPE_MBC1_RAM, 1); // 2KB
        cart.write_register(0x0000, RAM_ENABLE_SIGNATURE);

        // 2KB = 0x800 bytes, so offset 0xA800 is beyond 2KB.
        // Address 0xA800 maps to offset 0x800 which is past end.
        assert_eq!(cart.read_ram(0xA800), 0xFF);
    }

    #[test]
    #[should_panic(expected = "Unsupported cartridge type")]
    fn test_unsupported_cartridge_type_panics() {
        let mut data = vec![0u8; ROM_BANK_SIZE * 2];
        data[HEADER_TYPE_OFFSET] = 0x05; // Unsupported
        let _ = Cartridge::from_bytes(&data);
    }

    #[test]
    fn test_rom_size_header_parsing() {
        assert_eq!(Cartridge::parse_rom_size(0), ROM_BANK_SIZE * 2); // 32KB
        assert_eq!(Cartridge::parse_rom_size(1), ROM_BANK_SIZE * 4); // 64KB
        assert_eq!(Cartridge::parse_rom_size(2), ROM_BANK_SIZE * 8); // 128KB
        assert_eq!(Cartridge::parse_rom_size(4), ROM_BANK_SIZE * 32); // 512KB
    }

    #[test]
    fn test_ram_size_header_parsing() {
        assert_eq!(Cartridge::parse_ram_size(0), 0);
        assert_eq!(Cartridge::parse_ram_size(1), 2048);
        assert_eq!(Cartridge::parse_ram_size(2), 8 * RAM_BANK_SIZE);
        assert_eq!(Cartridge::parse_ram_size(3), 4 * RAM_BANK_SIZE);
    }

    #[test]
    fn test_bank0_window_always_reads_from_offset_zero() {
        let mut data = vec![0u8; ROM_BANK_SIZE * 4]; // 64KB = 4 banks
        data[0x100] = 0xAA; // Bank 0 marker
        data[ROM_BANK_SIZE + 0x100] = 0xBB; // Bank 1 marker
        data[HEADER_TYPE_OFFSET] = CART_TYPE_MBC1;
        data[HEADER_ROM_SIZE_OFFSET] = 1; // 64KB
        data[HEADER_RAM_SIZE_OFFSET] = 0;
        let mut cart = Cartridge::from_bytes(&data);

        // Even after switching to bank 2, bank 0 window stays fixed.
        cart.write_register(0x2000, 2);
        assert_eq!(cart.read_rom(0x0100), 0xAA);
    }

    #[test]
    fn test_ram_round_trip_full_address_range() {
        let mut cart = make_cart_with_ram(CART_TYPE_MBC1_RAM, 2); // 8KB
        cart.write_register(0x0000, RAM_ENABLE_SIGNATURE);

        cart.write_ram(0xA000, 0x11);
        cart.write_ram(0xBFFF, 0x22);
        assert_eq!(cart.read_ram(0xA000), 0x11);
        assert_eq!(cart.read_ram(0xBFFF), 0x22);
    }
}
