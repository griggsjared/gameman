//! Control-flow and stack instruction helpers.
//!
//! This module implements the opcode mnemonics in the `JP/JR/CALL/RET/RST`
//! and `PUSH/POP` families, plus shared condition evaluation.

use crate::bus::Bus;

use super::{Condition, Cpu, StackPair};

impl Cpu {
    /// Execute `STOP` (opcode `0x10`).
    ///
    /// The DMG CPU consumes the following byte and enters STOP state.
    pub(in crate::cpu) fn stop(&mut self) {
        // STOP consumes the next byte as a second opcode byte.
        self.registers.pc = self.registers.pc.wrapping_add(1);
        self.halted = false;
        self.halt_bug = false;
        self.stopped = true;
    }

    /// Execute `JP HL` (`0xE9`): jump to address in HL.
    pub(in crate::cpu) fn jp_hl(&mut self) {
        self.registers.pc = self.registers.hl();
    }

    /// Execute `JP nn` (`0xC3`): unconditional absolute jump.
    pub(in crate::cpu) fn jp_nn(&mut self, bus: &Bus) {
        self.registers.pc = self.read_immediate_u16(bus);
    }

    /// Execute `JP cc, nn` (`0xC2/0xCA/0xD2/0xDA`).
    ///
    /// Returns `16` cycles when taken, otherwise `12` cycles.
    pub(in crate::cpu) fn jp_cc_nn(&mut self, condition: Condition, bus: &Bus) -> u8 {
        let address = self.read_immediate_u16(bus);
        if self.check_condition(condition) {
            self.registers.pc = address;
            16
        } else {
            12
        }
    }

    /// Execute `JR d` (`0x18`): relative jump by signed immediate offset.
    pub(in crate::cpu) fn jr_d(&mut self, bus: &Bus) {
        #[allow(clippy::cast_possible_wrap)]
        let offset = self.read_immediate_u8(bus).cast_signed();
        #[allow(clippy::cast_sign_loss)]
        let target = self.registers.pc.wrapping_add(offset as u16);
        self.registers.pc = target;
    }

    /// Execute `JR cc, d` (`0x20/0x28/0x30/0x38`).
    ///
    /// Returns `12` cycles when taken, otherwise `8` cycles.
    pub(in crate::cpu) fn jr_cc_d(&mut self, condition: Condition, bus: &Bus) -> u8 {
        #[allow(clippy::cast_possible_wrap)]
        let offset = self.read_immediate_u8(bus).cast_signed();
        if self.check_condition(condition) {
            #[allow(clippy::cast_sign_loss)]
            let target = self.registers.pc.wrapping_add(offset as u16);
            self.registers.pc = target;
            12
        } else {
            8
        }
    }

    /// Execute `CALL nn` (`0xCD`): push return address, then jump.
    pub(in crate::cpu) fn call_nn(&mut self, bus: &mut Bus) {
        let address = self.read_immediate_u16(bus);
        self.push16(self.registers.pc, bus);
        self.registers.pc = address;
    }

    /// Execute `CALL cc, nn` (`0xC4/0xCC/0xD4/0xDC`).
    ///
    /// Returns `24` cycles when taken, otherwise `12` cycles.
    pub(in crate::cpu) fn call_cc_nn(&mut self, condition: Condition, bus: &mut Bus) -> u8 {
        let address = self.read_immediate_u16(bus);
        if self.check_condition(condition) {
            self.push16(self.registers.pc, bus);
            self.registers.pc = address;
            24
        } else {
            12
        }
    }

    /// Execute `RET` (`0xC9`): pop `PC` from the stack.
    pub(in crate::cpu) fn ret(&mut self, bus: &Bus) {
        self.registers.pc = self.pop16(bus);
    }

    /// Execute `RET cc` (`0xC0/0xC8/0xD0/0xD8`).
    ///
    /// Returns `20` cycles when taken, otherwise `8` cycles.
    pub(in crate::cpu) fn ret_cc(&mut self, condition: Condition, bus: &Bus) -> u8 {
        if self.check_condition(condition) {
            self.registers.pc = self.pop16(bus);
            20
        } else {
            8
        }
    }

    /// Execute `RETI` (`0xD9`): return from ISR and re-enable IME immediately.
    pub(in crate::cpu) fn reti(&mut self, bus: &Bus) {
        self.registers.pc = self.pop16(bus);
        self.ime = true;
        self.ei_delay = 0;
    }

    /// Execute `RST n` (`0xC7..=0xFF` with stride 8): call fixed vector.
    pub(in crate::cpu) fn rst(&mut self, opcode: u8, bus: &mut Bus) {
        let address = u16::from(opcode & 0b0011_1000);
        self.push16(self.registers.pc, bus);
        self.registers.pc = address;
    }

    /// Execute `PUSH rr` (`0xC5/0xD5/0xE5/0xF5`).
    pub(in crate::cpu) fn push_rr(&mut self, pair: StackPair, bus: &mut Bus) {
        let value = match pair {
            StackPair::Bc => self.registers.bc(),
            StackPair::De => self.registers.de(),
            StackPair::Hl => self.registers.hl(),
            StackPair::Af => self.registers.af(),
        };
        self.push16(value, bus);
    }

    /// Execute `POP rr` (`0xC1/0xD1/0xE1/0xF1`).
    pub(in crate::cpu) fn pop_rr(&mut self, pair: StackPair, bus: &Bus) {
        let value = self.pop16(bus);
        match pair {
            StackPair::Bc => self.registers.set_bc(value),
            StackPair::De => self.registers.set_de(value),
            StackPair::Hl => self.registers.set_hl(value),
            StackPair::Af => self.registers.set_af(value),
        }
    }

    /// Push a 16-bit value onto the stack in Game Boy byte order.
    pub(in crate::cpu) fn push16(&mut self, value: u16, bus: &mut Bus) {
        self.registers.sp = self.registers.sp.wrapping_sub(1);
        #[allow(clippy::cast_possible_truncation)]
        bus.write(self.registers.sp, (value >> 8) as u8);
        self.registers.sp = self.registers.sp.wrapping_sub(1);
        #[allow(clippy::cast_possible_truncation)]
        bus.write(self.registers.sp, value as u8);
    }

    /// Pop a 16-bit value from the stack in Game Boy byte order.
    #[must_use]
    pub(in crate::cpu) fn pop16(&mut self, bus: &Bus) -> u16 {
        let low = bus.read(self.registers.sp);
        self.registers.sp = self.registers.sp.wrapping_add(1);
        let high = bus.read(self.registers.sp);
        self.registers.sp = self.registers.sp.wrapping_add(1);
        u16::from(high) << 8 | u16::from(low)
    }

    /// Evaluate a decoded branch condition (`NZ/Z/NC/C`).
    #[must_use]
    pub(in crate::cpu) fn check_condition(&self, condition: Condition) -> bool {
        match condition {
            Condition::Nz => !self.registers.zero(),
            Condition::Z => self.registers.zero(),
            Condition::Nc => !self.registers.carry(),
            Condition::C => self.registers.carry(),
        }
    }
}
