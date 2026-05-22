mod registers;
pub use registers::Registers;

#[derive(Debug, Default)]
pub struct Cpu {
    pub registers: Registers,
}

impl Cpu {
    pub fn new() -> Self {
        Cpu {
            registers: Registers::new(),
        }
    }

    /// Reset the CPU to initial state
    pub fn reset(&mut self) {
        self.registers = Registers::new();
        // Game Boy starts execution at 0x0100
        self.registers.pc = 0x0100;
        // Initial stack pointer
        self.registers.sp = 0xFFFE;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_new() {
        let cpu = Cpu::new();
        assert_eq!(cpu.registers.pc, 0);
        assert_eq!(cpu.registers.sp, 0);
    }

    #[test]
    fn test_cpu_reset() {
        let mut cpu = Cpu::new();
        cpu.registers.a = 0xFF;
        cpu.registers.pc = 0x5555;

        cpu.reset();

        assert_eq!(cpu.registers.a, 0);
        assert_eq!(cpu.registers.pc, 0x0100);
        assert_eq!(cpu.registers.sp, 0xFFFE);
    }
}
