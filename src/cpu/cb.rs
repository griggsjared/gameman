use crate::bus::Bus;

use super::{Cpu, Reg};

impl Cpu {
    pub(in crate::cpu) fn execute_cb(&mut self, opcode: u8, bus: &mut Bus) -> u8 {
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
}
