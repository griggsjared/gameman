use crate::bus::Bus;

use super::{Cpu, Reg};

impl Cpu {
    pub(in crate::cpu) fn ld_a_from_bc(&mut self, bus: &Bus) {
        self.registers.a = bus.read(self.registers.bc());
    }

    pub(in crate::cpu) fn ld_a_from_de(&mut self, bus: &Bus) {
        self.registers.a = bus.read(self.registers.de());
    }

    pub(in crate::cpu) fn ld_bc_from_a(&mut self, bus: &mut Bus) {
        bus.write(self.registers.bc(), self.registers.a);
    }

    pub(in crate::cpu) fn ld_de_from_a(&mut self, bus: &mut Bus) {
        bus.write(self.registers.de(), self.registers.a);
    }

    pub(in crate::cpu) fn ld_hli_a(&mut self, bus: &mut Bus) {
        bus.write(self.registers.hl(), self.registers.a);
        self.registers.set_hl(self.registers.hl().wrapping_add(1));
    }

    pub(in crate::cpu) fn ld_hld_a(&mut self, bus: &mut Bus) {
        bus.write(self.registers.hl(), self.registers.a);
        self.registers.set_hl(self.registers.hl().wrapping_sub(1));
    }

    pub(in crate::cpu) fn ld_a_hli(&mut self, bus: &Bus) {
        self.registers.a = bus.read(self.registers.hl());
        self.registers.set_hl(self.registers.hl().wrapping_add(1));
    }

    pub(in crate::cpu) fn ld_a_hld(&mut self, bus: &Bus) {
        self.registers.a = bus.read(self.registers.hl());
        self.registers.set_hl(self.registers.hl().wrapping_sub(1));
    }

    pub(in crate::cpu) fn ld_hl_n(&mut self, bus: &mut Bus) {
        let value = self.imm8(bus);
        bus.write(self.registers.hl(), value);
    }

    pub(in crate::cpu) fn ld_r_r(&mut self, opcode: u8, bus: &mut Bus) -> u8 {
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

    pub(in crate::cpu) fn inc_r(&mut self, opcode: u8, bus: &mut Bus) -> u8 {
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

    pub(in crate::cpu) fn dec_r(&mut self, opcode: u8, bus: &mut Bus) -> u8 {
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

    pub(in crate::cpu) fn ld_nn_a(&mut self, bus: &mut Bus) {
        let address = self.imm16(bus);
        bus.write(address, self.registers.a);
    }

    pub(in crate::cpu) fn ld_a_nn(&mut self, bus: &Bus) {
        let address = self.imm16(bus);
        self.registers.a = bus.read(address);
    }

    pub(in crate::cpu) fn ld_sp_hl(&mut self) {
        self.registers.sp = self.registers.hl();
    }

    pub(in crate::cpu) fn ld_nn_sp(&mut self, bus: &mut Bus) {
        let address = self.imm16(bus);
        let sp = self.registers.sp;
        #[allow(clippy::cast_possible_truncation)]
        bus.write(address, sp as u8);
        #[allow(clippy::cast_possible_truncation)]
        bus.write(address.wrapping_add(1), (sp >> 8) as u8);
    }

    pub(in crate::cpu) fn ldh_n_a(&mut self, bus: &mut Bus) {
        let offset = self.imm8(bus);
        bus.write(0xFF00 | u16::from(offset), self.registers.a);
    }

    pub(in crate::cpu) fn ldh_a_n(&mut self, bus: &Bus) {
        let offset = self.imm8(bus);
        self.registers.a = bus.read(0xFF00 | u16::from(offset));
    }

    pub(in crate::cpu) fn ldh_c_a(&mut self, bus: &mut Bus) {
        bus.write(0xFF00 | u16::from(self.registers.c), self.registers.a);
    }

    pub(in crate::cpu) fn ldh_a_c(&mut self, bus: &Bus) {
        self.registers.a = bus.read(0xFF00 | u16::from(self.registers.c));
    }

    pub(in crate::cpu) fn ld_hl_sp_plus_e(&mut self, bus: &Bus) {
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
    }

    #[must_use]
    pub(in crate::cpu) fn read_register(&self, reg: Reg, bus: &Bus) -> u8 {
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

    pub(in crate::cpu) fn write_register(&mut self, reg: Reg, value: u8, bus: &mut Bus) {
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
    pub(in crate::cpu) fn read_rr(&self, pair: u8) -> u16 {
        match pair {
            0 => self.registers.bc(),
            1 => self.registers.de(),
            2 => self.registers.hl(),
            3 => self.registers.sp,
            _ => unreachable!(),
        }
    }

    pub(in crate::cpu) fn write_rr(&mut self, pair: u8, value: u16) {
        match pair {
            0 => self.registers.set_bc(value),
            1 => self.registers.set_de(value),
            2 => self.registers.set_hl(value),
            3 => self.registers.sp = value,
            _ => unreachable!(),
        }
    }
}
