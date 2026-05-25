use crate::bus::Bus;

mod alu;
mod cb;
mod control;
mod load;
mod registers;
#[cfg(test)]
mod tests;

pub use registers::Registers;

#[derive(Debug, Default)]
pub struct Cpu {
    pub registers: Registers,
}

/// 3-bit register encoding used in opcode fields.
#[derive(Clone, Copy)]
pub(in crate::cpu) enum Reg {
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

            0x0A => {
                self.ld_a_from_bc(bus);
                8
            }
            0x1A => {
                self.ld_a_from_de(bus);
                8
            }
            0x02 => {
                self.ld_bc_from_a(bus);
                8
            }
            0x12 => {
                self.ld_de_from_a(bus);
                8
            }
            0x22 => {
                self.ld_hli_a(bus);
                8
            }
            0x32 => {
                self.ld_hld_a(bus);
                8
            }
            0x2A => {
                self.ld_a_hli(bus);
                8
            }
            0x3A => {
                self.ld_a_hld(bus);
                8
            }

            0x36 => {
                self.ld_hl_n(bus);
                12
            }

            0x07 | 0x0F | 0x17 | 0x1F => {
                self.rotate_accumulator(opcode);
                4
            }

            0x27 => {
                self.daa();
                4
            }

            0x2F => {
                self.cpl();
                4
            }

            0x37 => {
                self.scf();
                4
            }

            0x3F => {
                self.ccf();
                4
            }

            0x10 => {
                self.stop();
                4
            }

            0x76 => panic!("Unimplemented opcode: 0x76"), // HALT

            0x40..=0x7F => self.ld_r_r(opcode, bus),

            0x04 | 0x0C | 0x14 | 0x1C | 0x24 | 0x2C | 0x34 | 0x3C => self.inc_r(opcode, bus),

            0x05 | 0x0D | 0x15 | 0x1D | 0x25 | 0x2D | 0x35 | 0x3D => self.dec_r(opcode, bus),

            0x80..=0xBF => self.alu_a_r(opcode, bus),

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
                self.ld_nn_a(bus);
                16
            }

            0xFA => {
                self.ld_a_nn(bus);
                16
            }

            0xE9 => {
                self.jp_hl();
                4
            }

            0xF9 => {
                self.ld_sp_hl();
                8
            }

            0x08 => {
                self.ld_nn_sp(bus);
                20
            }

            0xE0 => {
                self.ldh_n_a(bus);
                12
            }

            0xF0 => {
                self.ldh_a_n(bus);
                12
            }

            0xE2 => {
                self.ldh_c_a(bus);
                8
            }

            0xF2 => {
                self.ldh_a_c(bus);
                8
            }

            0xF8 => {
                self.ld_hl_sp_plus_e(bus);
                12
            }

            0xE8 => {
                self.add_sp_e(bus);
                16
            }

            0xC3 => {
                self.jp_nn(bus);
                16
            }

            0xC2 | 0xCA | 0xD2 | 0xDA => self.jp_cc_nn(opcode, bus),

            0x18 => {
                self.jr_d(bus);
                12
            }

            0x20 | 0x28 | 0x30 | 0x38 => self.jr_cc_d(opcode, bus),

            0xCD => {
                self.call_nn(bus);
                24
            }

            0xC4 | 0xCC | 0xD4 | 0xDC => self.call_cc_nn(opcode, bus),

            0xC9 => {
                self.ret(bus);
                16
            }

            0xC0 | 0xC8 | 0xD0 | 0xD8 => self.ret_cc(opcode, bus),

            0xD9 => {
                self.reti(bus);
                16
            }

            0xC7 | 0xCF | 0xD7 | 0xDF | 0xE7 | 0xEF | 0xF7 | 0xFF => {
                self.rst(opcode, bus);
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

    /// Read an immediate byte from `PC` and advance `PC` by 1.
    #[must_use]
    fn imm8(&mut self, bus: &Bus) -> u8 {
        let value = bus.read(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);
        value
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
}
