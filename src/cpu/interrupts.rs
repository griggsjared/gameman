//! Interrupt-control, HALT behavior, and interrupt dispatch helpers.

use crate::bus::Bus;

use super::Cpu;

impl Cpu {
    /// Execute `DI` (`0xF3`): disable interrupts immediately.
    pub(in crate::cpu) fn di(&mut self) {
        self.ime = false;
        self.ei_delay = 0;
    }

    /// Execute `EI` (`0xFB`): schedule IME enable after next instruction.
    pub(in crate::cpu) fn ei(&mut self) {
        // EI enables IME after the *next* instruction.
        self.ei_delay = 2;
    }

    /// Execute `HALT` (`0x76`) and model HALT-bug entry conditions.
    pub(in crate::cpu) fn halt(&mut self, bus: &Bus) {
        let pending = bus.pending_interrupts();
        if !self.ime && pending != 0 {
            self.halted = false;
            self.halt_bug = true;
        } else {
            self.halted = true;
            self.halt_bug = false;
        }
    }

    /// Apply delayed EI completion after each retired instruction.
    pub(in crate::cpu) fn apply_ei_delay_after_instruction(&mut self) {
        if self.ei_delay > 0 {
            self.ei_delay -= 1;
            if self.ei_delay == 0 {
                self.ime = true;
            }
        }
    }

    /// Service the highest-priority pending interrupt.
    ///
    /// Side effects:
    /// - Clears the acknowledged IF bit.
    /// - Disables IME and clears transient low-power/bug states.
    /// - Pushes current `PC` and jumps to vector.
    /// - Returns fixed 20-cycle ISR entry cost.
    pub(in crate::cpu) fn service_interrupt(&mut self, bus: &mut Bus, pending: u8) -> u8 {
        let index = highest_priority_interrupt_index(pending);
        let mask = 1u8 << index;
        bus.clear_interrupt(mask);

        self.ime = false;
        self.ei_delay = 0;
        self.halted = false;
        self.stopped = false;
        self.stop_joypad_latch = false;
        self.halt_bug = false;

        self.push16(self.registers.pc, bus);
        self.registers.pc = interrupt_vector(index);

        20
    }
}

const fn interrupt_vector(index: u8) -> u16 {
    match index {
        0 => 0x40, // VBlank
        1 => 0x48, // LCD STAT
        2 => 0x50, // Timer
        3 => 0x58, // Serial
        4 => 0x60, // Joypad
        _ => unreachable!(),
    }
}

const fn highest_priority_interrupt_index(mask: u8) -> u8 {
    if mask & 0b0000_0001 != 0 {
        0
    } else if mask & 0b0000_0010 != 0 {
        1
    } else if mask & 0b0000_0100 != 0 {
        2
    } else if mask & 0b0000_1000 != 0 {
        3
    } else if mask & 0b0001_0000 != 0 {
        4
    } else {
        unreachable!()
    }
}
