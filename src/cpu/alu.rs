//! Arithmetic, logic, and flag-manipulation helpers.

use crate::bus::Bus;

use super::{Cpu, Reg, RegPair};

impl Cpu {
    /// Execute accumulator-only rotates:
    /// - `RLCA` (`0x07`)
    /// - `RRCA` (`0x0F`)
    /// - `RLA` (`0x17`)
    /// - `RRA` (`0x1F`)
    ///
    /// These forms always clear `Z` on DMG.
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

    /// Execute `DAA` (`0x27`) for BCD correction after add/subtract.
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

    /// Execute `CPL` (`0x2F`): bitwise invert `A`, set `N` and `H`.
    pub(in crate::cpu) fn cpl(&mut self) {
        self.registers.a = !self.registers.a;
        self.registers.set_subtract(true);
        self.registers.set_half_carry(true);
    }

    /// Execute `SCF` (`0x37`): set carry flag and clear `N/H`.
    pub(in crate::cpu) fn scf(&mut self) {
        self.registers.set_subtract(false);
        self.registers.set_half_carry(false);
        self.registers.set_carry(true);
    }

    /// Execute `CCF` (`0x3F`): complement carry flag and clear `N/H`.
    pub(in crate::cpu) fn ccf(&mut self) {
        self.registers.set_subtract(false);
        self.registers.set_half_carry(false);
        self.registers.set_carry(!self.registers.carry());
    }

    /// Execute grouped `ALU A, r` operations (`0x80..=0xBF`).
    ///
    /// Decodes one of: `ADD/ADC/SUB/SBC/AND/XOR/OR/CP` based on opcode bits.
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

    /// Execute `ADD SP, e` (`0xE8`).
    ///
    /// Flags: `Z=0`, `N=0`, `H/C` from low-byte signed-add carry behavior.
    pub(in crate::cpu) fn add_sp_e(&mut self, bus: &Bus) {
        #[allow(clippy::cast_possible_wrap)]
        let offset = self.read_immediate_u8(bus).cast_signed();
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

    /// Execute `ADD A, value` shared implementation.
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

    /// Execute `INC rr` (`0x03/0x13/0x23/0x33`).
    ///
    /// Flags are unaffected.
    pub(in crate::cpu) fn inc_rr(&mut self, pair: RegPair) {
        let value = self.read_reg_pair(pair);
        self.write_reg_pair(pair, value.wrapping_add(1));
    }

    /// Execute `DEC rr` (`0x0B/0x1B/0x2B/0x3B`).
    ///
    /// Flags are unaffected.
    pub(in crate::cpu) fn dec_rr(&mut self, pair: RegPair) {
        let value = self.read_reg_pair(pair);
        self.write_reg_pair(pair, value.wrapping_sub(1));
    }

    /// Execute `ADD HL, rr` (`0x09/0x19/0x29/0x39`).
    ///
    /// Updates `N/H/C`; `Z` is preserved.
    pub(in crate::cpu) fn add_hl_rr(&mut self, pair: RegPair) {
        let hl = self.registers.hl();
        let value = self.read_reg_pair(pair);
        let result = hl.wrapping_add(value);
        self.registers.set_hl(result);
        self.registers.set_subtract(false);
        self.registers
            .set_half_carry((hl & 0x0FFF) + (value & 0x0FFF) > 0x0FFF);
        self.registers.set_carry(hl > 0xFFFF - value);
    }

    /// Execute `ADC A, value` shared implementation.
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

    /// Execute `SUB A, value` shared implementation.
    pub(in crate::cpu) fn sub_a(&mut self, value: u8) {
        let a = self.registers.a;
        let result = a.wrapping_sub(value);
        self.registers.a = result;
        self.registers.set_zero(result == 0);
        self.registers.set_subtract(true);
        self.registers.set_half_carry((a & 0x0F) < (value & 0x0F));
        self.registers.set_carry(a < value);
    }

    /// Execute `SBC A, value` shared implementation.
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

    /// Execute `AND A, value` shared implementation.
    pub(in crate::cpu) fn and_a(&mut self, value: u8) {
        let result = self.registers.a & value;
        self.registers.a = result;
        self.registers.set_zero(result == 0);
        self.registers.set_subtract(false);
        self.registers.set_half_carry(true);
        self.registers.set_carry(false);
    }

    /// Execute `XOR A, value` shared implementation.
    pub(in crate::cpu) fn xor_a(&mut self, value: u8) {
        let result = self.registers.a ^ value;
        self.registers.a = result;
        self.registers.set_zero(result == 0);
        self.registers.set_subtract(false);
        self.registers.set_half_carry(false);
        self.registers.set_carry(false);
    }

    /// Execute `OR A, value` shared implementation.
    pub(in crate::cpu) fn or_a(&mut self, value: u8) {
        let result = self.registers.a | value;
        self.registers.a = result;
        self.registers.set_zero(result == 0);
        self.registers.set_subtract(false);
        self.registers.set_half_carry(false);
        self.registers.set_carry(false);
    }

    /// Execute `CP A, value` shared implementation (compare without storing).
    pub(in crate::cpu) fn cp_a(&mut self, value: u8) {
        let a = self.registers.a;
        let result = a.wrapping_sub(value);
        self.registers.set_zero(result == 0);
        self.registers.set_subtract(true);
        self.registers.set_half_carry((a & 0x0F) < (value & 0x0F));
        self.registers.set_carry(a < value);
    }
}
