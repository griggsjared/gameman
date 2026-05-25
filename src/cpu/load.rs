//! Load/store and register access helpers.
//!
//! This module implements data-movement opcodes and shared register decoding
//! helpers used by multiple opcode families.

use crate::bus::Bus;

use super::{Cpu, Reg, RegPair};

impl Cpu {
    /// Execute `LD A, (BC)` (`0x0A`).
    pub(in crate::cpu) fn ld_a_from_bc(&mut self, bus: &Bus) {
        self.registers.a = bus.read(self.registers.bc());
    }

    /// Execute `LD A, (DE)` (`0x1A`).
    pub(in crate::cpu) fn ld_a_from_de(&mut self, bus: &Bus) {
        self.registers.a = bus.read(self.registers.de());
    }

    /// Execute `LD (BC), A` (`0x02`).
    pub(in crate::cpu) fn ld_bc_from_a(&mut self, bus: &mut Bus) {
        bus.write(self.registers.bc(), self.registers.a);
    }

    /// Execute `LD (DE), A` (`0x12`).
    pub(in crate::cpu) fn ld_de_from_a(&mut self, bus: &mut Bus) {
        bus.write(self.registers.de(), self.registers.a);
    }

    /// Execute `LD (HL+), A` (`0x22`) and post-increment `HL`.
    pub(in crate::cpu) fn ld_hli_a(&mut self, bus: &mut Bus) {
        bus.write(self.registers.hl(), self.registers.a);
        self.registers.set_hl(self.registers.hl().wrapping_add(1));
    }

    /// Execute `LD (HL-), A` (`0x32`) and post-decrement `HL`.
    pub(in crate::cpu) fn ld_hld_a(&mut self, bus: &mut Bus) {
        bus.write(self.registers.hl(), self.registers.a);
        self.registers.set_hl(self.registers.hl().wrapping_sub(1));
    }

    /// Execute `LD A, (HL+)` (`0x2A`) and post-increment `HL`.
    pub(in crate::cpu) fn ld_a_hli(&mut self, bus: &Bus) {
        self.registers.a = bus.read(self.registers.hl());
        self.registers.set_hl(self.registers.hl().wrapping_add(1));
    }

    /// Execute `LD A, (HL-)` (`0x3A`) and post-decrement `HL`.
    pub(in crate::cpu) fn ld_a_hld(&mut self, bus: &Bus) {
        self.registers.a = bus.read(self.registers.hl());
        self.registers.set_hl(self.registers.hl().wrapping_sub(1));
    }

    /// Execute `LD (HL), n` (`0x36`).
    pub(in crate::cpu) fn ld_hl_n(&mut self, bus: &mut Bus) {
        let value = self.read_immediate_u8(bus);
        bus.write(self.registers.hl(), value);
    }

    /// Execute `LD r, r'` (`0x40..=0x7F`, excluding `0x76`).
    ///
    /// Returns `8` cycles when either operand is `(HL)`, else `4`.
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

    /// Execute `INC r` (`0x04/0x0C/.../0x3C`).
    ///
    /// Updates `Z`, clears `N`, sets `H` on low-nibble overflow.
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

    /// Execute `DEC r` (`0x05/0x0D/.../0x3D`).
    ///
    /// Updates `Z`, sets `N`, sets `H` on low-nibble borrow.
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

    /// Execute `LD (nn), A` (`0xEA`).
    pub(in crate::cpu) fn ld_nn_a(&mut self, bus: &mut Bus) {
        let address = self.read_immediate_u16(bus);
        bus.write(address, self.registers.a);
    }

    /// Execute `LD A, (nn)` (`0xFA`).
    pub(in crate::cpu) fn ld_a_nn(&mut self, bus: &Bus) {
        let address = self.read_immediate_u16(bus);
        self.registers.a = bus.read(address);
    }

    /// Execute `LD SP, HL` (`0xF9`).
    pub(in crate::cpu) fn ld_sp_hl(&mut self) {
        self.registers.sp = self.registers.hl();
    }

    /// Execute `LD (nn), SP` (`0x08`) in little-endian order.
    pub(in crate::cpu) fn ld_nn_sp(&mut self, bus: &mut Bus) {
        let address = self.read_immediate_u16(bus);
        let sp = self.registers.sp;
        #[allow(clippy::cast_possible_truncation)]
        bus.write(address, sp as u8);
        #[allow(clippy::cast_possible_truncation)]
        bus.write(address.wrapping_add(1), (sp >> 8) as u8);
    }

    /// Execute `LDH (n), A` (`0xE0`) using address `0xFF00 + n`.
    pub(in crate::cpu) fn ldh_n_a(&mut self, bus: &mut Bus) {
        let offset = self.read_immediate_u8(bus);
        bus.write(0xFF00 | u16::from(offset), self.registers.a);
    }

    /// Execute `LDH A, (n)` (`0xF0`) using address `0xFF00 + n`.
    pub(in crate::cpu) fn ldh_a_n(&mut self, bus: &Bus) {
        let offset = self.read_immediate_u8(bus);
        self.registers.a = bus.read(0xFF00 | u16::from(offset));
    }

    /// Execute `LDH (C), A` (`0xE2`) using address `0xFF00 + C`.
    pub(in crate::cpu) fn ldh_c_a(&mut self, bus: &mut Bus) {
        bus.write(0xFF00 | u16::from(self.registers.c), self.registers.a);
    }

    /// Execute `LDH A, (C)` (`0xF2`) using address `0xFF00 + C`.
    pub(in crate::cpu) fn ldh_a_c(&mut self, bus: &Bus) {
        self.registers.a = bus.read(0xFF00 | u16::from(self.registers.c));
    }

    /// Execute `LD HL, SP+e` (`0xF8`).
    ///
    /// Flags: `Z=0`, `N=0`, `H/C` from low-byte signed-add carry behavior.
    pub(in crate::cpu) fn ld_hl_sp_plus_e(&mut self, bus: &Bus) {
        #[allow(clippy::cast_possible_wrap)]
        let offset = self.read_immediate_u8(bus).cast_signed();
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

    /// Read an 8-bit operand selected by `Reg`.
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

    /// Write an 8-bit operand selected by `Reg`.
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

    /// Read a 16-bit register pair (`BC/DE/HL/SP`).
    #[must_use]
    pub(in crate::cpu) fn read_reg_pair(&self, pair: RegPair) -> u16 {
        match pair {
            RegPair::Bc => self.registers.bc(),
            RegPair::De => self.registers.de(),
            RegPair::Hl => self.registers.hl(),
            RegPair::Sp => self.registers.sp,
        }
    }

    /// Write a 16-bit register pair (`BC/DE/HL/SP`).
    pub(in crate::cpu) fn write_reg_pair(&mut self, pair: RegPair, value: u16) {
        match pair {
            RegPair::Bc => self.registers.set_bc(value),
            RegPair::De => self.registers.set_de(value),
            RegPair::Hl => self.registers.set_hl(value),
            RegPair::Sp => self.registers.sp = value,
        }
    }
}
