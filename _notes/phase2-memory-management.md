# Phase 2: Memory Management

In Phase 1, you built the CPU and manually fed it instructions. In Phase 2, we'll create the Memory Management Unit (MMU) so your CPU can read actual ROM data and access memory like real hardware.

## Table of Contents

1. [Gameboy Memory Map](#gameboy-memory-map)
2. [MMU Architecture](#mmu-architecture)
3. [Basic Memory Implementation](#basic-memory-implementation)
4. [ROM Loading](#rom-loading)
5. [Memory-Mapped I/O](#memory-mapped-io)
6. [Banking Basics](#banking-basics)
7. [CPU-MMU Integration](#cpu-mmu-integration)
8. [Milestones](#milestones)

---

## Gameboy Memory Map

The Gameboy has a 16-bit address bus, giving it a 64KB address space. Different ranges map to different hardware:

```
Memory Map (64KB total):

0x0000-0x3FFF : ROM Bank 0 (16KB) - Always mapped
0x4000-0x7FFF : ROM Bank 1-N (16KB) - Switchable via MBC
0x8000-0x9FFF : VRAM (8KB) - Video RAM
0xA000-0xBFFF : External RAM (8KB) - Cartridge RAM (if present)
0xC000-0xCFFF : WRAM Bank 0 (4KB) - Work RAM
0xD000-0xDFFF : WRAM Bank 1 (4KB) - Work RAM
0xE000-0xFDFF : Echo RAM (mirror of 0xC000-0xDDFF)
0xFE00-0xFE9F : OAM (160 bytes) - Sprite attribute table
0xFEA0-0xFEFF : Unusable
0xFF00-0xFF7F : I/O Registers (128 bytes)
0xFF80-0xFFFE : HRAM (127 bytes) - High RAM (fast)
0xFFFF        : IE Register - Interrupt Enable
```

### Key Memory Regions

**ROM (0x0000-0x7FFF)**
- Contains game code and data
- Read-only
- Bank 0 always accessible
- Bank 1+ can be switched (for games > 32KB)

**VRAM (0x8000-0x9FFF)**
- Holds tile data and tilemaps
- Accessed by PPU during rendering
- CPU can't access during certain PPU modes

**WRAM (0xC000-0xDFFF)**
- General purpose RAM
- Fast, always accessible
- 8KB total

**I/O Registers (0xFF00-0xFF7F)**
- Hardware control registers
- Joypad, timers, sound, LCD control, etc.

**HRAM (0xFF80-0xFFFE)**
- Super fast RAM
- Often used for interrupt handlers
- 127 bytes

---

## MMU Architecture

### Design Goal

The MMU should:
1. Provide `read(addr) -> u8` and `write(addr, value)` interfaces
2. Route addresses to appropriate hardware
3. Handle banking (switching ROM/RAM banks)
4. Enforce access restrictions (read-only ROM, PPU conflicts, etc.)

### Design Decision: Trait vs Struct

**Option 1: Struct-based MMU**

```rust
pub struct Mmu {
    rom: Vec<u8>,
    vram: [u8; 0x2000],  // 8KB
    wram: [u8; 0x2000],  // 8KB
    hram: [u8; 0x7F],    // 127 bytes
    // ... etc
}

impl Mmu {
    pub fn read(&self, addr: u16) -> u8 { /* ... */ }
    pub fn write(&mut self, addr: u16, value: u8) { /* ... */ }
}
```

**Pros:**
- Simple, direct
- Easy to understand
- Fast (no vtable overhead)

**Cons:**
- Harder to test in isolation
- CPU tightly coupled to MMU implementation

**Option 2: Trait-based Memory Bus**

```rust
pub trait MemoryBus {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, value: u8);
}

pub struct Mmu {
    // ... fields
}

impl MemoryBus for Mmu {
    fn read(&self, addr: u16) -> u8 { /* ... */ }
    fn write(&mut self, addr: u16, value: u8) { /* ... */ }
}
```

**Pros:**
- CPU can work with any MemoryBus implementation
- Easy to create mock/test implementations
- Better abstraction

**Cons:**
- Slightly more complex
- Trait object overhead (if using dynamic dispatch)

**Recommendation:** Use **Option 1 (Struct)** for simplicity. The abstraction isn't needed for a single emulator. You can always refactor later.

---

## Basic Memory Implementation

Let's build the MMU step by step using TDD.

### Step 1: Create MMU Structure

**File: `src/mmu/mod.rs`**

```rust
pub struct Mmu {
    rom: Vec<u8>,              // ROM data (up to multiple banks)
    vram: [u8; 0x2000],        // 8KB Video RAM
    wram: [u8; 0x2000],        // 8KB Work RAM
    hram: [u8; 0x7F],          // 127 bytes High RAM
    io_registers: [u8; 0x80],  // 128 bytes I/O registers
    ie_register: u8,           // Interrupt Enable register (0xFFFF)
}

impl Mmu {
    pub fn new() -> Self {
        Mmu {
            rom: Vec::new(),
            vram: [0; 0x2000],
            wram: [0; 0x2000],
            hram: [0; 0x7F],
            io_registers: [0; 0x80],
            ie_register: 0,
        }
    }
    
    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            // ROM (0x0000-0x7FFF)
            0x0000..=0x7FFF => self.read_rom(addr),
            
            // VRAM (0x8000-0x9FFF)
            0x8000..=0x9FFF => self.vram[(addr - 0x8000) as usize],
            
            // External RAM (0xA000-0xBFFF) - not implemented yet
            0xA000..=0xBFFF => 0xFF,  // Return 0xFF for unmapped
            
            // WRAM (0xC000-0xDFFF)
            0xC000..=0xDFFF => self.wram[(addr - 0xC000) as usize],
            
            // Echo RAM (0xE000-0xFDFF) - mirror of WRAM
            0xE000..=0xFDFF => self.wram[(addr - 0xE000) as usize],
            
            // OAM (0xFE00-0xFE9F) - not implemented yet
            0xFE00..=0xFE9F => 0xFF,
            
            // Unusable (0xFEA0-0xFEFF)
            0xFEA0..=0xFEFF => 0xFF,
            
            // I/O Registers (0xFF00-0xFF7F)
            0xFF00..=0xFF7F => self.io_registers[(addr - 0xFF00) as usize],
            
            // HRAM (0xFF80-0xFFFE)
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize],
            
            // IE Register (0xFFFF)
            0xFFFF => self.ie_register,
        }
    }
    
    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            // ROM (0x0000-0x7FFF) - read-only, but writes can trigger banking
            0x0000..=0x7FFF => {
                // TODO: Handle MBC (Memory Bank Controller) writes
                // For now, ignore writes to ROM
            }
            
            // VRAM (0x8000-0x9FFF)
            0x8000..=0x9FFF => self.vram[(addr - 0x8000) as usize] = value,
            
            // External RAM (0xA000-0xBFFF)
            0xA000..=0xBFFF => {
                // TODO: Implement cartridge RAM
            }
            
            // WRAM (0xC000-0xDFFF)
            0xC000..=0xDFFF => self.wram[(addr - 0xC000) as usize] = value,
            
            // Echo RAM (0xE000-0xFDFF)
            0xE000..=0xFDFF => self.wram[(addr - 0xE000) as usize] = value,
            
            // OAM (0xFE00-0xFE9F)
            0xFE00..=0xFE9F => {
                // TODO: Implement OAM
            }
            
            // Unusable (0xFEA0-0xFEFF)
            0xFEA0..=0xFEFF => {
                // Ignore writes
            }
            
            // I/O Registers (0xFF00-0xFF7F)
            0xFF00..=0xFF7F => self.io_registers[(addr - 0xFF00) as usize] = value,
            
            // HRAM (0xFF80-0xFFFE)
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize] = value,
            
            // IE Register (0xFFFF)
            0xFFFF => self.ie_register = value,
        }
    }
    
    fn read_rom(&self, addr: u16) -> u8 {
        if (addr as usize) < self.rom.len() {
            self.rom[addr as usize]
        } else {
            0xFF  // Return 0xFF if ROM not loaded
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mmu_new() {
        let mmu = Mmu::new();
        // WRAM should be initialized to 0
        assert_eq!(mmu.read(0xC000), 0);
    }
    
    #[test]
    fn test_wram_read_write() {
        let mut mmu = Mmu::new();
        
        mmu.write(0xC000, 0x42);
        assert_eq!(mmu.read(0xC000), 0x42);
        
        mmu.write(0xDFFF, 0xAB);
        assert_eq!(mmu.read(0xDFFF), 0xAB);
    }
    
    #[test]
    fn test_echo_ram_mirrors_wram() {
        let mut mmu = Mmu::new();
        
        // Write to WRAM
        mmu.write(0xC100, 0x55);
        
        // Read from Echo RAM (same offset)
        assert_eq!(mmu.read(0xE100), 0x55);
        
        // Write to Echo RAM
        mmu.write(0xE200, 0x66);
        
        // Read from WRAM
        assert_eq!(mmu.read(0xC200), 0x66);
    }
    
    #[test]
    fn test_hram_read_write() {
        let mut mmu = Mmu::new();
        
        mmu.write(0xFF80, 0x11);
        assert_eq!(mmu.read(0xFF80), 0x11);
        
        mmu.write(0xFFFE, 0x22);
        assert_eq!(mmu.read(0xFFFE), 0x22);
    }
    
    #[test]
    fn test_ie_register() {
        let mut mmu = Mmu::new();
        
        mmu.write(0xFFFF, 0b00011111);
        assert_eq!(mmu.read(0xFFFF), 0b00011111);
    }
    
    #[test]
    fn test_vram_read_write() {
        let mut mmu = Mmu::new();
        
        mmu.write(0x8000, 0xAB);
        assert_eq!(mmu.read(0x8000), 0xAB);
        
        mmu.write(0x9FFF, 0xCD);
        assert_eq!(mmu.read(0x9FFF), 0xCD);
    }
    
    #[test]
    fn test_unmapped_memory_returns_ff() {
        let mmu = Mmu::new();
        
        // Unusable region
        assert_eq!(mmu.read(0xFEA0), 0xFF);
        assert_eq!(mmu.read(0xFEFF), 0xFF);
    }
}
```

**TDD Checkpoint:** Run `cargo test` - all MMU tests should pass!

---

## ROM Loading

Now let's add the ability to load ROM files.

### Understanding ROM Format

Gameboy ROMs are binary files containing:
1. Entry point at 0x0100
2. Nintendo logo at 0x0104-0x0133 (for authentication)
3. Title at 0x0134-0x0143
4. ROM size indicator at 0x0148
5. Cartridge type at 0x0147 (MBC type)

For Phase 2, we'll just load the raw data. We'll handle cartridge types in Phase 3.

### Implementation

```rust
// Add to src/mmu/mod.rs

use std::fs;
use std::io;
use std::path::Path;

impl Mmu {
    pub fn load_rom<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        self.rom = fs::read(path)?;
        
        // Basic validation
        if self.rom.len() < 0x150 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "ROM too small (minimum 0x150 bytes)"
            ));
        }
        
        Ok(())
    }
    
    pub fn load_rom_from_bytes(&mut self, data: Vec<u8>) {
        self.rom = data;
    }
    
    /// Get ROM title from header (0x0134-0x0143)
    pub fn get_rom_title(&self) -> String {
        if self.rom.len() < 0x0143 {
            return String::from("Unknown");
        }
        
        let title_bytes = &self.rom[0x0134..=0x0143];
        String::from_utf8_lossy(title_bytes)
            .trim_end_matches('\0')
            .to_string()
    }
    
    /// Get ROM size from header (0x0148)
    pub fn get_rom_size(&self) -> usize {
        if self.rom.len() < 0x0148 {
            return 0;
        }
        
        let size_code = self.rom[0x0148];
        // ROM size = 32KB << size_code
        32 * 1024 * (1 << size_code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_load_rom_from_bytes() {
        let mut mmu = Mmu::new();
        let rom_data = vec![0x00; 0x8000];  // 32KB of zeros
        
        mmu.load_rom_from_bytes(rom_data);
        
        assert_eq!(mmu.read(0x0000), 0x00);
    }
    
    #[test]
    fn test_read_rom_data() {
        let mut mmu = Mmu::new();
        let mut rom_data = vec![0x00; 0x8000];
        rom_data[0x0100] = 0xC3;  // JP instruction
        rom_data[0x0101] = 0x50;
        rom_data[0x0102] = 0x01;
        
        mmu.load_rom_from_bytes(rom_data);
        
        assert_eq!(mmu.read(0x0100), 0xC3);
        assert_eq!(mmu.read(0x0101), 0x50);
        assert_eq!(mmu.read(0x0102), 0x01);
    }
    
    #[test]
    fn test_rom_title() {
        let mut mmu = Mmu::new();
        let mut rom_data = vec![0x00; 0x8000];
        
        // Set title at 0x0134
        let title = b"TETRIS\0\0\0\0\0\0\0\0\0\0";
        rom_data[0x0134..=0x0143].copy_from_slice(title);
        
        mmu.load_rom_from_bytes(rom_data);
        
        assert_eq!(mmu.get_rom_title(), "TETRIS");
    }
    
    #[test]
    fn test_rom_cannot_be_written() {
        let mut mmu = Mmu::new();
        let rom_data = vec![0xAA; 0x8000];
        mmu.load_rom_from_bytes(rom_data);
        
        // Try to write to ROM
        mmu.write(0x0100, 0xFF);
        
        // ROM should be unchanged
        assert_eq!(mmu.read(0x0100), 0xAA);
    }
}
```

**TDD Checkpoint:** Run `cargo test` - ROM loading tests should pass!

---

## Memory-Mapped I/O

I/O registers control hardware. We'll implement a few key ones for testing.

### Common I/O Registers

```
0xFF00: P1   - Joypad
0xFF01: SB   - Serial transfer data
0xFF02: SC   - Serial transfer control
0xFF04: DIV  - Divider register (increments at 16384 Hz)
0xFF05: TIMA - Timer counter
0xFF06: TMA  - Timer modulo
0xFF07: TAC  - Timer control
0xFF0F: IF   - Interrupt flags
0xFF40: LCDC - LCD control
0xFF41: STAT - LCD status
0xFF42: SCY  - Scroll Y
0xFF43: SCX  - Scroll X
0xFF44: LY   - LCD Y coordinate (current scanline)
0xFF45: LYC  - LY compare
```

For now, our generic `io_registers` array handles basic reads/writes. Later phases will add special behavior (e.g., DIV auto-increments).

### Special Case: DIV Register (0xFF04)

The DIV register is special - it auto-increments and resets to 0 when written.

```rust
// Add special handling in read/write methods

impl Mmu {
    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            // ... other cases ...
            
            0xFF04 => {
                // Writing any value to DIV resets it to 0
                self.io_registers[0x04] = 0;
            }
            
            0xFF00..=0xFF7F => {
                self.io_registers[(addr - 0xFF00) as usize] = value;
            }
            
            // ... rest ...
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_div_register_resets_on_write() {
        let mut mmu = Mmu::new();
        
        // Set DIV to some value
        mmu.io_registers[0x04] = 0x42;
        assert_eq!(mmu.read(0xFF04), 0x42);
        
        // Write any value
        mmu.write(0xFF04, 0xFF);
        
        // DIV should be reset to 0
        assert_eq!(mmu.read(0xFF04), 0x00);
    }
}
```

---

## Banking Basics

Most Gameboy games are larger than 32KB. To access more ROM, they use **Memory Bank Controllers (MBCs)**.

### How Banking Works

```
Address Space:
0x0000-0x3FFF: ROM Bank 0 (always visible)
0x4000-0x7FFF: ROM Bank N (switchable)

Physical ROM:
Bank 0: 0x0000-0x3FFF
Bank 1: 0x4000-0x7FFF
Bank 2: 0x8000-0xBFFF
Bank 3: 0xC000-0xFFFF
...
```

To switch banks, games write to ROM address space (which doesn't actually write to ROM, but controls the MBC).

### MBC1 (Most Common)

**Bank Switching:**
- Write to 0x2000-0x3FFF: Select ROM bank (lower 5 bits)
- Write to 0x4000-0x5FFF: Select upper bits or RAM bank
- Write to 0x6000-0x7FFF: Select banking mode

### Basic Implementation

For Phase 2, we'll implement simple ROM banking.

```rust
// Add to Mmu struct
pub struct Mmu {
    rom: Vec<u8>,
    vram: [u8; 0x2000],
    wram: [u8; 0x2000],
    hram: [u8; 0x7F],
    io_registers: [u8; 0x80],
    ie_register: u8,
    
    // Banking
    rom_bank: usize,  // Current ROM bank (for 0x4000-0x7FFF)
}

impl Mmu {
    pub fn new() -> Self {
        Mmu {
            rom: Vec::new(),
            vram: [0; 0x2000],
            wram: [0; 0x2000],
            hram: [0; 0x7F],
            io_registers: [0; 0x80],
            ie_register: 0,
            rom_bank: 1,  // Start at bank 1 (bank 0 is always at 0x0000)
        }
    }
    
    fn read_rom(&self, addr: u16) -> u8 {
        match addr {
            // Bank 0 (0x0000-0x3FFF)
            0x0000..=0x3FFF => {
                if (addr as usize) < self.rom.len() {
                    self.rom[addr as usize]
                } else {
                    0xFF
                }
            }
            
            // Switchable bank (0x4000-0x7FFF)
            0x4000..=0x7FFF => {
                let offset = (addr - 0x4000) as usize;
                let rom_addr = self.rom_bank * 0x4000 + offset;
                
                if rom_addr < self.rom.len() {
                    self.rom[rom_addr]
                } else {
                    0xFF
                }
            }
            
            _ => 0xFF,
        }
    }
    
    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            // MBC1 bank switching (simplified)
            0x2000..=0x3FFF => {
                // Select ROM bank
                let bank = (value & 0x1F) as usize;
                // Bank 0 acts as bank 1
                self.rom_bank = if bank == 0 { 1 } else { bank };
            }
            
            // ... rest of write implementation ...
            
            _ => { /* ... */ }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_rom_bank_0_always_visible() {
        let mut mmu = Mmu::new();
        let mut rom = vec![0x00; 0x8000];  // 2 banks
        rom[0x0000] = 0xAA;  // Bank 0
        rom[0x4000] = 0xBB;  // Bank 1
        
        mmu.load_rom_from_bytes(rom);
        
        // Bank 0 always visible at 0x0000-0x3FFF
        assert_eq!(mmu.read(0x0000), 0xAA);
    }
    
    #[test]
    fn test_rom_bank_switching() {
        let mut mmu = Mmu::new();
        let mut rom = vec![0x00; 0xC000];  // 3 banks
        rom[0x4000] = 0x11;  // Bank 1
        rom[0x8000] = 0x22;  // Bank 2
        
        mmu.load_rom_from_bytes(rom);
        
        // Default bank 1
        assert_eq!(mmu.read(0x4000), 0x11);
        
        // Switch to bank 2
        mmu.write(0x2000, 0x02);
        assert_eq!(mmu.read(0x4000), 0x22);
    }
    
    #[test]
    fn test_rom_bank_0_redirects_to_1() {
        let mut mmu = Mmu::new();
        let mut rom = vec![0x00; 0x8000];
        rom[0x4000] = 0xAA;  // Bank 1
        
        mmu.load_rom_from_bytes(rom);
        
        // Try to select bank 0 (should redirect to bank 1)
        mmu.write(0x2000, 0x00);
        assert_eq!(mmu.read(0x4000), 0xAA);
    }
}
```

**Note:** This is a simplified MBC1 implementation. Full MBC1 has more features (RAM banking, mode switching). We'll complete it in Phase 3.

---

## CPU-MMU Integration

Now let's connect the CPU to the MMU so it can fetch real instructions.

### Update CPU to Use MMU

**File: `src/cpu/mod.rs`**

```rust
use crate::mmu::Mmu;

pub struct Cpu {
    pub registers: Registers,
    pub mmu: Mmu,  // Add MMU
}

impl Cpu {
    pub fn new() -> Self {
        Cpu {
            registers: Registers::new(),
            mmu: Mmu::new(),
        }
    }
    
    /// Fetch the next byte from memory and advance PC
    fn fetch_byte(&mut self) -> u8 {
        let byte = self.mmu.read(self.registers.pc);
        self.registers.pc += 1;
        byte
    }
    
    /// Execute one instruction
    pub fn step(&mut self) -> u8 {
        let opcode = self.fetch_byte();
        self.execute(opcode)
    }
    
    pub fn execute(&mut self, opcode: u8) -> u8 {
        match opcode {
            0x00 => {
                // NOP - PC already advanced by fetch_byte
                4
            }
            
            0x3E => {
                // LD A, n
                let value = self.fetch_byte();
                self.registers.a = value;
                8
            }
            
            0x06 => {
                // LD B, n
                let value = self.fetch_byte();
                self.registers.b = value;
                8
            }
            
            // ... other opcodes ...
            
            _ => panic!("Unimplemented opcode: 0x{:02X} at PC: 0x{:04X}", 
                        opcode, self.registers.pc - 1),
        }
    }
}
```

### Update main.rs

```rust
// File: src/main.rs
mod cpu;
mod mmu;

fn main() {
    let mut cpu = cpu::Cpu::new();
    
    // Create a simple test program
    let test_rom = vec![
        0x3E, 0x42,  // LD A, 0x42
        0x06, 0x10,  // LD B, 0x10
        0x80,        // ADD A, B
        0x00,        // NOP
    ];
    
    cpu.mmu.load_rom_from_bytes(test_rom);
    cpu.registers.pc = 0x0000;
    
    println!("Starting execution...");
    
    // Execute 4 instructions
    for i in 0..4 {
        let cycles = cpu.step();
        println!("Step {}: PC=0x{:04X}, A=0x{:02X}, B=0x{:02X}, Cycles={}",
                 i + 1, cpu.registers.pc, cpu.registers.a, cpu.registers.b, cycles);
    }
    
    println!("\nFinal state:");
    println!("A = 0x{:02X} (should be 0x52)", cpu.registers.a);
    println!("B = 0x{:02X} (should be 0x10)", cpu.registers.b);
}
```

Run with `cargo run` and you should see:

```
Starting execution...
Step 1: PC=0x0002, A=0x42, B=0x00, Cycles=8
Step 2: PC=0x0004, A=0x42, B=0x10, Cycles=8
Step 3: PC=0x0005, A=0x52, B=0x10, Cycles=4
Step 4: PC=0x0006, A=0x52, B=0x10, Cycles=4

Final state:
A = 0x52 (should be 0x52)
B = 0x10 (should be 0x10)
```

**Congratulations!** Your CPU is now reading and executing instructions from memory!

---

## Integration Tests

Let's write tests that verify CPU + MMU work together.

```rust
// Add to src/cpu/mod.rs tests

#[test]
fn test_cpu_executes_from_rom() {
    let mut cpu = Cpu::new();
    
    // Simple program: LD A, 0x42
    let rom = vec![0x3E, 0x42];
    cpu.mmu.load_rom_from_bytes(rom);
    cpu.registers.pc = 0x0000;
    
    cpu.step();
    
    assert_eq!(cpu.registers.a, 0x42);
    assert_eq!(cpu.registers.pc, 0x0002);
}

#[test]
fn test_cpu_multi_instruction_program() {
    let mut cpu = Cpu::new();
    
    let rom = vec![
        0x3E, 0x05,  // LD A, 5
        0x06, 0x03,  // LD B, 3
        0x80,        // ADD A, B
    ];
    cpu.mmu.load_rom_from_bytes(rom);
    cpu.registers.pc = 0x0000;
    
    cpu.step();  // LD A, 5
    cpu.step();  // LD B, 3
    cpu.step();  // ADD A, B
    
    assert_eq!(cpu.registers.a, 0x08);
}

#[test]
fn test_cpu_can_access_wram() {
    let mut cpu = Cpu::new();
    
    // Write to WRAM
    cpu.mmu.write(0xC000, 0x99);
    
    // Read from WRAM
    let value = cpu.mmu.read(0xC000);
    assert_eq!(value, 0x99);
}
```

---

## Milestones

### Milestone 1: Basic MMU Structure
- [ ] Create `src/mmu/mod.rs`
- [ ] Implement `Mmu` struct with memory regions
- [ ] Implement `read()` and `write()` methods
- [ ] Handle all memory regions (ROM, VRAM, WRAM, I/O, HRAM)
- [ ] Write 10+ unit tests
- [ ] All tests passing

**Checkpoint:** `cargo test` shows MMU tests passing

---

### Milestone 2: ROM Loading
- [ ] Implement `load_rom_from_bytes()`
- [ ] Implement `load_rom()` for file loading
- [ ] Add ROM header parsing (title, size)
- [ ] Test ROM reads
- [ ] Test that ROM cannot be written
- [ ] Write 5+ tests

**Checkpoint:** Can load and read ROM data

---

### Milestone 3: Memory Regions
- [ ] Test WRAM read/write
- [ ] Test VRAM read/write
- [ ] Test HRAM read/write
- [ ] Test Echo RAM mirrors WRAM
- [ ] Test I/O register access
- [ ] Test IE register (0xFFFF)
- [ ] Write 8+ tests

**Checkpoint:** All memory regions work correctly

---

### Milestone 4: Basic Banking
- [ ] Add `rom_bank` field to MMU
- [ ] Implement bank switching (0x2000-0x3FFF writes)
- [ ] Implement banked ROM reads (0x4000-0x7FFF)
- [ ] Test switching between banks
- [ ] Test bank 0 always visible at 0x0000-0x3FFF
- [ ] Write 5+ tests

**Checkpoint:** Can switch ROM banks

---

### Milestone 5: CPU-MMU Integration
- [ ] Add MMU to CPU struct
- [ ] Implement `fetch_byte()` method
- [ ] Update `step()` to fetch from memory
- [ ] Update `execute()` to fetch operands
- [ ] Test CPU executes from ROM
- [ ] Test multi-instruction programs
- [ ] Write integration tests

**Checkpoint:** CPU executes instructions from ROM

---

### Milestone 6: Test Program Execution
- [ ] Create test ROM with multiple instructions
- [ ] Execute and verify results
- [ ] Test PC advancement
- [ ] Test memory reads/writes during execution
- [ ] Update `main.rs` to run test program
- [ ] Verify output

**Checkpoint:** Can run complete programs!

---

## Phase 2 Complete!

When all milestones are done:

- [ ] All tests passing (30+ tests)
- [ ] CPU reads instructions from ROM
- [ ] All memory regions accessible
- [ ] Basic ROM banking works
- [ ] Can execute multi-instruction programs

**What You've Built:**
- Complete memory management system
- ROM loading from files
- Memory-mapped I/O foundation
- CPU-MMU integration
- Your emulator can now run real code!

**Next Steps:**
- Move to [Phase 3: Complete CPU](./phase3-complete-cpu.md)
- Implement all 256 main opcodes
- Add CB-prefixed instructions
- Implement full interrupt handling
- Pass Blargg's CPU tests

---

## Common Pitfalls

### Pitfall 1: Off-by-One in Address Ranges

```rust
// WRONG - doesn't include 0x9FFF
0x8000..0x9FFF => { /* ... */ }

// CORRECT - includes 0x9FFF
0x8000..=0x9FFF => { /* ... */ }
```

### Pitfall 2: Forgetting to Advance PC

```rust
// WRONG - PC not advanced
fn fetch_byte(&mut self) -> u8 {
    self.mmu.read(self.registers.pc)  // Forgot to increment!
}

// CORRECT
fn fetch_byte(&mut self) -> u8 {
    let byte = self.mmu.read(self.registers.pc);
    self.registers.pc += 1;
    byte
}
```

### Pitfall 3: Array Index Out of Bounds

```rust
// WRONG - could panic if rom_addr >= rom.len()
self.rom[rom_addr]

// CORRECT - check bounds
if rom_addr < self.rom.len() {
    self.rom[rom_addr]
} else {
    0xFF
}
```

### Pitfall 4: Incorrect Bank Calculation

```rust
// WRONG - wrong bank size
let rom_addr = self.rom_bank * 0x8000 + offset;  // Bank is 16KB, not 32KB!

// CORRECT
let rom_addr = self.rom_bank * 0x4000 + offset;  // 16KB = 0x4000
```

---

Great work! You now have a complete memory system. Your emulator can load ROMs and execute real programs. In Phase 3, we'll complete the CPU instruction set and start running actual Gameboy code!
