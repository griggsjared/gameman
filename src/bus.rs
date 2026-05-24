/// Flat 64KB memory buffer backing the Game Boy address space.
///
/// Uses an inline `[u8; 0x10000]` for zero-indirection O(1) access.
///
/// The array is created on the stack inside `new()`. For most use cases
/// (including tests) this is fine, but if stack pressure becomes an issue
/// the field can be changed to `Box<[u8; 0x10000]>` with minimal churn.
#[derive(Debug)]
pub struct Bus {
    memory: [u8; 0x10000],
}

impl Default for Bus {
    fn default() -> Self {
        Self::new()
    }
}

impl Bus {
    /// Create a new bus with all bytes initialised to zero.
    #[must_use]
    #[allow(clippy::large_stack_arrays)]
    pub fn new() -> Self {
        Bus {
            memory: [0; 0x10000],
        }
    }

    /// Read a byte from the 64KB address space.
    #[must_use]
    pub fn read(&self, address: u16) -> u8 {
        self.memory[usize::from(address)]
    }

    /// Write a byte into the 64KB address space.
    pub fn write(&mut self, address: u16, value: u8) {
        self.memory[usize::from(address)] = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bus_read_write() {
        let mut bus = Bus::new();
        bus.write(0x1234, 0xAB);
        assert_eq!(bus.read(0x1234), 0xAB);
    }

    #[test]
    fn test_bus_initially_zero() {
        let bus = Bus::new();
        assert_eq!(bus.read(0x0000), 0x00);
        assert_eq!(bus.read(0xFFFF), 0x00);
    }
}
