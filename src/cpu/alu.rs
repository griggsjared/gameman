use crate::bus::Bus;

use super::{Cpu, Reg};

impl Cpu {
    pub(in crate::cpu) fn rotate_accumulator(&mut self, opcode: u8) {
        match opcode {
            0x07 => {
                // RLCA
                let a = self.registers.a;
                let carry = a >> 7;
                self.registers.a = (a << 1) | carry;
                self.registers.set_zero(false);
                self.registers.set_subtract(false);
                self.registers.set_half_carry(false);
                self.registers.set_carry(carry != 0);
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
            }
            _ => unreachable!(),
        }
    }

    pub(in crate::cpu) fn daa(&mut self) {
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
    }

    pub(in crate::cpu) fn cpl(&mut self) {
        self.registers.a = !self.registers.a;
        self.registers.set_subtract(true);
        self.registers.set_half_carry(true);
    }

    pub(in crate::cpu) fn scf(&mut self) {
        self.registers.set_subtract(false);
        self.registers.set_half_carry(false);
        self.registers.set_carry(true);
    }

    pub(in crate::cpu) fn ccf(&mut self) {
        self.registers.set_subtract(false);
        self.registers.set_half_carry(false);
        self.registers.set_carry(!self.registers.carry());
    }

    pub(in crate::cpu) fn alu_a_r(&mut self, opcode: u8, bus: &Bus) -> u8 {
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

    pub(in crate::cpu) fn add_sp_e(&mut self, bus: &Bus) {
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
    }

    pub(in crate::cpu) fn add_a(&mut self, value: u8) {
        let a = self.registers.a;
        let (result, carry) = a.overflowing_add(value);
        self.registers.a = result;
        self.registers.set_zero(result == 0);
        self.registers.set_subtract(false);
        self.registers
            .set_half_carry((a & 0x0F) + (value & 0x0F) > 0x0F);
        self.registers.set_carry(carry);
    }

    pub(in crate::cpu) fn inc_rr(&mut self, pair: u8) {
        let value = self.read_rr(pair);
        self.write_rr(pair, value.wrapping_add(1));
    }

    pub(in crate::cpu) fn dec_rr(&mut self, pair: u8) {
        let value = self.read_rr(pair);
        self.write_rr(pair, value.wrapping_sub(1));
    }

    pub(in crate::cpu) fn add_hl_rr(&mut self, pair: u8) {
        let hl = self.registers.hl();
        let value = self.read_rr(pair);
        let result = hl.wrapping_add(value);
        self.registers.set_hl(result);
        self.registers.set_subtract(false);
        self.registers
            .set_half_carry((hl & 0x0FFF) + (value & 0x0FFF) > 0x0FFF);
        self.registers.set_carry(hl > 0xFFFF - value);
    }

    pub(in crate::cpu) fn adc_a(&mut self, value: u8) {
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

    pub(in crate::cpu) fn sub_a(&mut self, value: u8) {
        let a = self.registers.a;
        let result = a.wrapping_sub(value);
        self.registers.a = result;
        self.registers.set_zero(result == 0);
        self.registers.set_subtract(true);
        self.registers.set_half_carry((a & 0x0F) < (value & 0x0F));
        self.registers.set_carry(a < value);
    }

    pub(in crate::cpu) fn sbc_a(&mut self, value: u8) {
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

    pub(in crate::cpu) fn and_a(&mut self, value: u8) {
        let result = self.registers.a & value;
        self.registers.a = result;
        self.registers.set_zero(result == 0);
        self.registers.set_subtract(false);
        self.registers.set_half_carry(true);
        self.registers.set_carry(false);
    }

    pub(in crate::cpu) fn xor_a(&mut self, value: u8) {
        let result = self.registers.a ^ value;
        self.registers.a = result;
        self.registers.set_zero(result == 0);
        self.registers.set_subtract(false);
        self.registers.set_half_carry(false);
        self.registers.set_carry(false);
    }

    pub(in crate::cpu) fn or_a(&mut self, value: u8) {
        let result = self.registers.a | value;
        self.registers.a = result;
        self.registers.set_zero(result == 0);
        self.registers.set_subtract(false);
        self.registers.set_half_carry(false);
        self.registers.set_carry(false);
    }

    pub(in crate::cpu) fn cp_a(&mut self, value: u8) {
        let a = self.registers.a;
        let result = a.wrapping_sub(value);
        self.registers.set_zero(result == 0);
        self.registers.set_subtract(true);
        self.registers.set_half_carry((a & 0x0F) < (value & 0x0F));
        self.registers.set_carry(a < value);
    }
}
