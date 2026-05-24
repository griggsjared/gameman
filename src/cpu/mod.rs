use crate::bus::Bus;

mod registers;
pub use registers::Registers;

#[derive(Debug, Default)]
pub struct Cpu {
    pub registers: Registers,
}

impl Cpu {
    #[must_use]
    pub fn new() -> Self {
        Cpu {
            registers: Registers::new(),
        }
    }

    /// Reset the CPU to initial state
    pub fn reset(&mut self) {
        self.registers = Registers::new();
        // Game Boy starts execution at 0x0100
        self.registers.pc = 0x0100;
        // Initial stack pointer
        self.registers.sp = 0xFFFE;
    }

    /// Execute one instruction and return the cycle count
    pub fn step(&mut self, bus: &mut Bus) -> u8 {
        let opcode = bus.read(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);
        self.execute(opcode, bus)
    }

    /// Decode and execute a single opcode.
    ///
    /// `opcode` has already been fetched and PC advanced by `step()`.
    /// This method reads any immediate operands, updates registers/flags,
    /// writes memory, and returns the number of cycles consumed.
    ///
    /// Register encoding (3-bit `DDD` / `SSS` fields):
    /// ```text
    /// 000 = B    001 = C    010 = D    011 = E
    /// 100 = H    101 = L    110 = (HL) 111 = A
    /// ```
    fn execute(&mut self, opcode: u8, bus: &mut Bus) -> u8 {
        match opcode {
            0x3E => {
                self.registers.a = self.imm8(bus);
                8
            } // LD A, n
            0x06 => {
                self.registers.b = self.imm8(bus);
                8
            } // LD B, n
            0x0E => {
                self.registers.c = self.imm8(bus);
                8
            } // LD C, n
            0x16 => {
                self.registers.d = self.imm8(bus);
                8
            } // LD D, n
            0x1E => {
                self.registers.e = self.imm8(bus);
                8
            } // LD E, n
            0x26 => {
                self.registers.h = self.imm8(bus);
                8
            } // LD H, n
            0x2E => {
                self.registers.l = self.imm8(bus);
                8
            } // LD L, n

            0x76 => panic!("Unimplemented opcode: 0x76"), // HALT

            0x40..=0x7F => {
                // LD r, r'
                let source = opcode & 0b0000_0111;
                let dest = (opcode & 0b0011_1000) >> 3;
                let value = self.read_register(source, bus);
                self.write_register(dest, value, bus);
                if source == 6 || dest == 6 { 8 } else { 4 }
            }

            0x04 | 0x0C | 0x14 | 0x1C | 0x24 | 0x2C | 0x34 | 0x3C => {
                // INC r
                let reg = (opcode & 0b0011_1000) >> 3;
                let value = self.read_register(reg, bus);
                let result = value.wrapping_add(1);
                self.write_register(reg, result, bus);
                self.registers.set_zero(result == 0);
                self.registers.set_subtract(false);
                self.registers.set_half_carry((value & 0x0F) == 0x0F);
                if reg == 6 { 12 } else { 4 }
            }

            0x05 | 0x0D | 0x15 | 0x1D | 0x25 | 0x2D | 0x35 | 0x3D => {
                // DEC r
                let reg = (opcode & 0b0011_1000) >> 3;
                let value = self.read_register(reg, bus);
                let result = value.wrapping_sub(1);
                self.write_register(reg, result, bus);
                self.registers.set_zero(result == 0);
                self.registers.set_subtract(true);
                #[allow(clippy::verbose_bit_mask)]
                self.registers.set_half_carry((value & 0x0F) == 0x00);
                if reg == 6 { 12 } else { 4 }
            }

            0x80..=0x87 => {
                // ADD A, r
                let source = opcode & 0b0000_0111;
                let value = self.read_register(source, bus);
                self.add_a(value);
                if source == 6 { 8 } else { 4 }
            }

            0xC6 => {
                // ADD A, n
                let value = self.imm8(bus);
                self.add_a(value);
                8
            }

            0x00 => 4, // NOP

            _ => panic!("Unimplemented opcode: 0x{opcode:02X}"),
        }
    }

    /// Read an immediate byte from `PC` and advance `PC` by 1.
    fn imm8(&mut self, bus: &Bus) -> u8 {
        let value = bus.read(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);
        value
    }

    fn add_a(&mut self, value: u8) {
        let a = self.registers.a;
        let (result, carry) = a.overflowing_add(value);
        self.registers.a = result;
        self.registers.set_zero(result == 0);
        self.registers.set_subtract(false);
        self.registers
            .set_half_carry((a & 0x0F) + (value & 0x0F) > 0x0F);
        self.registers.set_carry(carry);
    }

    fn read_register(&self, code: u8, bus: &Bus) -> u8 {
        match code {
            0 => self.registers.b,
            1 => self.registers.c,
            2 => self.registers.d,
            3 => self.registers.e,
            4 => self.registers.h,
            5 => self.registers.l,
            6 => bus.read(self.registers.hl()),
            7 => self.registers.a,
            _ => unreachable!(),
        }
    }

    fn write_register(&mut self, code: u8, value: u8, bus: &mut Bus) {
        match code {
            0 => self.registers.b = value,
            1 => self.registers.c = value,
            2 => self.registers.d = value,
            3 => self.registers.e = value,
            4 => self.registers.h = value,
            5 => self.registers.l = value,
            6 => bus.write(self.registers.hl(), value),
            7 => self.registers.a = value,
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::Bus;

    #[test]
    fn test_cpu_new() {
        let cpu = Cpu::new();
        assert_eq!(cpu.registers.pc, 0);
        assert_eq!(cpu.registers.sp, 0);
    }

    #[test]
    fn test_cpu_reset() {
        let mut cpu = Cpu::new();
        cpu.registers.a = 0xFF;
        cpu.registers.pc = 0x5555;

        cpu.reset();

        assert_eq!(cpu.registers.a, 0);
        assert_eq!(cpu.registers.pc, 0x0100);
        assert_eq!(cpu.registers.sp, 0xFFFE);
    }

    #[test]
    fn test_nop() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        bus.write(0x0000, 0x00); // NOP

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 4);
        assert_eq!(cpu.registers.pc, 0x0001);
    }

    #[test]
    fn test_ld_r_n() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        bus.write(0x0000, 0x3E); // LD A, n
        bus.write(0x0001, 0x42);

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 8);
        assert_eq!(cpu.registers.a, 0x42);
        assert_eq!(cpu.registers.pc, 0x0002);
    }

    #[test]
    fn test_ld_r_n_boundary() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        bus.write(0x0000, 0x06); // LD B, n
        bus.write(0x0001, 0xFF);

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.b, 0xFF);
    }

    #[test]
    fn test_ld_r_r() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.b = 0x37;
        bus.write(0x0000, 0x78); // LD A, B

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 4);
        assert_eq!(cpu.registers.a, 0x37);
        assert_eq!(cpu.registers.pc, 0x0001);
    }

    #[test]
    fn test_ld_r_r_all_registers() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();

        // LD A, B
        cpu.registers.b = 0x01;
        bus.write(0x0000, 0x78);
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.a, 0x01);

        // LD A, C
        cpu.registers.c = 0x02;
        bus.write(0x0001, 0x79);
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.a, 0x02);

        // LD A, D
        cpu.registers.d = 0x03;
        bus.write(0x0002, 0x7A);
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.a, 0x03);

        // LD A, E
        cpu.registers.e = 0x04;
        bus.write(0x0003, 0x7B);
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.a, 0x04);

        // LD A, H
        cpu.registers.h = 0x05;
        bus.write(0x0004, 0x7C);
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.a, 0x05);

        // LD A, L
        cpu.registers.l = 0x06;
        bus.write(0x0005, 0x7D);
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.a, 0x06);
    }

    #[test]
    fn test_ld_r_r_hl_indirect() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_hl(0xC000);
        bus.write(0xC000, 0xAB);
        bus.write(0x0000, 0x7E); // LD A, (HL)

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 8);
        assert_eq!(cpu.registers.a, 0xAB);
    }

    #[test]
    fn test_ld_hl_indirect_r() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0xCD;
        cpu.registers.set_hl(0xC000);
        bus.write(0x0000, 0x77); // LD (HL), A

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 8);
        assert_eq!(bus.read(0xC000), 0xCD);
    }

    #[test]
    fn test_inc_r() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.b = 0x01;
        bus.write(0x0000, 0x04); // INC B

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 4);
        assert_eq!(cpu.registers.b, 0x02);
        assert!(!cpu.registers.zero());
        assert!(!cpu.registers.subtract());
        assert!(!cpu.registers.half_carry());
    }

    #[test]
    fn test_inc_r_zero() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.b = 0xFF;
        bus.write(0x0000, 0x04); // INC B

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.b, 0x00);
        assert!(cpu.registers.zero());
        assert!(!cpu.registers.subtract());
        assert!(cpu.registers.half_carry());
    }

    #[test]
    fn test_inc_r_half_carry() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.b = 0x0F;
        bus.write(0x0000, 0x04); // INC B

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.b, 0x10);
        assert!(!cpu.registers.zero());
        assert!(!cpu.registers.subtract());
        assert!(cpu.registers.half_carry());
    }

    #[test]
    fn test_dec_r() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.b = 0x02;
        bus.write(0x0000, 0x05); // DEC B

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 4);
        assert_eq!(cpu.registers.b, 0x01);
        assert!(!cpu.registers.zero());
        assert!(cpu.registers.subtract());
        assert!(!cpu.registers.half_carry());
    }

    #[test]
    fn test_dec_r_zero() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.b = 0x01;
        bus.write(0x0000, 0x05); // DEC B

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.b, 0x00);
        assert!(cpu.registers.zero());
        assert!(cpu.registers.subtract());
        assert!(!cpu.registers.half_carry());
    }

    #[test]
    fn test_dec_r_half_carry() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.b = 0x10;
        bus.write(0x0000, 0x05); // DEC B

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.b, 0x0F);
        assert!(!cpu.registers.zero());
        assert!(cpu.registers.subtract());
        assert!(cpu.registers.half_carry());
    }

    #[test]
    fn test_dec_r_underflow() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.b = 0x00;
        bus.write(0x0000, 0x05); // DEC B

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.b, 0xFF);
        assert!(!cpu.registers.zero());
        assert!(cpu.registers.subtract());
        assert!(cpu.registers.half_carry());
    }

    #[test]
    fn test_add_a_r() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0x01;
        cpu.registers.b = 0x02;
        bus.write(0x0000, 0x80); // ADD A, B

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 4);
        assert_eq!(cpu.registers.a, 0x03);
        assert!(!cpu.registers.zero());
        assert!(!cpu.registers.subtract());
        assert!(!cpu.registers.half_carry());
        assert!(!cpu.registers.carry());
    }

    #[test]
    fn test_add_a_r_zero() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0x00;
        cpu.registers.b = 0x00;
        bus.write(0x0000, 0x80); // ADD A, B

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.a, 0x00);
        assert!(cpu.registers.zero());
    }

    #[test]
    fn test_add_a_r_half_carry() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0x0F;
        cpu.registers.b = 0x01;
        bus.write(0x0000, 0x80); // ADD A, B

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.a, 0x10);
        assert!(cpu.registers.half_carry());
        assert!(!cpu.registers.carry());
    }

    #[test]
    fn test_add_a_r_carry() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0xFF;
        cpu.registers.b = 0x01;
        bus.write(0x0000, 0x80); // ADD A, B

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.a, 0x00);
        assert!(cpu.registers.zero());
        assert!(!cpu.registers.subtract());
        assert!(cpu.registers.half_carry());
        assert!(cpu.registers.carry());
    }

    #[test]
    fn test_add_a_n() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0x12;
        bus.write(0x0000, 0xC6); // ADD A, n
        bus.write(0x0001, 0x34);

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 8);
        assert_eq!(cpu.registers.a, 0x46);
        assert_eq!(cpu.registers.pc, 0x0002);
    }

    #[test]
    fn test_add_a_hl_indirect() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0x20;
        cpu.registers.set_hl(0xC000);
        bus.write(0xC000, 0x20);
        bus.write(0x0000, 0x86); // ADD A, (HL)

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 8);
        assert_eq!(cpu.registers.a, 0x40);
    }

    #[test]
    fn test_inc_hl_indirect() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_hl(0xC000);
        bus.write(0xC000, 0x0F);
        bus.write(0x0000, 0x34); // INC (HL)

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 12);
        assert_eq!(bus.read(0xC000), 0x10);
        assert!(cpu.registers.half_carry());
    }

    #[test]
    fn test_dec_hl_indirect() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_hl(0xC000);
        bus.write(0xC000, 0x10);
        bus.write(0x0000, 0x35); // DEC (HL)

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 12);
        assert_eq!(bus.read(0xC000), 0x0F);
        assert!(cpu.registers.half_carry());
    }

    #[test]
    fn test_sequence_multiple_instructions() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        bus.write(0x0000, 0x06); // LD B, 0x10
        bus.write(0x0001, 0x10);
        bus.write(0x0002, 0x04); // INC B
        bus.write(0x0003, 0x80); // ADD A, B

        cpu.step(&mut bus); // LD B, 0x10
        assert_eq!(cpu.registers.b, 0x10);

        cpu.step(&mut bus); // INC B
        assert_eq!(cpu.registers.b, 0x11);

        cpu.step(&mut bus); // ADD A, B
        assert_eq!(cpu.registers.a, 0x11);
    }
}
