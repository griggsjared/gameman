// Define CPU flag constants using bitwise representation
// These constants represent individual bits in a status register.
const FLAG_ZERO: u8 = 0b1000_0000; // Bit 7
const FLAG_SUBTRACT: u8 = 0b0100_0000; // Bit 6
const FLAG_HALF_CARRY: u8 = 0b0010_0000; // Bit 5
const FLAG_CARRY: u8 = 0b0001_0000; // Bit 4

/// Represents the CPU registers of a simple 8-bit architecture.
/// Contains both 8-bit general purpose registers and 16-bit special purpose registers.
/// - A: Accumulator - Used for arithmetic and logic operations
/// - F: Flags - Used to store the status flags resulting from operations
/// - B, C: General-purpose - Used together as a 16-bit register pair (BC) often used as a counter
/// - D, E: General-purpose - Used together as a 16-bit register pair (DE) often used as a pointer
/// - H, L: Used together as a 16-bit register pair (HL) often used for indirect addressing
/// - SP: Stack Pointer
/// - PC: Program Counter
#[derive(Debug)]
struct Registers {
    // 8-bit general purpose registers
    /// Accumulator - The primary register for arithmetic and logic operations
    a: u8,
    /// Flags - Stores the status flags resulting from operations
    f: u8,
    /// The high byte of the BC pair.
    b: u8,
    /// The low byte of the BC pair.
    c: u8,
    /// The high byte of the DE pair.
    d: u8,
    /// The low byte of the DE pair.
    e: u8,
    /// The high byte of the HL pair.
    h: u8,
    /// The low byte of the HL pair.
    l: u8,
    // 16-bit special purpose registers
    /// Stack Pointer
    sp: u16,
    /// Program Counter
    pc: u16,
}

impl Registers {
    /// new initializes all registers to zero
    pub fn new() -> Self {
        Registers {
            a: 0,
            f: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            sp: 0,
            pc: 0,
        }
    }

    pub fn af(&self) -> u16 {
        ((self.a as u16) << 8) | (self.f as u16)
    }

    /// set_af sets the AF register pair
    /// Writes the high byte to A and the low byte to F
    /// When setting the F register, the lower 4 bits are masked to 0
    pub fn set_af(&mut self, value: u16) {
        self.a = (value >> 8) as u8;
        self.f = (value as u8) & 0xF0;
    }

    pub fn bc(&self) -> u16 {
        ((self.b as u16) << 8) | (self.c as u16)
    }

    pub fn set_bc(&mut self, value: u16) {
        self.b = (value >> 8) as u8;
        self.c = value as u8;
    }

    pub fn de(&self) -> u16 {
        ((self.d as u16) << 8) | (self.e as u16)
    }

    pub fn set_de(&mut self, value: u16) {
        self.d = (value >> 8) as u8;
        self.e = value as u8;
    }

    pub fn hl(&self) -> u16 {
        ((self.h as u16) << 8) | (self.l as u16)
    }

    pub fn set_hl(&mut self, value: u16) {
        self.h = (value >> 8) as u8;
        self.l = value as u8;
    }

    pub fn zero(&self) -> bool {
        (self.f & FLAG_ZERO) != 0
    }

    pub fn set_zero(&mut self, value: bool) {
        if value {
            self.f |= FLAG_ZERO;
        } else {
            self.f &= !FLAG_ZERO;
        }
    }

    pub fn subtract(&self) -> bool {
        (self.f & FLAG_SUBTRACT) != 0
    }

    pub fn set_subtract(&mut self, value: bool) {
        if value {
            self.f |= FLAG_SUBTRACT;
        } else {
            self.f &= !FLAG_SUBTRACT;
        }
    }

    pub fn half_carry(&self) -> bool {
        (self.f & FLAG_HALF_CARRY) != 0
    }

    pub fn set_half_carry(&mut self, value: bool) {
        if value {
            self.f |= FLAG_HALF_CARRY;
        } else {
            self.f &= !FLAG_HALF_CARRY;
        }
    }

    pub fn carry(&self) -> bool {
        (self.f & FLAG_CARRY) != 0
    }

    pub fn set_carry(&mut self, value: bool) {
        if value {
            self.f |= FLAG_CARRY;
        } else {
            self.f &= !FLAG_CARRY;
        }
    }
}

fn main() {
    let mut registers = Registers::new();
    println!("CPU Registers initialized: {:?}", registers);
    println!("AF Register: {:04X}", registers.af());
    println!("BC Register: {:04X}", registers.bc());
    println!("DE Register: {:04X}", registers.de());
    println!("HL Register: {:04X}", registers.hl());
    println!("SP Register: {:04X}", registers.sp);
    println!("PC Register: {:04X}", registers.pc);

    registers.set_af(0x1234);
    registers.set_bc(0x5678);
    registers.set_de(0x9ABC);
    registers.set_hl(0xDEF0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registers_initialization() {
        let registers = Registers::new();
        assert_eq!(registers.a, 0);
        assert_eq!(registers.f, 0);
        assert_eq!(registers.b, 0);
        assert_eq!(registers.c, 0);
        assert_eq!(registers.d, 0);
        assert_eq!(registers.e, 0);
        assert_eq!(registers.h, 0);
        assert_eq!(registers.l, 0);
        assert_eq!(registers.sp, 0);
        assert_eq!(registers.pc, 0);
    }

    #[test]
    fn test_16bit_register_pair_af() {
        let mut registers = Registers::new();
        registers.a = 0x12;
        registers.f = 0x30;
        assert_eq!(registers.af(), 0x1230);
    }

    #[test]
    fn test_16bit_register_pair_bc() {
        let mut registers = Registers::new();
        registers.b = 0xAB;
        registers.c = 0xCD;
        assert_eq!(registers.bc(), 0xABCD);
    }

    #[test]
    fn test_16bit_register_pair_de() {
        let mut registers = Registers::new();
        registers.d = 0x11;
        registers.e = 0x22;
        assert_eq!(registers.de(), 0x1122);
    }

    #[test]
    fn test_16bit_register_pair_hl() {
        let mut registers = Registers::new();
        registers.h = 0xFF;
        registers.l = 0x00;
        assert_eq!(registers.hl(), 0xFF00);
    }

    #[test]
    fn test_set_16bit_register_pair_af() {
        let mut registers = Registers::new();
        registers.set_af(0x34F0); // Only upper 4 bits of F should be set
        assert_eq!(registers.a, 0x34);
        assert_eq!(registers.f, 0xF0); //Will always mask lower 4 bits to 0
    }

    #[test]
    fn test_set_af_masks_lower_4_bits() {
        let mut registers = Registers::new();
        registers.set_af(0x12FF); // Try to set all bits in F. The lower 4 should be masked.
        assert_eq!(registers.a, 0x12);
        assert_eq!(registers.f, 0xF0); // Lower 4 bits should be masked to 0
    }

    #[test]
    fn test_set_16bit_register_pair_bc() {
        let mut registers = Registers::new();
        registers.set_bc(0x1234);
        assert_eq!(registers.b, 0x12);
        assert_eq!(registers.c, 0x34);
        assert_eq!(registers.bc(), 0x1234);
    }

    #[test]
    fn test_set_16bit_register_pair_de() {
        let mut registers = Registers::new();
        registers.set_de(0x5678);
        assert_eq!(registers.d, 0x56);
        assert_eq!(registers.e, 0x78);
    }

    #[test]
    fn test_set_16bit_register_pair_hl() {
        let mut registers = Registers::new();
        registers.set_hl(0xABCD);
        assert_eq!(registers.h, 0xAB);
        assert_eq!(registers.l, 0xCD);
    }

    #[test]
    fn test_zero_flag_getter_setter() {
        let mut registers = Registers::new();
        assert_eq!(registers.zero(), false);
        registers.set_zero(true);
        assert_eq!(registers.zero(), true);
        assert_eq!(registers.f, 0b1000_0000);
        registers.set_zero(false);
        assert_eq!(registers.zero(), false);
    }

    #[test]
    fn test_subtract_flag_getter_and_setter() {
        let mut registers = Registers::new();
        assert_eq!(registers.subtract(), false);
        registers.set_subtract(true);
        assert_eq!(registers.subtract(), true);
        assert_eq!(registers.f, 0b0100_0000);
        registers.set_subtract(false);
        assert_eq!(registers.subtract(), false);
    }

    #[test]
    fn test_half_carry_flag_getter_and_setter() {
        let mut registers = Registers::new();
        assert_eq!(registers.half_carry(), false);
        registers.set_half_carry(true);
        assert_eq!(registers.half_carry(), true);
        assert_eq!(registers.f, 0b0010_0000);
        registers.set_half_carry(false);
        assert_eq!(registers.half_carry(), false);
    }

    #[test]
    fn test_carry_flag_getter_and_setter() {
        let mut registers = Registers::new();
        assert_eq!(registers.carry(), false);
        registers.set_carry(true);
        assert_eq!(registers.carry(), true);
        assert_eq!(registers.f, 0b0001_0000);
        registers.set_carry(false);
        assert_eq!(registers.carry(), false);
    }

    #[test]
    fn test_multiple_flag_indpendence() {
        // test that setting one flag does not affect others
        let mut registers = Registers::new();

        // set both zero and carry flags
        registers.set_zero(true);
        registers.set_carry(true);
        assert_eq!(registers.f, 0b1001_0000);

        // clear zero flag
        registers.set_zero(false);
        assert_eq!(registers.zero(), false);
        assert_eq!(registers.carry(), true);
        assert_eq!(registers.f, 0b0001_0000);

        // set subtract flag and make sure carry is still set
        registers.set_subtract(true);
        assert_eq!(registers.carry(), true);
        assert_eq!(registers.subtract(), true);
        assert_eq!(registers.f, 0b0101_0000);
    }

    #[test]
    fn test_all_flags_set_and_clear() {
        let mut registers = Registers::new();

        //setting all flags
        registers.set_zero(true);
        registers.set_subtract(true);
        registers.set_half_carry(true);
        registers.set_carry(true);

        assert_eq!(registers.f, 0b1111_0000);
        assert!(registers.zero());
        assert!(registers.subtract());
        assert!(registers.half_carry());
        assert!(registers.carry());

        //clearing all flags
        registers.set_zero(false);
        registers.set_subtract(false);
        registers.set_half_carry(false);
        registers.set_carry(false);

        assert_eq!(registers.f, 0b0000_0000);
        assert!(!registers.zero());
        assert!(!registers.subtract());
        assert!(!registers.half_carry());
        assert!(!registers.carry());
    }

    #[test]
    fn test_read_multiple_flags() {
        let mut registers = Registers::new();

        // Set zero and half-carry flags
        registers.set_zero(true);
        registers.set_half_carry(true);

        assert!(registers.zero());
        assert!(!registers.subtract());
        assert!(registers.half_carry());
        assert!(!registers.carry());
    }
}
