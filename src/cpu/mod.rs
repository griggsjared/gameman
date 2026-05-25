use crate::bus::Bus;

mod alu;
mod cb;
mod control;
mod interrupts;
mod load;
mod registers;
#[cfg(test)]
mod tests;

pub use registers::Registers;

/// Core DMG CPU state and execution pipeline.
#[derive(Debug, Default)]
pub struct Cpu {
    pub registers: Registers,
    ime: bool,
    ei_delay: u8,
    halted: bool,
    stopped: bool,
    stop_joypad_latch: bool,
    halt_bug: bool,
}

/// 3-bit register encoding used in opcode fields.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

/// 2-bit condition encoding used by conditional control-flow opcodes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::cpu) enum Condition {
    Nz = 0,
    Z,
    Nc,
    C,
}

impl Condition {
    #[must_use]
    const fn from_u2(code: u8) -> Self {
        match code & 0b11 {
            0 => Self::Nz,
            1 => Self::Z,
            2 => Self::Nc,
            3 => Self::C,
            _ => unreachable!(),
        }
    }
}

/// 2-bit register-pair encoding used by `rr` operands (`BC`, `DE`, `HL`, `SP`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::cpu) enum RegPair {
    Bc = 0,
    De,
    Hl,
    Sp,
}

impl RegPair {
    #[must_use]
    const fn from_u2(code: u8) -> Self {
        match code & 0b11 {
            0 => Self::Bc,
            1 => Self::De,
            2 => Self::Hl,
            3 => Self::Sp,
            _ => unreachable!(),
        }
    }
}

/// 2-bit stack pair encoding used by PUSH/POP (`BC`, `DE`, `HL`, `AF`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::cpu) enum StackPair {
    Bc = 0,
    De,
    Hl,
    Af,
}

impl StackPair {
    #[must_use]
    const fn from_u2(code: u8) -> Self {
        match code & 0b11 {
            0 => Self::Bc,
            1 => Self::De,
            2 => Self::Hl,
            3 => Self::Af,
            _ => unreachable!(),
        }
    }
}

impl Cpu {
    #[must_use]
    pub fn new() -> Self {
        Cpu {
            registers: Registers::new(),
            ime: false,
            ei_delay: 0,
            halted: false,
            stopped: false,
            stop_joypad_latch: false,
            halt_bug: false,
        }
    }

    /// Reset the CPU to initial state
    pub fn reset(&mut self) {
        self.registers = Registers::new();
        self.ime = false;
        self.ei_delay = 0;
        self.halted = false;
        self.stopped = false;
        self.stop_joypad_latch = false;
        self.halt_bug = false;
        // Game Boy starts execution at 0x0100
        self.registers.pc = 0x0100;
        // Initial stack pointer
        self.registers.sp = 0xFFFE;
    }

    /// Execute one CPU step and return consumed machine cycles.
    ///
    /// Flow:
    /// 1. Sample interrupt state and current joypad interrupt request.
    /// 2. If STOP is active, wake only on a fresh joypad request edge.
    /// 3. Execute an active cycle (IRQ service, HALT idle, or instruction).
    /// 4. Tick timers only when the CPU performed active work.
    pub fn step(&mut self, bus: &mut Bus) -> u8 {
        let pending = bus.pending_interrupts();

        let joypad_requested = bus.joypad_interrupt_requested();

        let (cycles, should_tick_timers) = if self.stopped {
            let should_wake = joypad_requested && !self.stop_joypad_latch;
            self.stop_joypad_latch = joypad_requested;

            if should_wake {
                self.stopped = false;
                (self.run_active_cycle(bus, pending), true)
            } else {
                (4, false)
            }
        } else {
            self.stop_joypad_latch = joypad_requested;
            (self.run_active_cycle(bus, pending), true)
        };

        if should_tick_timers {
            bus.tick_timers(cycles);
        }
        cycles
    }

    /// Execute one non-STOP CPU cycle.
    ///
    /// Priority order:
    /// - HALT idle behavior and wake conditions
    /// - Interrupt service when `IME` is enabled
    /// - Normal instruction execution
    fn run_active_cycle(&mut self, bus: &mut Bus, pending: u8) -> u8 {
        if self.halted {
            if pending == 0 {
                4
            } else {
                self.halted = false;

                if self.ime {
                    self.service_interrupt(bus, pending)
                } else {
                    self.execute_next_instruction(bus)
                }
            }
        } else if self.ime && pending != 0 {
            self.service_interrupt(bus, pending)
        } else {
            self.execute_next_instruction(bus)
        }
    }

    /// Fetch, execute, and retire one opcode at the current `PC`.
    ///
    /// Handles HALT-bug fetch behavior and EI delayed enable semantics.
    fn execute_next_instruction(&mut self, bus: &mut Bus) -> u8 {
        let opcode = bus.read(self.registers.pc);
        if self.halt_bug {
            self.halt_bug = false;
        } else {
            self.registers.pc = self.registers.pc.wrapping_add(1);
        }
        let cycles = self.execute(opcode, bus);
        self.apply_ei_delay_after_instruction();
        cycles
    }

    /// Decode and execute one fetched opcode.
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
    ///
    /// Dispatch is organized by opcode family helpers:
    /// - power/interrupt control (`NOP`, `DI`, `EI`, `HALT`, `STOP`)
    /// - load/data-move
    /// - arithmetic/logic/flag ops
    /// - control-flow + stack
    /// - `CB` prefix
    fn execute(&mut self, opcode: u8, bus: &mut Bus) -> u8 {
        if let Some(cycles) = self.execute_power_interrupt_opcode(opcode, bus) {
            return cycles;
        }

        if let Some(cycles) = self.execute_load_opcode(opcode, bus) {
            return cycles;
        }

        if let Some(cycles) = self.execute_alu_opcode(opcode, bus) {
            return cycles;
        }

        if let Some(cycles) = self.execute_control_opcode(opcode, bus) {
            return cycles;
        }

        if let Some(cycles) = self.execute_prefixed_opcode(opcode, bus) {
            return cycles;
        }

        panic!(
            "Unimplemented opcode: 0x{opcode:02X} at PC={:04X}",
            self.registers.pc.wrapping_sub(1)
        )
    }

    /// Handle core power and interrupt-control opcodes with fixed timing.
    fn execute_power_interrupt_opcode(&mut self, opcode: u8, bus: &mut Bus) -> Option<u8> {
        let cycles = match opcode {
            0x00 => 4, // NOP

            0xF3 => {
                self.di();
                4
            }

            0xFB => {
                self.ei();
                4
            }

            0x10 => {
                self.stop();
                4
            }

            0x76 => {
                self.halt(bus);
                4
            }

            _ => return None,
        };

        Some(cycles)
    }

    /// Handle all load/data-movement opcodes (`LD*`, `LDH*`).
    fn execute_load_opcode(&mut self, opcode: u8, bus: &mut Bus) -> Option<u8> {
        let cycles = match opcode {
            0x01 | 0x11 | 0x21 | 0x31 => {
                let pair = RegPair::from_u2(opcode >> 4);
                let value = self.read_immediate_u16(bus);
                self.write_reg_pair(pair, value);
                12
            }

            0x06 | 0x0E | 0x16 | 0x1E | 0x26 | 0x2E | 0x3E => {
                let reg = Reg::from_u3(opcode >> 3);
                let value = self.read_immediate_u8(bus);
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

            0x40..=0x75 | 0x77..=0x7F => self.ld_r_r(opcode, bus),

            0xEA => {
                self.ld_nn_a(bus);
                16
            }

            0xFA => {
                self.ld_a_nn(bus);
                16
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

            _ => return None,
        };

        Some(cycles)
    }

    /// Handle arithmetic, logical, and flag-manipulation opcodes.
    fn execute_alu_opcode(&mut self, opcode: u8, bus: &mut Bus) -> Option<u8> {
        let cycles = match opcode {
            0x03 | 0x13 | 0x23 | 0x33 => {
                let pair = RegPair::from_u2(opcode >> 4);
                self.inc_rr(pair);
                8
            }

            0x0B | 0x1B | 0x2B | 0x3B => {
                let pair = RegPair::from_u2(opcode >> 4);
                self.dec_rr(pair);
                8
            }

            0x09 | 0x19 | 0x29 | 0x39 => {
                let pair = RegPair::from_u2(opcode >> 4);
                self.add_hl_rr(pair);
                8
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

            0x04 | 0x0C | 0x14 | 0x1C | 0x24 | 0x2C | 0x34 | 0x3C => self.inc_r(opcode, bus),

            0x05 | 0x0D | 0x15 | 0x1D | 0x25 | 0x2D | 0x35 | 0x3D => self.dec_r(opcode, bus),

            0x80..=0xBF => self.alu_a_r(opcode, bus),

            0xC6 => {
                let value = self.read_immediate_u8(bus);
                self.add_a(value);
                8
            }

            0xCE => {
                let value = self.read_immediate_u8(bus);
                self.adc_a(value);
                8
            }

            0xD6 => {
                let value = self.read_immediate_u8(bus);
                self.sub_a(value);
                8
            }

            0xDE => {
                let value = self.read_immediate_u8(bus);
                self.sbc_a(value);
                8
            }

            0xE6 => {
                let value = self.read_immediate_u8(bus);
                self.and_a(value);
                8
            }

            0xEE => {
                let value = self.read_immediate_u8(bus);
                self.xor_a(value);
                8
            }

            0xF6 => {
                let value = self.read_immediate_u8(bus);
                self.or_a(value);
                8
            }

            0xFE => {
                let value = self.read_immediate_u8(bus);
                self.cp_a(value);
                8
            }

            0xE8 => {
                self.add_sp_e(bus);
                16
            }

            _ => return None,
        };

        Some(cycles)
    }

    /// Handle control-flow and stack opcodes (`JP/JR/CALL/RET/RST`, `PUSH/POP`).
    fn execute_control_opcode(&mut self, opcode: u8, bus: &mut Bus) -> Option<u8> {
        let cycles = match opcode {
            0xC1 | 0xD1 | 0xE1 | 0xF1 => {
                let pair = StackPair::from_u2(opcode >> 4);
                self.pop_rr(pair, bus);
                12
            }

            0xC5 | 0xD5 | 0xE5 | 0xF5 => {
                let pair = StackPair::from_u2(opcode >> 4);
                self.push_rr(pair, bus);
                16
            }

            0xE9 => {
                self.jp_hl();
                4
            }

            0xC3 => {
                self.jp_nn(bus);
                16
            }

            0xC2 | 0xCA | 0xD2 | 0xDA => {
                let condition = Condition::from_u2(opcode >> 3);
                self.jp_cc_nn(condition, bus)
            }

            0x18 => {
                self.jr_d(bus);
                12
            }

            0x20 | 0x28 | 0x30 | 0x38 => {
                let condition = Condition::from_u2(opcode >> 3);
                self.jr_cc_d(condition, bus)
            }

            0xCD => {
                self.call_nn(bus);
                24
            }

            0xC4 | 0xCC | 0xD4 | 0xDC => {
                let condition = Condition::from_u2(opcode >> 3);
                self.call_cc_nn(condition, bus)
            }

            0xC9 => {
                self.ret(bus);
                16
            }

            0xC0 | 0xC8 | 0xD0 | 0xD8 => {
                let condition = Condition::from_u2(opcode >> 3);
                self.ret_cc(condition, bus)
            }

            0xD9 => {
                self.reti(bus);
                16
            }

            0xC7 | 0xCF | 0xD7 | 0xDF | 0xE7 | 0xEF | 0xF7 | 0xFF => {
                self.rst(opcode, bus);
                16
            }

            _ => return None,
        };

        Some(cycles)
    }

    /// Handle prefixed opcode pages (`0xCB xx`).
    fn execute_prefixed_opcode(&mut self, opcode: u8, bus: &mut Bus) -> Option<u8> {
        if opcode != 0xCB {
            return None;
        }

        let cb_opcode = self.read_immediate_u8(bus);
        Some(self.execute_cb(cb_opcode, bus))
    }

    /// Read an immediate byte from `PC` and advance `PC` by 1.
    #[must_use]
    fn read_immediate_u8(&mut self, bus: &Bus) -> u8 {
        let value = bus.read(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);
        value
    }

    /// Read a 16-bit immediate from `PC` and advance `PC` by 2 (little-endian).
    #[must_use]
    fn read_immediate_u16(&mut self, bus: &Bus) -> u16 {
        let low = bus.read(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);
        let high = bus.read(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);
        u16::from(high) << 8 | u16::from(low)
    }
}
