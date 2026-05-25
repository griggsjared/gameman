use crate::bus::Bus;

use super::Cpu;

impl Cpu {
    pub(in crate::cpu) fn stop(&mut self) {
        // Consumes the next byte (0x00) as a second opcode byte
        self.registers.pc = self.registers.pc.wrapping_add(1);
    }

    pub(in crate::cpu) fn jp_hl(&mut self) {
        self.registers.pc = self.registers.hl();
    }

    pub(in crate::cpu) fn jp_nn(&mut self, bus: &Bus) {
        self.registers.pc = self.imm16(bus);
    }

    pub(in crate::cpu) fn jp_cc_nn(&mut self, opcode: u8, bus: &Bus) -> u8 {
        let cc = (opcode >> 3) & 0b11;
        let address = self.imm16(bus);
        if self.check_condition(cc) {
            self.registers.pc = address;
            16
        } else {
            12
        }
    }

    pub(in crate::cpu) fn jr_d(&mut self, bus: &Bus) {
        #[allow(clippy::cast_possible_wrap)]
        let offset = self.imm8(bus).cast_signed();
        #[allow(clippy::cast_sign_loss)]
        let target = self.registers.pc.wrapping_add(offset as u16);
        self.registers.pc = target;
    }

    pub(in crate::cpu) fn jr_cc_d(&mut self, opcode: u8, bus: &Bus) -> u8 {
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

    pub(in crate::cpu) fn call_nn(&mut self, bus: &mut Bus) {
        let address = self.imm16(bus);
        self.push16(self.registers.pc, bus);
        self.registers.pc = address;
    }

    pub(in crate::cpu) fn call_cc_nn(&mut self, opcode: u8, bus: &mut Bus) -> u8 {
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

    pub(in crate::cpu) fn ret(&mut self, bus: &Bus) {
        self.registers.pc = self.pop16(bus);
    }

    pub(in crate::cpu) fn ret_cc(&mut self, opcode: u8, bus: &Bus) -> u8 {
        let cc = (opcode >> 3) & 0b11;
        if self.check_condition(cc) {
            self.registers.pc = self.pop16(bus);
            20
        } else {
            8
        }
    }

    pub(in crate::cpu) fn reti(&mut self, bus: &Bus) {
        self.registers.pc = self.pop16(bus);
        self.ime = true;
        self.ei_delay = 0;
    }

    pub(in crate::cpu) fn rst(&mut self, opcode: u8, bus: &mut Bus) {
        let address = u16::from(opcode & 0b0011_1000);
        self.push16(self.registers.pc, bus);
        self.registers.pc = address;
    }

    pub(in crate::cpu) fn push_rr(&mut self, pair: u8, bus: &mut Bus) {
        let value = match pair {
            0 => self.registers.bc(),
            1 => self.registers.de(),
            2 => self.registers.hl(),
            3 => self.registers.af(),
            _ => unreachable!(),
        };
        self.push16(value, bus);
    }

    pub(in crate::cpu) fn pop_rr(&mut self, pair: u8, bus: &Bus) {
        let value = self.pop16(bus);
        match pair {
            0 => self.registers.set_bc(value),
            1 => self.registers.set_de(value),
            2 => self.registers.set_hl(value),
            3 => self.registers.set_af(value),
            _ => unreachable!(),
        }
    }

    pub(in crate::cpu) fn push16(&mut self, value: u16, bus: &mut Bus) {
        self.registers.sp = self.registers.sp.wrapping_sub(1);
        #[allow(clippy::cast_possible_truncation)]
        bus.write(self.registers.sp, (value >> 8) as u8);
        self.registers.sp = self.registers.sp.wrapping_sub(1);
        #[allow(clippy::cast_possible_truncation)]
        bus.write(self.registers.sp, value as u8);
    }

    #[must_use]
    pub(in crate::cpu) fn pop16(&mut self, bus: &Bus) -> u16 {
        let low = bus.read(self.registers.sp);
        self.registers.sp = self.registers.sp.wrapping_add(1);
        let high = bus.read(self.registers.sp);
        self.registers.sp = self.registers.sp.wrapping_add(1);
        u16::from(high) << 8 | u16::from(low)
    }

    #[must_use]
    pub(in crate::cpu) fn check_condition(&self, cc: u8) -> bool {
        match cc {
            0 => !self.registers.zero(),  // NZ
            1 => self.registers.zero(),   // Z
            2 => !self.registers.carry(), // NC
            3 => self.registers.carry(),  // C
            _ => unreachable!(),
        }
    }
}
