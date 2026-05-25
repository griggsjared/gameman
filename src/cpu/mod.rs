use crate::bus::Bus;

mod registers;
pub use registers::Registers;

#[derive(Debug, Default)]
pub struct Cpu {
    pub registers: Registers,
}

/// 3-bit register encoding used in opcode fields.
#[derive(Clone, Copy)]
enum Reg {
    B = 0,
    C,
    D,
    E,
    H,
    L,
    IndirectHl,
    A,
}

impl Reg {
    #[must_use]
    const fn from_u3(code: u8) -> Self {
        match code & 0b111 {
            0 => Self::B,
            1 => Self::C,
            2 => Self::D,
            3 => Self::E,
            4 => Self::H,
            5 => Self::L,
            6 => Self::IndirectHl,
            7 => Self::A,
            _ => unreachable!(),
        }
    }
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
    #[allow(clippy::too_many_lines)]
    fn execute(&mut self, opcode: u8, bus: &mut Bus) -> u8 {
        match opcode {
            0x00 | 0xF3 | 0xFB => 4, // NOP, DI, EI

            0x03 | 0x13 | 0x23 | 0x33 => {
                // INC rr
                let pair = (opcode >> 4) & 0b11;
                self.inc_rr(pair);
                8
            }

            0x0B | 0x1B | 0x2B | 0x3B => {
                // DEC rr
                let pair = (opcode >> 4) & 0b11;
                self.dec_rr(pair);
                8
            }

            0x09 | 0x19 | 0x29 | 0x39 => {
                // ADD HL, rr
                let pair = (opcode >> 4) & 0b11;
                self.add_hl_rr(pair);
                8
            }

            0x01 | 0x11 | 0x21 | 0x31 => {
                // LD rr, nn
                let pair = (opcode >> 4) & 0b11;
                let value = self.imm16(bus);
                self.write_rr(pair, value);
                12
            }

            0x06 | 0x0E | 0x16 | 0x1E | 0x26 | 0x2E | 0x3E => {
                // LD r, n
                let reg = Reg::from_u3(opcode >> 3);
                let value = self.imm8(bus);
                self.write_register(reg, value, bus);
                8
            }

            // LD A, (rr) and LD (rr), A
            0x0A => {
                self.registers.a = bus.read(self.registers.bc());
                8
            } // LD A, (BC)
            0x1A => {
                self.registers.a = bus.read(self.registers.de());
                8
            } // LD A, (DE)
            0x02 => {
                bus.write(self.registers.bc(), self.registers.a);
                8
            } // LD (BC), A
            0x12 => {
                bus.write(self.registers.de(), self.registers.a);
                8
            } // LD (DE), A
            0x22 => {
                // LD (HL+), A
                bus.write(self.registers.hl(), self.registers.a);
                self.registers.set_hl(self.registers.hl().wrapping_add(1));
                8
            }
            0x32 => {
                // LD (HL-), A
                bus.write(self.registers.hl(), self.registers.a);
                self.registers.set_hl(self.registers.hl().wrapping_sub(1));
                8
            }
            0x2A => {
                // LD A, (HL+)
                self.registers.a = bus.read(self.registers.hl());
                self.registers.set_hl(self.registers.hl().wrapping_add(1));
                8
            }
            0x3A => {
                // LD A, (HL-)
                self.registers.a = bus.read(self.registers.hl());
                self.registers.set_hl(self.registers.hl().wrapping_sub(1));
                8
            }

            0x36 => {
                // LD (HL), n
                let value = self.imm8(bus);
                bus.write(self.registers.hl(), value);
                12
            }

            0x07 => {
                // RLCA
                let a = self.registers.a;
                let carry = a >> 7;
                self.registers.a = (a << 1) | carry;
                self.registers.set_zero(false);
                self.registers.set_subtract(false);
                self.registers.set_half_carry(false);
                self.registers.set_carry(carry != 0);
                4
            }

            0x0F => {
                // RRCA
                let a = self.registers.a;
                let carry = a & 1;
                self.registers.a = (a >> 1) | (carry << 7);
                self.registers.set_zero(false);
                self.registers.set_subtract(false);
                self.registers.set_half_carry(false);
                self.registers.set_carry(carry != 0);
                4
            }

            0x17 => {
                // RLA
                let a = self.registers.a;
                let carry = u8::from(self.registers.carry());
                let new_carry = a >> 7;
                self.registers.a = (a << 1) | carry;
                self.registers.set_zero(false);
                self.registers.set_subtract(false);
                self.registers.set_half_carry(false);
                self.registers.set_carry(new_carry != 0);
                4
            }

            0x1F => {
                // RRA
                let a = self.registers.a;
                let carry = u8::from(self.registers.carry());
                let new_carry = a & 1;
                self.registers.a = (a >> 1) | (carry << 7);
                self.registers.set_zero(false);
                self.registers.set_subtract(false);
                self.registers.set_half_carry(false);
                self.registers.set_carry(new_carry != 0);
                4
            }

            0x27 => {
                // DAA
                let mut a = self.registers.a;
                let mut carry = self.registers.carry();
                if self.registers.subtract() {
                    if self.registers.half_carry() {
                        a = a.wrapping_sub(0x06);
                    }
                    if self.registers.carry() {
                        a = a.wrapping_sub(0x60);
                    }
                } else {
                    let mut a_wide = u16::from(a);
                    if self.registers.half_carry() || (a & 0x0F) > 9 {
                        a_wide += 0x06;
                    }
                    if self.registers.carry() || (a_wide & 0x1F0) > 0x90 {
                        a_wide += 0x60;
                        carry = true;
                    } else {
                        carry = false;
                    }
                    #[allow(clippy::cast_possible_truncation)]
                    {
                        a = a_wide as u8;
                    }
                }
                self.registers.a = a;
                self.registers.set_zero(a == 0);
                self.registers.set_half_carry(false);
                self.registers.set_carry(carry);
                4
            }

            0x2F => {
                // CPL
                self.registers.a = !self.registers.a;
                self.registers.set_subtract(true);
                self.registers.set_half_carry(true);
                4
            }

            0x37 => {
                // SCF
                self.registers.set_subtract(false);
                self.registers.set_half_carry(false);
                self.registers.set_carry(true);
                4
            }

            0x3F => {
                // CCF
                self.registers.set_subtract(false);
                self.registers.set_half_carry(false);
                self.registers.set_carry(!self.registers.carry());
                4
            }

            0x10 => {
                // STOP
                // Consumes the next byte (0x00) as a second opcode byte
                self.registers.pc = self.registers.pc.wrapping_add(1);
                4
            }

            0x76 => panic!("Unimplemented opcode: 0x76"), // HALT

            0x40..=0x7F => {
                // LD r, r'
                let source = Reg::from_u3(opcode);
                let dest = Reg::from_u3(opcode >> 3);
                let value = self.read_register(source, bus);
                self.write_register(dest, value, bus);
                if matches!(source, Reg::IndirectHl) || matches!(dest, Reg::IndirectHl) {
                    8
                } else {
                    4
                }
            }

            0x04 | 0x0C | 0x14 | 0x1C | 0x24 | 0x2C | 0x34 | 0x3C => {
                // INC r
                let reg = Reg::from_u3(opcode >> 3);
                let value = self.read_register(reg, bus);
                let result = value.wrapping_add(1);
                self.write_register(reg, result, bus);
                self.registers.set_zero(result == 0);
                self.registers.set_subtract(false);
                self.registers.set_half_carry((value & 0x0F) == 0x0F);
                if matches!(reg, Reg::IndirectHl) {
                    12
                } else {
                    4
                }
            }

            0x05 | 0x0D | 0x15 | 0x1D | 0x25 | 0x2D | 0x35 | 0x3D => {
                // DEC r
                let reg = Reg::from_u3(opcode >> 3);
                let value = self.read_register(reg, bus);
                let result = value.wrapping_sub(1);
                self.write_register(reg, result, bus);
                self.registers.set_zero(result == 0);
                self.registers.set_subtract(true);
                #[allow(clippy::verbose_bit_mask)]
                self.registers.set_half_carry(value & 0x0F == 0);
                if matches!(reg, Reg::IndirectHl) {
                    12
                } else {
                    4
                }
            }

            0x80..=0xBF => {
                // ALU A, r — ADD, ADC, SUB, SBC, AND, XOR, OR, CP
                let op = (opcode >> 3) & 0b111;
                let source = Reg::from_u3(opcode);
                let value = self.read_register(source, bus);
                match op {
                    0 => self.add_a(value),
                    1 => self.adc_a(value),
                    2 => self.sub_a(value),
                    3 => self.sbc_a(value),
                    4 => self.and_a(value),
                    5 => self.xor_a(value),
                    6 => self.or_a(value),
                    7 => self.cp_a(value),
                    _ => unreachable!(),
                }
                if matches!(source, Reg::IndirectHl) {
                    8
                } else {
                    4
                }
            }

            0xC6 => {
                // ADD A, n
                let value = self.imm8(bus);
                self.add_a(value);
                8
            }

            0xCE => {
                // ADC A, n
                let value = self.imm8(bus);
                self.adc_a(value);
                8
            }

            0xD6 => {
                // SUB A, n
                let value = self.imm8(bus);
                self.sub_a(value);
                8
            }

            0xDE => {
                // SBC A, n
                let value = self.imm8(bus);
                self.sbc_a(value);
                8
            }

            0xE6 => {
                // AND A, n
                let value = self.imm8(bus);
                self.and_a(value);
                8
            }

            0xEE => {
                // XOR A, n
                let value = self.imm8(bus);
                self.xor_a(value);
                8
            }

            0xF6 => {
                // OR A, n
                let value = self.imm8(bus);
                self.or_a(value);
                8
            }

            0xFE => {
                // CP A, n
                let value = self.imm8(bus);
                self.cp_a(value);
                8
            }

            // Stack operations
            0xC1 | 0xD1 | 0xE1 | 0xF1 => {
                // POP rr
                let pair = (opcode >> 4) & 0b11;
                self.pop_rr(pair, bus);
                12
            }

            0xC5 | 0xD5 | 0xE5 | 0xF5 => {
                // PUSH rr
                let pair = (opcode >> 4) & 0b11;
                self.push_rr(pair, bus);
                16
            }

            0xEA => {
                // LD (nn), A
                let address = self.imm16(bus);
                bus.write(address, self.registers.a);
                16
            }

            0xFA => {
                // LD A, (nn)
                let address = self.imm16(bus);
                self.registers.a = bus.read(address);
                16
            }

            0xE9 => {
                // JP (HL)
                self.registers.pc = self.registers.hl();
                4
            }

            0xF9 => {
                // LD SP, HL
                self.registers.sp = self.registers.hl();
                8
            }

            0x08 => {
                // LD (nn), SP
                let address = self.imm16(bus);
                let sp = self.registers.sp;
                #[allow(clippy::cast_possible_truncation)]
                bus.write(address, sp as u8);
                #[allow(clippy::cast_possible_truncation)]
                bus.write(address.wrapping_add(1), (sp >> 8) as u8);
                20
            }

            0xE0 => {
                // LDH (n), A
                let offset = self.imm8(bus);
                bus.write(0xFF00 | u16::from(offset), self.registers.a);
                12
            }

            0xF0 => {
                // LDH A, (n)
                let offset = self.imm8(bus);
                self.registers.a = bus.read(0xFF00 | u16::from(offset));
                12
            }

            0xE2 => {
                // LDH (C), A
                bus.write(0xFF00 | u16::from(self.registers.c), self.registers.a);
                8
            }

            0xF2 => {
                // LDH A, (C)
                self.registers.a = bus.read(0xFF00 | u16::from(self.registers.c));
                8
            }

            0xF8 => {
                // LD HL, SP+e
                #[allow(clippy::cast_possible_wrap)]
                let offset = self.imm8(bus).cast_signed();
                let sp = self.registers.sp;
                #[allow(clippy::cast_sign_loss)]
                let result = sp.wrapping_add(offset as u16);
                self.registers.set_hl(result);
                self.registers.set_zero(false);
                self.registers.set_subtract(false);
                #[allow(clippy::cast_sign_loss)]
                self.registers
                    .set_half_carry((sp & 0x0F) + (offset as u16 & 0x0F) > 0x0F);
                #[allow(clippy::cast_sign_loss)]
                self.registers
                    .set_carry((sp & 0xFF) + (offset as u16 & 0xFF) > 0xFF);
                12
            }

            0xE8 => {
                // ADD SP, e
                #[allow(clippy::cast_possible_wrap)]
                let offset = self.imm8(bus).cast_signed();
                let sp = self.registers.sp;
                #[allow(clippy::cast_sign_loss)]
                let result = sp.wrapping_add(offset as u16);
                self.registers.sp = result;
                self.registers.set_zero(false);
                self.registers.set_subtract(false);
                #[allow(clippy::cast_sign_loss)]
                self.registers
                    .set_half_carry((sp & 0x0F) + (offset as u16 & 0x0F) > 0x0F);
                #[allow(clippy::cast_sign_loss)]
                self.registers
                    .set_carry((sp & 0xFF) + (offset as u16 & 0xFF) > 0xFF);
                16
            }

            // Jumps
            0xC3 => {
                // JP nn
                self.registers.pc = self.imm16(bus);
                16
            }

            0xC2 | 0xCA | 0xD2 | 0xDA => {
                // JP cc, nn
                let cc = (opcode >> 3) & 0b11;
                let address = self.imm16(bus);
                if self.check_condition(cc) {
                    self.registers.pc = address;
                    16
                } else {
                    12
                }
            }

            0x18 => {
                // JR d
                #[allow(clippy::cast_possible_wrap)]
                let offset = self.imm8(bus).cast_signed();
                #[allow(clippy::cast_sign_loss)]
                let target = self.registers.pc.wrapping_add(offset as u16);
                self.registers.pc = target;
                12
            }

            0x20 | 0x28 | 0x30 | 0x38 => {
                // JR cc, d
                let cc = (opcode >> 3) & 0b11;
                #[allow(clippy::cast_possible_wrap)]
                let offset = self.imm8(bus).cast_signed();
                if self.check_condition(cc) {
                    #[allow(clippy::cast_sign_loss)]
                    let target = self.registers.pc.wrapping_add(offset as u16);
                    self.registers.pc = target;
                    12
                } else {
                    8
                }
            }

            // Calls and returns
            0xCD => {
                // CALL nn
                let address = self.imm16(bus);
                self.push16(self.registers.pc, bus);
                self.registers.pc = address;
                24
            }

            0xC4 | 0xCC | 0xD4 | 0xDC => {
                // CALL cc, nn
                let cc = (opcode >> 3) & 0b11;
                let address = self.imm16(bus);
                if self.check_condition(cc) {
                    self.push16(self.registers.pc, bus);
                    self.registers.pc = address;
                    24
                } else {
                    12
                }
            }

            0xC9 => {
                // RET
                self.registers.pc = self.pop16(bus);
                16
            }

            0xC0 | 0xC8 | 0xD0 | 0xD8 => {
                // RET cc
                let cc = (opcode >> 3) & 0b11;
                if self.check_condition(cc) {
                    self.registers.pc = self.pop16(bus);
                    20
                } else {
                    8
                }
            }

            0xD9 => {
                // RETI (treated as RET for now; interrupt enable pending)
                self.registers.pc = self.pop16(bus);
                16
            }

            0xC7 | 0xCF | 0xD7 | 0xDF | 0xE7 | 0xEF | 0xF7 | 0xFF => {
                // RST n
                let address = u16::from(opcode & 0b0011_1000);
                self.push16(self.registers.pc, bus);
                self.registers.pc = address;
                16
            }

            0xCB => {
                // CB prefix: bit manipulation instructions
                let cb_opcode = self.imm8(bus);
                self.execute_cb(cb_opcode, bus)
            }

            _ => panic!(
                "Unimplemented opcode: 0x{opcode:02X} at PC={:04X}",
                self.registers.pc.wrapping_sub(1)
            ),
        }
    }

    /// Execute a CB-prefixed opcode.
    fn execute_cb(&mut self, opcode: u8, bus: &mut Bus) -> u8 {
        let reg = Reg::from_u3(opcode);
        let is_hl = matches!(reg, Reg::IndirectHl);
        let mut value = self.read_register(reg, bus);

        match opcode {
            0x00..=0x07 => {
                // RLC r
                let carry = value >> 7;
                value = (value << 1) | carry;
                self.registers.set_zero(value == 0);
                self.registers.set_subtract(false);
                self.registers.set_half_carry(false);
                self.registers.set_carry(carry != 0);
            }

            0x08..=0x0F => {
                // RRC r
                let carry = value & 1;
                value = (value >> 1) | (carry << 7);
                self.registers.set_zero(value == 0);
                self.registers.set_subtract(false);
                self.registers.set_half_carry(false);
                self.registers.set_carry(carry != 0);
            }

            0x10..=0x17 => {
                // RL r
                let carry_in = u8::from(self.registers.carry());
                let carry_out = value >> 7;
                value = (value << 1) | carry_in;
                self.registers.set_zero(value == 0);
                self.registers.set_subtract(false);
                self.registers.set_half_carry(false);
                self.registers.set_carry(carry_out != 0);
            }

            0x18..=0x1F => {
                // RR r
                let carry_in = u8::from(self.registers.carry());
                let carry_out = value & 1;
                value = (value >> 1) | (carry_in << 7);
                self.registers.set_zero(value == 0);
                self.registers.set_subtract(false);
                self.registers.set_half_carry(false);
                self.registers.set_carry(carry_out != 0);
            }

            0x20..=0x27 => {
                // SLA r
                let carry = value >> 7;
                value <<= 1;
                self.registers.set_zero(value == 0);
                self.registers.set_subtract(false);
                self.registers.set_half_carry(false);
                self.registers.set_carry(carry != 0);
            }

            0x28..=0x2F => {
                // SRA r
                let carry = value & 1;
                let msb = value & 0x80;
                value = (value >> 1) | msb;
                self.registers.set_zero(value == 0);
                self.registers.set_subtract(false);
                self.registers.set_half_carry(false);
                self.registers.set_carry(carry != 0);
            }

            0x30..=0x37 => {
                // SWAP r
                value = value.rotate_left(4);
                self.registers.set_zero(value == 0);
                self.registers.set_subtract(false);
                self.registers.set_half_carry(false);
                self.registers.set_carry(false);
            }

            0x38..=0x3F => {
                // SRL r
                let carry = value & 1;
                value >>= 1;
                self.registers.set_zero(value == 0);
                self.registers.set_subtract(false);
                self.registers.set_half_carry(false);
                self.registers.set_carry(carry != 0);
            }

            0x40..=0x7F => {
                // BIT n, r
                let bit = (opcode >> 3) & 0b111;
                self.registers.set_zero((value & (1 << bit)) == 0);
                self.registers.set_subtract(false);
                self.registers.set_half_carry(true);
                // C unchanged
                return if is_hl { 12 } else { 8 };
            }

            0x80..=0xBF => {
                // RES n, r
                let bit = (opcode >> 3) & 0b111;
                value &= !(1 << bit);
            }

            0xC0..=0xFF => {
                // SET n, r
                let bit = (opcode >> 3) & 0b111;
                value |= 1 << bit;
            }
        }

        self.write_register(reg, value, bus);

        if is_hl { 16 } else { 8 }
    }

    /// Read an immediate byte from `PC` and advance `PC` by 1.
    #[must_use]
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

    fn read_register(&self, reg: Reg, bus: &Bus) -> u8 {
        match reg {
            Reg::B => self.registers.b,
            Reg::C => self.registers.c,
            Reg::D => self.registers.d,
            Reg::E => self.registers.e,
            Reg::H => self.registers.h,
            Reg::L => self.registers.l,
            Reg::IndirectHl => bus.read(self.registers.hl()),
            Reg::A => self.registers.a,
        }
    }

    fn write_register(&mut self, reg: Reg, value: u8, bus: &mut Bus) {
        match reg {
            Reg::B => self.registers.b = value,
            Reg::C => self.registers.c = value,
            Reg::D => self.registers.d = value,
            Reg::E => self.registers.e = value,
            Reg::H => self.registers.h = value,
            Reg::L => self.registers.l = value,
            Reg::IndirectHl => bus.write(self.registers.hl(), value),
            Reg::A => self.registers.a = value,
        }
    }

    #[must_use]
    fn read_rr(&self, pair: u8) -> u16 {
        match pair {
            0 => self.registers.bc(),
            1 => self.registers.de(),
            2 => self.registers.hl(),
            3 => self.registers.sp,
            _ => unreachable!(),
        }
    }

    fn write_rr(&mut self, pair: u8, value: u16) {
        match pair {
            0 => self.registers.set_bc(value),
            1 => self.registers.set_de(value),
            2 => self.registers.set_hl(value),
            3 => self.registers.sp = value,
            _ => unreachable!(),
        }
    }

    fn inc_rr(&mut self, pair: u8) {
        let value = self.read_rr(pair);
        self.write_rr(pair, value.wrapping_add(1));
    }

    fn dec_rr(&mut self, pair: u8) {
        let value = self.read_rr(pair);
        self.write_rr(pair, value.wrapping_sub(1));
    }

    fn add_hl_rr(&mut self, pair: u8) {
        let hl = self.registers.hl();
        let value = self.read_rr(pair);
        let result = hl.wrapping_add(value);
        self.registers.set_hl(result);
        self.registers.set_subtract(false);
        self.registers
            .set_half_carry((hl & 0x0FFF) + (value & 0x0FFF) > 0x0FFF);
        self.registers.set_carry(hl > 0xFFFF - value);
    }

    /// Read a 16-bit immediate from `PC` and advance `PC` by 2 (little-endian).
    #[must_use]
    fn imm16(&mut self, bus: &Bus) -> u16 {
        let low = bus.read(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);
        let high = bus.read(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);
        u16::from(high) << 8 | u16::from(low)
    }

    fn adc_a(&mut self, value: u8) {
        let a = self.registers.a;
        let carry = u8::from(self.registers.carry());
        let result = a.wrapping_add(value).wrapping_add(carry);
        self.registers.a = result;
        self.registers.set_zero(result == 0);
        self.registers.set_subtract(false);
        self.registers
            .set_half_carry((a & 0x0F) + (value & 0x0F) + carry > 0x0F);
        self.registers
            .set_carry(u16::from(a) + u16::from(value) + u16::from(carry) > 0xFF);
    }

    fn sub_a(&mut self, value: u8) {
        let a = self.registers.a;
        let result = a.wrapping_sub(value);
        self.registers.a = result;
        self.registers.set_zero(result == 0);
        self.registers.set_subtract(true);
        self.registers.set_half_carry((a & 0x0F) < (value & 0x0F));
        self.registers.set_carry(a < value);
    }

    fn sbc_a(&mut self, value: u8) {
        let a = self.registers.a;
        let carry = u8::from(self.registers.carry());
        let result = a.wrapping_sub(value).wrapping_sub(carry);
        self.registers.a = result;
        self.registers.set_zero(result == 0);
        self.registers.set_subtract(true);
        self.registers
            .set_half_carry((a & 0x0F) < (value & 0x0F) + carry);
        self.registers
            .set_carry(u16::from(a) < u16::from(value) + u16::from(carry));
    }

    fn and_a(&mut self, value: u8) {
        let result = self.registers.a & value;
        self.registers.a = result;
        self.registers.set_zero(result == 0);
        self.registers.set_subtract(false);
        self.registers.set_half_carry(true);
        self.registers.set_carry(false);
    }

    fn xor_a(&mut self, value: u8) {
        let result = self.registers.a ^ value;
        self.registers.a = result;
        self.registers.set_zero(result == 0);
        self.registers.set_subtract(false);
        self.registers.set_half_carry(false);
        self.registers.set_carry(false);
    }

    fn or_a(&mut self, value: u8) {
        let result = self.registers.a | value;
        self.registers.a = result;
        self.registers.set_zero(result == 0);
        self.registers.set_subtract(false);
        self.registers.set_half_carry(false);
        self.registers.set_carry(false);
    }

    fn cp_a(&mut self, value: u8) {
        let a = self.registers.a;
        let result = a.wrapping_sub(value);
        self.registers.set_zero(result == 0);
        self.registers.set_subtract(true);
        self.registers.set_half_carry((a & 0x0F) < (value & 0x0F));
        self.registers.set_carry(a < value);
    }

    fn push_rr(&mut self, pair: u8, bus: &mut Bus) {
        let value = match pair {
            0 => self.registers.bc(),
            1 => self.registers.de(),
            2 => self.registers.hl(),
            3 => self.registers.af(),
            _ => unreachable!(),
        };
        self.push16(value, bus);
    }

    fn pop_rr(&mut self, pair: u8, bus: &Bus) {
        let value = self.pop16(bus);
        match pair {
            0 => self.registers.set_bc(value),
            1 => self.registers.set_de(value),
            2 => self.registers.set_hl(value),
            3 => self.registers.set_af(value),
            _ => unreachable!(),
        }
    }

    fn push16(&mut self, value: u16, bus: &mut Bus) {
        self.registers.sp = self.registers.sp.wrapping_sub(1);
        #[allow(clippy::cast_possible_truncation)]
        bus.write(self.registers.sp, (value >> 8) as u8);
        self.registers.sp = self.registers.sp.wrapping_sub(1);
        #[allow(clippy::cast_possible_truncation)]
        bus.write(self.registers.sp, value as u8);
    }

    #[must_use]
    fn pop16(&mut self, bus: &Bus) -> u16 {
        let low = bus.read(self.registers.sp);
        self.registers.sp = self.registers.sp.wrapping_add(1);
        let high = bus.read(self.registers.sp);
        self.registers.sp = self.registers.sp.wrapping_add(1);
        u16::from(high) << 8 | u16::from(low)
    }

    fn check_condition(&self, cc: u8) -> bool {
        match cc {
            0 => !self.registers.zero(),  // NZ
            1 => self.registers.zero(),   // Z
            2 => !self.registers.carry(), // NC
            3 => self.registers.carry(),  // C
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

    #[test]
    fn test_inc_rr() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_bc(0x0001);
        bus.write(0x0000, 0x03); // INC BC

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 8);
        assert_eq!(cpu.registers.bc(), 0x0002);
    }

    #[test]
    fn test_inc_rr_overflow() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_hl(0xFFFF);
        cpu.registers.set_zero(true);
        cpu.registers.set_subtract(true);
        cpu.registers.set_half_carry(true);
        cpu.registers.set_carry(true);
        bus.write(0x0000, 0x23); // INC HL

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.hl(), 0x0000);
        // INC rr does not affect any flags.
        assert!(cpu.registers.zero());
        assert!(cpu.registers.subtract());
        assert!(cpu.registers.half_carry());
        assert!(cpu.registers.carry());
    }

    #[test]
    fn test_inc_rr_preserves_flags() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_bc(0x0001);
        cpu.registers.set_zero(true);
        cpu.registers.set_subtract(true);
        cpu.registers.set_half_carry(true);
        cpu.registers.set_carry(true);
        bus.write(0x0000, 0x03); // INC BC

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.bc(), 0x0002);
        // INC rr does not affect any flags.
        assert!(cpu.registers.zero());
        assert!(cpu.registers.subtract());
        assert!(cpu.registers.half_carry());
        assert!(cpu.registers.carry());
    }

    #[test]
    fn test_dec_rr() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_de(0x0002);
        bus.write(0x0000, 0x1B); // DEC DE

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 8);
        assert_eq!(cpu.registers.de(), 0x0001);
    }

    #[test]
    fn test_dec_rr_underflow() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.sp = 0x0000;
        cpu.registers.set_zero(true);
        cpu.registers.set_subtract(true);
        cpu.registers.set_half_carry(true);
        cpu.registers.set_carry(true);
        bus.write(0x0000, 0x3B); // DEC SP

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.sp, 0xFFFF);
        // DEC rr does not affect any flags.
        assert!(cpu.registers.zero());
        assert!(cpu.registers.subtract());
        assert!(cpu.registers.half_carry());
        assert!(cpu.registers.carry());
    }

    #[test]
    fn test_dec_rr_preserves_flags() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_bc(0x0001);
        cpu.registers.set_zero(true);
        cpu.registers.set_subtract(true);
        cpu.registers.set_half_carry(true);
        cpu.registers.set_carry(true);
        bus.write(0x0000, 0x0B); // DEC BC

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.bc(), 0x0000);
        // DEC rr does not affect any flags.
        assert!(cpu.registers.zero());
        assert!(cpu.registers.subtract());
        assert!(cpu.registers.half_carry());
        assert!(cpu.registers.carry());
    }

    #[test]
    fn test_add_hl_rr() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_hl(0x1234);
        cpu.registers.set_bc(0x5678);
        bus.write(0x0000, 0x09); // ADD HL, BC

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 8);
        assert_eq!(cpu.registers.hl(), 0x68AC);
        assert!(!cpu.registers.zero());
        assert!(!cpu.registers.subtract());
        assert!(!cpu.registers.half_carry());
        assert!(!cpu.registers.carry());
    }

    #[test]
    fn test_add_hl_rr_zero_result_preserves_z() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_hl(0x0000);
        cpu.registers.set_bc(0x0000);
        cpu.registers.set_zero(true);
        bus.write(0x0000, 0x09); // ADD HL, BC

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.hl(), 0x0000);
        assert!(cpu.registers.zero());
        assert!(!cpu.registers.subtract());
        assert!(!cpu.registers.half_carry());
        assert!(!cpu.registers.carry());
    }

    #[test]
    fn test_add_hl_rr_carry() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_hl(0x8000);
        cpu.registers.set_zero(false);
        bus.write(0x0000, 0x29); // ADD HL, HL

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.hl(), 0x0000);
        assert!(!cpu.registers.zero());
        assert!(!cpu.registers.subtract());
        assert!(!cpu.registers.half_carry());
        assert!(cpu.registers.carry());
    }

    #[test]
    fn test_add_hl_rr_half_carry() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_hl(0x0FFF);
        cpu.registers.set_bc(0x0001);
        bus.write(0x0000, 0x09); // ADD HL, BC

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.hl(), 0x1000);
        assert!(!cpu.registers.zero());
        assert!(!cpu.registers.subtract());
        assert!(cpu.registers.half_carry());
        assert!(!cpu.registers.carry());
    }

    #[test]
    fn test_add_hl_rr_preserves_z() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_hl(0x0001);
        cpu.registers.set_de(0x0001);
        cpu.registers.set_zero(true);
        bus.write(0x0000, 0x19); // ADD HL, DE

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.hl(), 0x0002);
        // ADD HL, rr does not affect Z.
        assert!(cpu.registers.zero());
        assert!(!cpu.registers.subtract());
        assert!(!cpu.registers.half_carry());
        assert!(!cpu.registers.carry());
    }

    #[test]
    fn test_add_hl_rr_sp() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_hl(0x1000);
        cpu.registers.sp = 0x0200;
        bus.write(0x0000, 0x39); // ADD HL, SP

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 8);
        assert_eq!(cpu.registers.hl(), 0x1200);
        assert_eq!(cpu.registers.sp, 0x0200);
        assert!(!cpu.registers.subtract());
        assert!(!cpu.registers.half_carry());
        assert!(!cpu.registers.carry());
    }

    #[test]
    fn test_add_hl_rr_both_h_and_c() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_hl(0x8FFF);
        bus.write(0x0000, 0x29); // ADD HL, HL

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.hl(), 0x1FFE);
        assert!(!cpu.registers.zero());
        assert!(!cpu.registers.subtract());
        assert!(cpu.registers.half_carry());
        assert!(cpu.registers.carry());
    }

    #[test]
    fn test_add_hl_rr_sp_carry() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_hl(0xF000);
        cpu.registers.sp = 0x1000;
        bus.write(0x0000, 0x39); // ADD HL, SP

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.hl(), 0x0000);
        assert!(!cpu.registers.zero());
        assert!(!cpu.registers.subtract());
        assert!(!cpu.registers.half_carry());
        assert!(cpu.registers.carry());
    }

    #[test]
    fn test_inc_rr_de() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_de(0x1234);
        bus.write(0x0000, 0x13); // INC DE

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.de(), 0x1235);
    }

    #[test]
    fn test_inc_rr_sp() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.sp = 0xFFFE;
        bus.write(0x0000, 0x33); // INC SP

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.sp, 0xFFFF);
    }

    #[test]
    fn test_dec_rr_hl() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_hl(0x0100);
        bus.write(0x0000, 0x2B); // DEC HL

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.hl(), 0x00FF);
    }

    // ADC tests
    #[test]
    fn test_adc_a_r() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0x01;
        cpu.registers.b = 0x02;
        bus.write(0x0000, 0x88); // ADC A, B

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 4);
        assert_eq!(cpu.registers.a, 0x03);
        assert!(!cpu.registers.zero());
        assert!(!cpu.registers.subtract());
        assert!(!cpu.registers.half_carry());
        assert!(!cpu.registers.carry());
    }

    #[test]
    fn test_adc_a_r_with_carry() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0x01;
        cpu.registers.b = 0x02;
        cpu.registers.set_carry(true);
        bus.write(0x0000, 0x88); // ADC A, B

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.a, 0x04);
    }

    #[test]
    fn test_adc_a_r_half_carry() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0x0F;
        cpu.registers.b = 0x01;
        bus.write(0x0000, 0x88); // ADC A, B

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.a, 0x10);
        assert!(cpu.registers.half_carry());
        assert!(!cpu.registers.carry());
    }

    #[test]
    fn test_adc_a_r_carry() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0xFF;
        cpu.registers.b = 0x01;
        bus.write(0x0000, 0x88); // ADC A, B

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.a, 0x00);
        assert!(cpu.registers.zero());
        assert!(cpu.registers.half_carry());
        assert!(cpu.registers.carry());
    }

    #[test]
    fn test_adc_a_n() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0x12;
        bus.write(0x0000, 0xCE); // ADC A, n
        bus.write(0x0001, 0x34);

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 8);
        assert_eq!(cpu.registers.a, 0x46);
    }

    // SUB tests
    #[test]
    fn test_sub_a_r() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0x05;
        cpu.registers.b = 0x03;
        bus.write(0x0000, 0x90); // SUB A, B

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 4);
        assert_eq!(cpu.registers.a, 0x02);
        assert!(!cpu.registers.zero());
        assert!(cpu.registers.subtract());
        assert!(!cpu.registers.half_carry());
        assert!(!cpu.registers.carry());
    }

    #[test]
    fn test_sub_a_r_zero() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0x03;
        cpu.registers.b = 0x03;
        bus.write(0x0000, 0x90); // SUB A, B

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.a, 0x00);
        assert!(cpu.registers.zero());
        assert!(cpu.registers.subtract());
        assert!(!cpu.registers.half_carry());
        assert!(!cpu.registers.carry());
    }

    #[test]
    fn test_sub_a_r_half_carry() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0x10;
        cpu.registers.b = 0x01;
        bus.write(0x0000, 0x90); // SUB A, B

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.a, 0x0F);
        assert!(cpu.registers.half_carry());
        assert!(!cpu.registers.carry());
    }

    #[test]
    fn test_sub_a_r_carry() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0x01;
        cpu.registers.b = 0x02;
        bus.write(0x0000, 0x90); // SUB A, B

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.a, 0xFF);
        assert!(!cpu.registers.zero());
        assert!(cpu.registers.subtract());
        assert!(cpu.registers.half_carry());
        assert!(cpu.registers.carry());
    }

    #[test]
    fn test_sub_a_n() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0x12;
        bus.write(0x0000, 0xD6); // SUB A, n
        bus.write(0x0001, 0x02);

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 8);
        assert_eq!(cpu.registers.a, 0x10);
    }

    // SBC tests
    #[test]
    fn test_sbc_a_r() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0x05;
        cpu.registers.b = 0x03;
        bus.write(0x0000, 0x98); // SBC A, B

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 4);
        assert_eq!(cpu.registers.a, 0x02);
        assert!(!cpu.registers.zero());
        assert!(cpu.registers.subtract());
        assert!(!cpu.registers.half_carry());
        assert!(!cpu.registers.carry());
    }

    #[test]
    fn test_sbc_a_r_with_carry() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0x05;
        cpu.registers.b = 0x03;
        cpu.registers.set_carry(true);
        bus.write(0x0000, 0x98); // SBC A, B

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.a, 0x01);
    }

    #[test]
    fn test_sbc_a_r_borrow() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0x00;
        cpu.registers.b = 0x01;
        cpu.registers.set_carry(true);
        bus.write(0x0000, 0x98); // SBC A, B

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.a, 0xFE);
        assert!(cpu.registers.carry());
        assert!(cpu.registers.half_carry());
    }

    // AND tests
    #[test]
    fn test_and_a_r() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0b1010_1010;
        cpu.registers.b = 0b1100_1100;
        bus.write(0x0000, 0xA0); // AND A, B

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 4);
        assert_eq!(cpu.registers.a, 0b1000_1000);
        assert!(!cpu.registers.zero());
        assert!(!cpu.registers.subtract());
        assert!(cpu.registers.half_carry());
        assert!(!cpu.registers.carry());
    }

    #[test]
    fn test_and_a_r_zero() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0b1010_1010;
        cpu.registers.b = 0b0101_0101;
        bus.write(0x0000, 0xA0); // AND A, B

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.a, 0x00);
        assert!(cpu.registers.zero());
    }

    #[test]
    fn test_and_a_n() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0xFF;
        bus.write(0x0000, 0xE6); // AND A, n
        bus.write(0x0001, 0x0F);

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.a, 0x0F);
    }

    // XOR tests
    #[test]
    fn test_xor_a_r() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0b1010_1010;
        cpu.registers.b = 0b1100_1100;
        bus.write(0x0000, 0xA8); // XOR A, B

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 4);
        assert_eq!(cpu.registers.a, 0b0110_0110);
        assert!(!cpu.registers.zero());
        assert!(!cpu.registers.subtract());
        assert!(!cpu.registers.half_carry());
        assert!(!cpu.registers.carry());
    }

    #[test]
    fn test_xor_a_r_zero() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0x42;
        cpu.registers.b = 0x42;
        bus.write(0x0000, 0xA8); // XOR A, B

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.a, 0x00);
        assert!(cpu.registers.zero());
    }

    // OR tests
    #[test]
    fn test_or_a_r() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0b1010_1010;
        cpu.registers.b = 0b0101_0101;
        bus.write(0x0000, 0xB0); // OR A, B

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 4);
        assert_eq!(cpu.registers.a, 0xFF);
        assert!(!cpu.registers.zero());
        assert!(!cpu.registers.subtract());
        assert!(!cpu.registers.half_carry());
        assert!(!cpu.registers.carry());
    }

    #[test]
    fn test_or_a_r_zero() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0x00;
        cpu.registers.b = 0x00;
        bus.write(0x0000, 0xB0); // OR A, B

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.a, 0x00);
        assert!(cpu.registers.zero());
    }

    // CP tests
    #[test]
    fn test_cp_a_r_equal() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0x42;
        cpu.registers.b = 0x42;
        bus.write(0x0000, 0xB8); // CP A, B

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 4);
        assert_eq!(cpu.registers.a, 0x42); // A unchanged
        assert!(cpu.registers.zero());
        assert!(cpu.registers.subtract());
        assert!(!cpu.registers.half_carry());
        assert!(!cpu.registers.carry());
    }

    #[test]
    fn test_cp_a_r_less() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0x01;
        cpu.registers.b = 0x02;
        bus.write(0x0000, 0xB8); // CP A, B

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.a, 0x01);
        assert!(!cpu.registers.zero());
        assert!(cpu.registers.subtract());
        assert!(cpu.registers.half_carry());
        assert!(cpu.registers.carry());
    }

    #[test]
    fn test_cp_a_r_greater() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0x02;
        cpu.registers.b = 0x01;
        bus.write(0x0000, 0xB8); // CP A, B

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.a, 0x02);
        assert!(!cpu.registers.zero());
        assert!(cpu.registers.subtract());
        assert!(!cpu.registers.half_carry());
        assert!(!cpu.registers.carry());
    }

    #[test]
    fn test_cp_a_n() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0x42;
        bus.write(0x0000, 0xFE); // CP A, n
        bus.write(0x0001, 0x42);

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.a, 0x42);
        assert!(cpu.registers.zero());
    }

    // LD rr, nn tests
    #[test]
    fn test_ld_rr_nn() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        bus.write(0x0000, 0x01); // LD BC, nn
        bus.write(0x0001, 0x34);
        bus.write(0x0002, 0x12);

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 12);
        assert_eq!(cpu.registers.bc(), 0x1234);
        assert_eq!(cpu.registers.pc, 0x0003);
    }

    #[test]
    fn test_ld_sp_nn() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        bus.write(0x0000, 0x31); // LD SP, nn
        bus.write(0x0001, 0xFE);
        bus.write(0x0002, 0xFF);

        cpu.step(&mut bus);

        assert_eq!(cpu.registers.sp, 0xFFFE);
    }

    #[test]
    fn test_ld_hl_indirect_n() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_hl(0xC000);
        bus.write(0x0000, 0x36); // LD (HL), n
        bus.write(0x0001, 0xAB);

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 12);
        assert_eq!(bus.read(0xC000), 0xAB);
        assert_eq!(cpu.registers.pc, 0x0002);
    }

    // PUSH / POP tests
    #[test]
    fn test_push_pop_bc() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.sp = 0xFFFE;
        cpu.registers.set_bc(0x1234);
        bus.write(0x0000, 0xC5); // PUSH BC
        bus.write(0x0001, 0xC1); // POP BC

        cpu.step(&mut bus);
        assert_eq!(cpu.registers.sp, 0xFFFC);
        assert_eq!(bus.read(0xFFFD), 0x12);
        assert_eq!(bus.read(0xFFFC), 0x34);

        cpu.registers.set_bc(0x0000);
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.bc(), 0x1234);
        assert_eq!(cpu.registers.sp, 0xFFFE);
    }

    #[test]
    fn test_push_pop_af() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.sp = 0xFFFE;
        cpu.registers.a = 0x12;
        cpu.registers.set_f(0xF0); // Only upper nibble of F is valid
        bus.write(0x0000, 0xF5); // PUSH AF
        bus.write(0x0001, 0xF1); // POP AF

        cpu.step(&mut bus);
        // F lower nibble should be masked to 0 when pushed
        assert_eq!(bus.read(0xFFFD), 0x12);
        assert_eq!(bus.read(0xFFFC), 0xF0);

        bus.write(0xFFFC, 0xFF); // Corrupt lower nibble on stack
        cpu.registers.a = 0x00;
        cpu.registers.set_f(0x00);
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.a, 0x12);
        assert_eq!(cpu.registers.af() & 0x00FF, 0xF0); // Lower nibble masked on POP
    }

    // JP tests
    #[test]
    fn test_jp_nn() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        bus.write(0x0000, 0xC3); // JP nn
        bus.write(0x0001, 0x34);
        bus.write(0x0002, 0x12);

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 16);
        assert_eq!(cpu.registers.pc, 0x1234);
    }

    #[test]
    fn test_jp_nz_taken() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_zero(false);
        bus.write(0x0000, 0xC2); // JP NZ, nn
        bus.write(0x0001, 0x34);
        bus.write(0x0002, 0x12);

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 16);
        assert_eq!(cpu.registers.pc, 0x1234);
    }

    #[test]
    fn test_jp_nz_not_taken() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_zero(true);
        bus.write(0x0000, 0xC2); // JP NZ, nn
        bus.write(0x0001, 0x34);
        bus.write(0x0002, 0x12);

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 12);
        assert_eq!(cpu.registers.pc, 0x0003);
    }

    #[test]
    fn test_jp_z_taken() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_zero(true);
        bus.write(0x0000, 0xCA); // JP Z, nn
        bus.write(0x0001, 0x34);
        bus.write(0x0002, 0x12);

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 16);
        assert_eq!(cpu.registers.pc, 0x1234);
    }

    // JR tests
    #[test]
    fn test_jr_d_forward() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        bus.write(0x0000, 0x18); // JR d
        bus.write(0x0001, 0x05); // +5

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 12);
        assert_eq!(cpu.registers.pc, 0x0007); // 0x0002 + 5
    }

    #[test]
    fn test_jr_d_backward() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.pc = 0x0010;
        bus.write(0x0010, 0x18); // JR d
        bus.write(0x0011, 0xFB); // -5

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 12);
        assert_eq!(cpu.registers.pc, 0x000D); // 0x0012 - 5
    }

    #[test]
    fn test_jr_nz_taken() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_zero(false);
        bus.write(0x0000, 0x20); // JR NZ, d
        bus.write(0x0001, 0x05); // +5

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 12);
        assert_eq!(cpu.registers.pc, 0x0007);
    }

    #[test]
    fn test_jr_nz_not_taken() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_zero(true);
        bus.write(0x0000, 0x20); // JR NZ, d
        bus.write(0x0001, 0x05); // +5

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 8);
        assert_eq!(cpu.registers.pc, 0x0002);
    }

    // CALL / RET tests
    #[test]
    fn test_call_ret() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.sp = 0xFFFE;
        bus.write(0x0000, 0xCD); // CALL nn
        bus.write(0x0001, 0x34);
        bus.write(0x0002, 0x12);
        bus.write(0x1234, 0xC9); // RET

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 24);
        assert_eq!(cpu.registers.pc, 0x1234);
        assert_eq!(cpu.registers.sp, 0xFFFC);
        assert_eq!(bus.read(0xFFFD), 0x00);
        assert_eq!(bus.read(0xFFFC), 0x03); // Return address

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 16);
        assert_eq!(cpu.registers.pc, 0x0003);
        assert_eq!(cpu.registers.sp, 0xFFFE);
    }

    #[test]
    fn test_call_nz_taken() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.sp = 0xFFFE;
        cpu.registers.set_zero(false);
        bus.write(0x0000, 0xC4); // CALL NZ, nn
        bus.write(0x0001, 0x34);
        bus.write(0x0002, 0x12);

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 24);
        assert_eq!(cpu.registers.pc, 0x1234);
        assert_eq!(cpu.registers.sp, 0xFFFC);
    }

    #[test]
    fn test_call_nz_not_taken() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.sp = 0xFFFE;
        cpu.registers.set_zero(true);
        bus.write(0x0000, 0xC4); // CALL NZ, nn
        bus.write(0x0001, 0x34);
        bus.write(0x0002, 0x12);

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 12);
        assert_eq!(cpu.registers.pc, 0x0003);
        assert_eq!(cpu.registers.sp, 0xFFFE);
    }

    #[test]
    fn test_ret_nz_taken() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.sp = 0xFFFC;
        cpu.registers.set_zero(false);
        bus.write(0xFFFC, 0x34);
        bus.write(0xFFFD, 0x12);
        bus.write(0x0000, 0xC0); // RET NZ

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 20);
        assert_eq!(cpu.registers.pc, 0x1234);
        assert_eq!(cpu.registers.sp, 0xFFFE);
    }

    #[test]
    fn test_ret_nz_not_taken() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.sp = 0xFFFE;
        cpu.registers.set_zero(true);
        bus.write(0x0000, 0xC0); // RET NZ

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 8);
        assert_eq!(cpu.registers.pc, 0x0001);
        assert_eq!(cpu.registers.sp, 0xFFFE);
    }

    // RST tests
    #[test]
    fn test_rst_0() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.sp = 0xFFFE;
        bus.write(0x0000, 0xC7); // RST 0x00

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 16);
        assert_eq!(cpu.registers.pc, 0x0000);
        assert_eq!(cpu.registers.sp, 0xFFFC);
        assert_eq!(bus.read(0xFFFD), 0x00);
        assert_eq!(bus.read(0xFFFC), 0x01);
    }

    #[test]
    fn test_rst_38() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.sp = 0xFFFE;
        bus.write(0x0000, 0xFF); // RST 0x38

        let cycles = cpu.step(&mut bus);

        assert_eq!(cycles, 16);
        assert_eq!(cpu.registers.pc, 0x0038);
        assert_eq!(cpu.registers.sp, 0xFFFC);
    }

    #[test]
    fn test_call_ret_nested() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.sp = 0xFFFE;

        // 0x0000: CALL 0x0100
        bus.write(0x0000, 0xCD);
        bus.write(0x0001, 0x00);
        bus.write(0x0002, 0x01);

        // 0x0100: CALL 0x0200
        bus.write(0x0100, 0xCD);
        bus.write(0x0101, 0x00);
        bus.write(0x0102, 0x02);

        // 0x0200: RET
        bus.write(0x0200, 0xC9);

        // 0x0103: RET
        bus.write(0x0103, 0xC9);

        cpu.step(&mut bus);
        assert_eq!(cpu.registers.pc, 0x0100);
        assert_eq!(cpu.registers.sp, 0xFFFC);

        cpu.step(&mut bus);
        assert_eq!(cpu.registers.pc, 0x0200);
        assert_eq!(cpu.registers.sp, 0xFFFA);

        cpu.step(&mut bus);
        assert_eq!(cpu.registers.pc, 0x0103);
        assert_eq!(cpu.registers.sp, 0xFFFC);

        cpu.step(&mut bus);
        assert_eq!(cpu.registers.pc, 0x0003);
        assert_eq!(cpu.registers.sp, 0xFFFE);
    }

    // --- DAA ---

    #[test]
    fn test_daa_after_add_no_adjust() {
        // 0x05 + 0x03 = 0x08, BCD digits all < 10, no adjust needed
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0x08;
        cpu.registers.set_subtract(false);
        cpu.registers.set_half_carry(false);
        cpu.registers.set_carry(false);
        bus.write(0x0000, 0x27); // DAA
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.a, 0x08);
        assert!(!cpu.registers.carry());
        assert!(!cpu.registers.zero());
    }

    #[test]
    fn test_daa_after_add_low_nibble_overflow() {
        // 0x0A is invalid BCD — DAA should add 0x06
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0x0A;
        cpu.registers.set_subtract(false);
        cpu.registers.set_half_carry(false);
        cpu.registers.set_carry(false);
        bus.write(0x0000, 0x27); // DAA
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.a, 0x10);
        assert!(!cpu.registers.carry());
    }

    #[test]
    fn test_daa_after_add_carry() {
        // result > 0x99 sets carry, adds 0x60
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0x9A;
        cpu.registers.set_subtract(false);
        cpu.registers.set_half_carry(false);
        cpu.registers.set_carry(false);
        bus.write(0x0000, 0x27); // DAA
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.a, 0x00);
        assert!(cpu.registers.carry());
        assert!(cpu.registers.zero());
    }

    #[test]
    fn test_daa_after_sub_no_adjust() {
        // After subtraction with no borrows, no adjustment
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0x05;
        cpu.registers.set_subtract(true);
        cpu.registers.set_half_carry(false);
        cpu.registers.set_carry(false);
        bus.write(0x0000, 0x27); // DAA
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.a, 0x05);
        assert!(!cpu.registers.carry());
    }

    #[test]
    fn test_daa_after_sub_half_carry() {
        // After subtraction with half-carry (borrow from low nibble), subtract 0x06
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0x0F; // e.g. result after borrow
        cpu.registers.set_subtract(true);
        cpu.registers.set_half_carry(true);
        cpu.registers.set_carry(false);
        bus.write(0x0000, 0x27); // DAA
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.a, 0x09);
        assert!(!cpu.registers.carry());
    }

    #[test]
    fn test_daa_after_sub_with_carry_no_half_carry() {
        // After subtraction with carry, subtract 0x60
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0x40;
        cpu.registers.set_subtract(true);
        cpu.registers.set_half_carry(false);
        cpu.registers.set_carry(true);
        bus.write(0x0000, 0x27); // DAA
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.a, 0xE0);
        assert!(cpu.registers.carry());
    }

    #[test]
    fn test_daa_after_sub_with_carry_and_half_carry() {
        // After subtraction with both carry and half-carry, subtract 0x66
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0x73;
        cpu.registers.set_subtract(true);
        cpu.registers.set_half_carry(true);
        cpu.registers.set_carry(true);
        bus.write(0x0000, 0x27); // DAA
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.a, 0x0D);
        assert!(cpu.registers.carry());
    }

    // --- Rotates: RLCA, RRCA, RLA, RRA ---

    #[test]
    fn test_rlca() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0b1000_0001;
        bus.write(0x0000, 0x07); // RLCA
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.a, 0b0000_0011);
        assert!(cpu.registers.carry());
        assert!(!cpu.registers.zero());
        assert!(!cpu.registers.subtract());
        assert!(!cpu.registers.half_carry());
    }

    #[test]
    fn test_rrca() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0b0000_0011;
        bus.write(0x0000, 0x0F); // RRCA
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.a, 0b1000_0001);
        assert!(cpu.registers.carry());
    }

    #[test]
    fn test_rla() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0b0100_0000;
        cpu.registers.set_carry(true);
        bus.write(0x0000, 0x17); // RLA
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.a, 0b1000_0001);
        assert!(!cpu.registers.carry());
    }

    #[test]
    fn test_rra() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0b0000_0010;
        cpu.registers.set_carry(true);
        bus.write(0x0000, 0x1F); // RRA
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.a, 0b1000_0001);
        assert!(!cpu.registers.carry());
    }

    // --- CPL, SCF, CCF ---

    #[test]
    fn test_cpl() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0b1010_0101;
        bus.write(0x0000, 0x2F); // CPL
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.a, 0b0101_1010);
        assert!(cpu.registers.subtract());
        assert!(cpu.registers.half_carry());
    }

    #[test]
    fn test_scf() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_carry(false);
        cpu.registers.set_subtract(true);
        cpu.registers.set_half_carry(true);
        bus.write(0x0000, 0x37); // SCF
        cpu.step(&mut bus);
        assert!(cpu.registers.carry());
        assert!(!cpu.registers.subtract());
        assert!(!cpu.registers.half_carry());
    }

    #[test]
    fn test_ccf_set_to_clear() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_carry(true);
        bus.write(0x0000, 0x3F); // CCF
        cpu.step(&mut bus);
        assert!(!cpu.registers.carry());
        assert!(!cpu.registers.subtract());
        assert!(!cpu.registers.half_carry());
    }

    #[test]
    fn test_ccf_clear_to_set() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_carry(false);
        bus.write(0x0000, 0x3F); // CCF
        cpu.step(&mut bus);
        assert!(cpu.registers.carry());
    }

    // --- Conditional control flow: Z, NC, C ---

    #[test]
    fn test_jp_z_not_taken() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_zero(false);
        bus.write(0x0000, 0xCA); // JP Z, nn
        bus.write(0x0001, 0x00);
        bus.write(0x0002, 0x10);
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.pc, 0x0003);
    }

    #[test]
    fn test_jp_nc_taken() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_carry(false);
        bus.write(0x0000, 0xD2); // JP NC, nn
        bus.write(0x0001, 0x00);
        bus.write(0x0002, 0x10);
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.pc, 0x1000);
    }

    #[test]
    fn test_jp_nc_not_taken() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_carry(true);
        bus.write(0x0000, 0xD2); // JP NC, nn
        bus.write(0x0001, 0x00);
        bus.write(0x0002, 0x10);
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.pc, 0x0003);
    }

    #[test]
    fn test_jp_c_taken() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_carry(true);
        bus.write(0x0000, 0xDA); // JP C, nn
        bus.write(0x0001, 0x00);
        bus.write(0x0002, 0x10);
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.pc, 0x1000);
    }

    #[test]
    fn test_jp_c_not_taken() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_carry(false);
        bus.write(0x0000, 0xDA); // JP C, nn
        bus.write(0x0001, 0x00);
        bus.write(0x0002, 0x10);
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.pc, 0x0003);
    }

    #[test]
    fn test_jr_z_taken() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_zero(true);
        bus.write(0x0000, 0x28); // JR Z, e
        bus.write(0x0001, 0x05);
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.pc, 0x0007);
    }

    #[test]
    fn test_jr_z_not_taken() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_zero(false);
        bus.write(0x0000, 0x28); // JR Z, e
        bus.write(0x0001, 0x05);
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.pc, 0x0002);
    }

    #[test]
    fn test_jr_nc_taken() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_carry(false);
        bus.write(0x0000, 0x30); // JR NC, e
        bus.write(0x0001, 0x04);
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.pc, 0x0006);
    }

    #[test]
    fn test_jr_c_taken() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_carry(true);
        bus.write(0x0000, 0x38); // JR C, e
        bus.write(0x0001, 0x04);
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.pc, 0x0006);
    }

    #[test]
    fn test_jr_c_not_taken() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_carry(false);
        bus.write(0x0000, 0x38); // JR C, e
        bus.write(0x0001, 0x04);
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.pc, 0x0002);
    }

    #[test]
    fn test_call_z_taken() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.sp = 0xFFFE;
        cpu.registers.set_zero(true);
        bus.write(0x0000, 0xCC); // CALL Z, nn
        bus.write(0x0001, 0x00);
        bus.write(0x0002, 0x05);
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.pc, 0x0500);
        assert_eq!(cpu.registers.sp, 0xFFFC);
    }

    #[test]
    fn test_call_z_not_taken() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.sp = 0xFFFE;
        cpu.registers.set_zero(false);
        bus.write(0x0000, 0xCC); // CALL Z, nn
        bus.write(0x0001, 0x00);
        bus.write(0x0002, 0x05);
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.pc, 0x0003);
        assert_eq!(cpu.registers.sp, 0xFFFE);
    }

    #[test]
    fn test_ret_z_taken() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.sp = 0xFFFC;
        cpu.registers.set_zero(true);
        bus.write(0xFFFC, 0x00);
        bus.write(0xFFFD, 0x05);
        bus.write(0x0000, 0xC8); // RET Z
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.pc, 0x0500);
        assert_eq!(cpu.registers.sp, 0xFFFE);
    }

    #[test]
    fn test_ret_z_not_taken() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.sp = 0xFFFC;
        cpu.registers.set_zero(false);
        bus.write(0x0000, 0xC8); // RET Z
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.pc, 0x0001);
        assert_eq!(cpu.registers.sp, 0xFFFC);
    }

    #[test]
    fn test_ret_nc_taken() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.sp = 0xFFFC;
        cpu.registers.set_carry(false);
        bus.write(0xFFFC, 0x34);
        bus.write(0xFFFD, 0x12);
        bus.write(0x0000, 0xD0); // RET NC
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.pc, 0x1234);
    }

    #[test]
    fn test_ret_nc_not_taken() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.sp = 0xFFFC;
        cpu.registers.set_carry(true);
        bus.write(0x0000, 0xD0); // RET NC
        let cycles = cpu.step(&mut bus);
        assert_eq!(cpu.registers.pc, 0x0001);
        assert_eq!(cpu.registers.sp, 0xFFFC);
        assert_eq!(cycles, 8);
    }

    #[test]
    fn test_ret_c_taken() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.sp = 0xFFFC;
        cpu.registers.set_carry(true);
        bus.write(0xFFFC, 0x34);
        bus.write(0xFFFD, 0x12);
        bus.write(0x0000, 0xD8); // RET C
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.pc, 0x1234);
    }

    #[test]
    fn test_ret_c_not_taken() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.sp = 0xFFFC;
        cpu.registers.set_carry(false);
        bus.write(0x0000, 0xD8); // RET C
        let cycles = cpu.step(&mut bus);
        assert_eq!(cpu.registers.pc, 0x0001);
        assert_eq!(cpu.registers.sp, 0xFFFC);
        assert_eq!(cycles, 8);
    }

    // --- CB prefix ---

    #[test]
    fn test_cb_rlc_b() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.b = 0b1000_0001;
        bus.write(0x0000, 0xCB);
        bus.write(0x0001, 0x00); // RLC B
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.b, 0b0000_0011);
        assert!(cpu.registers.carry());
        assert!(!cpu.registers.zero());
    }

    #[test]
    fn test_cb_rrc_c() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.c = 0b0000_0011;
        bus.write(0x0000, 0xCB);
        bus.write(0x0001, 0x09); // RRC C
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.c, 0b1000_0001);
        assert!(cpu.registers.carry());
    }

    #[test]
    fn test_cb_rl_d() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.d = 0b0100_0000;
        cpu.registers.set_carry(true);
        bus.write(0x0000, 0xCB);
        bus.write(0x0001, 0x12); // RL D
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.d, 0b1000_0001);
        assert!(!cpu.registers.carry());
    }

    #[test]
    fn test_cb_rr_e() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.e = 0b0000_0010;
        cpu.registers.set_carry(true);
        bus.write(0x0000, 0xCB);
        bus.write(0x0001, 0x1B); // RR E
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.e, 0b1000_0001);
        assert!(!cpu.registers.carry());
    }

    #[test]
    fn test_cb_sla_h() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.h = 0b1100_0000;
        bus.write(0x0000, 0xCB);
        bus.write(0x0001, 0x24); // SLA H
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.h, 0b1000_0000);
        assert!(cpu.registers.carry());
        assert!(!cpu.registers.zero());
    }

    #[test]
    fn test_cb_sra_l() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.l = 0b1000_0010;
        bus.write(0x0000, 0xCB);
        bus.write(0x0001, 0x2D); // SRA L
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.l, 0b1100_0001);
        assert!(!cpu.registers.carry());
    }

    #[test]
    fn test_cb_swap_a() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0xAB;
        bus.write(0x0000, 0xCB);
        bus.write(0x0001, 0x37); // SWAP A
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.a, 0xBA);
        assert!(!cpu.registers.zero());
    }

    #[test]
    fn test_cb_swap_zero() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.a = 0x00;
        bus.write(0x0000, 0xCB);
        bus.write(0x0001, 0x37); // SWAP A
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.a, 0x00);
        assert!(cpu.registers.zero());
    }

    #[test]
    fn test_cb_srl_b() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.b = 0b1000_0001;
        bus.write(0x0000, 0xCB);
        bus.write(0x0001, 0x38); // SRL B
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.b, 0b0100_0000);
        assert!(cpu.registers.carry());
    }

    #[test]
    fn test_cb_bit_set() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.b = 0b0000_0100;
        cpu.registers.set_carry(true);
        bus.write(0x0000, 0xCB);
        bus.write(0x0001, 0x50); // BIT 2, B
        cpu.step(&mut bus);
        assert!(!cpu.registers.zero()); // bit is set → Z=0
        assert!(cpu.registers.half_carry());
        assert!(!cpu.registers.subtract());
        assert!(cpu.registers.carry()); // BIT preserves carry
    }

    #[test]
    fn test_cb_bit_clear() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.b = 0b0000_0000;
        cpu.registers.set_carry(false);
        bus.write(0x0000, 0xCB);
        bus.write(0x0001, 0x50); // BIT 2, B
        cpu.step(&mut bus);
        assert!(cpu.registers.zero()); // bit is 0 → Z=1
        assert!(!cpu.registers.carry()); // BIT preserves carry
    }

    #[test]
    fn test_cb_res_bit() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.b = 0xFF;
        bus.write(0x0000, 0xCB);
        bus.write(0x0001, 0x80); // RES 0, B
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.b, 0xFE);
    }

    #[test]
    fn test_cb_set_bit() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.b = 0x00;
        bus.write(0x0000, 0xCB);
        bus.write(0x0001, 0xC0); // SET 0, B
        cpu.step(&mut bus);
        assert_eq!(cpu.registers.b, 0x01);
    }

    #[test]
    fn test_cb_hl_indirect() {
        // BIT via (HL) — verify it reads from memory
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.registers.set_hl(0x8000);
        bus.write(0x8000, 0b0000_1000);
        bus.write(0x0000, 0xCB);
        bus.write(0x0001, 0x5E); // BIT 3, (HL)
        cpu.step(&mut bus);
        assert!(!cpu.registers.zero()); // bit 3 is set
    }
}
