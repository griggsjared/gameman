use crate::bus::Bus;

use super::Cpu;

impl Cpu {
    pub(in crate::cpu) fn di(&mut self) {
        self.ime = false;
        self.ei_delay = 0;
    }

    pub(in crate::cpu) fn ei(&mut self) {
        // EI enables IME after the *next* instruction.
        self.ei_delay = 2;
    }

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

    pub(in crate::cpu) fn apply_ei_delay_after_instruction(&mut self) {
        if self.ei_delay > 0 {
            self.ei_delay -= 1;
            if self.ei_delay == 0 {
                self.ime = true;
            }
        }
    }

    pub(in crate::cpu) fn service_interrupt(&mut self, bus: &mut Bus, pending: u8) -> u8 {
        let index = highest_priority_interrupt_index(pending);
        let mask = 1u8 << index;
        bus.clear_interrupt(mask);

        self.ime = false;
        self.ei_delay = 0;
        self.halted = false;

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
