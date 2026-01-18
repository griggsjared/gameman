# Phase 1: CPU Foundation

Welcome to Phase 1! This is where your emulator journey truly begins. In this phase, you'll build the foundational CPU structure and implement your first instructions. By the end, you'll have a working CPU that can execute basic operations.

## Table of Contents

1. [Hardware Background](#hardware-background)
2. [CPU Architecture Design](#cpu-architecture-design)
3. [Register Implementation](#register-implementation)
4. [Instruction Decoder](#instruction-decoder)
5. [First Instructions](#first-instructions)
6. [Testing Strategy](#testing-strategy)
7. [Milestones](#milestones)

---

## Hardware Background

### The Sharp LR35902 Processor

The Gameboy uses a custom 8-bit CPU called the Sharp LR35902, which is similar to the Intel 8080 and Zilog Z80. Understanding its architecture is crucial.

**Key Characteristics:**
- 8-bit data bus, 16-bit address bus (can address 64KB)
- Clock speed: ~4.19 MHz
- 8 main registers: A, F, B, C, D, E, H, L
- 2 16-bit registers: SP (stack pointer), PC (program counter)
- Registers can be paired: AF, BC, DE, HL

**Register Overview:**

```
┌─────────────────────────────────────┐
│  A  │  F  │  - Accumulator & Flags  │
├─────────────────────────────────────┤
│  B  │  C  │  - General purpose      │
├─────────────────────────────────────┤
│  D  │  E  │  - General purpose      │
├─────────────────────────────────────┤
│  H  │  L  │  - Memory addressing    │
├─────────────────────────────────────┤
│    SP       │  - Stack Pointer      │
├─────────────────────────────────────┤
│    PC       │  - Program Counter    │
└─────────────────────────────────────┘
```

**Flag Register (F):**

The F register contains 4 flags in the upper nibble:

```
Bit 7: Z (Zero Flag)
Bit 6: N (Subtract Flag)
Bit 5: H (Half Carry Flag)
Bit 4: C (Carry Flag)
Bits 3-0: Always 0
```

Example: `F = 0b10010000` means Z=1, N=0, H=0, C=1, Rest=0

---

## CPU Architecture Design

Before writing code, let's think about structure. Where should the CPU live in your project?

### Option 1: Monolithic CPU Module

```
src/
├── main.rs
└── cpu.rs  // Everything in one file
```

**Pros:**
- Simple to start
- Easy to navigate initially
- No module complexity

**Cons:**
- Will become unwieldy (1000+ lines)
- Hard to organize opcodes
- Mixing concerns (registers, decoding, execution)

**Recommendation:** Good for Phase 1, refactor in Phase 3

### Option 2: Modular CPU Package

```
src/
├── main.rs
└── cpu/
    ├── mod.rs        // Public interface
    ├── registers.rs  // Register logic
    ├── opcodes.rs    // Instruction implementations
    └── decode.rs     // Decoding logic (Phase 3)
```

**Pros:**
- Clean separation of concerns
- Easier to test individual components
- Scales well to full implementation
- Each file stays manageable

**Cons:**
- More files to manage upfront
- Module system overhead

**Recommendation:** Use this from the start for a solid foundation

### Decision: Start with Option 2

We'll use the modular approach because:
1. Good practice for Rust module organization
2. Makes testing easier
3. You won't need to refactor later
4. Each file has a clear purpose

---

## Register Implementation

Let's implement the CPU registers. This is your first TDD opportunity!

### Design Approach 1: Individual Fields (Recommended)

**File: `src/cpu/registers.rs`**

```rust
/// CPU Registers for the Sharp LR35902
pub struct Registers {
    // 8-bit registers
    pub a: u8,  // Accumulator
    pub f: u8,  // Flags (only upper 4 bits used)
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    
    // 16-bit registers
    pub sp: u16,  // Stack pointer
    pub pc: u16,  // Program counter
}

// Flag bit positions
const FLAG_ZERO: u8 = 0b1000_0000;       // Bit 7
const FLAG_SUBTRACT: u8 = 0b0100_0000;   // Bit 6
const FLAG_HALF_CARRY: u8 = 0b0010_0000; // Bit 5
const FLAG_CARRY: u8 = 0b0001_0000;      // Bit 4

impl Registers {
    pub fn new() -> Self {
        Registers {
            a: 0,
            f: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            sp: 0,
            pc: 0,
        }
    }
    
    // 16-bit register pair getters
    pub fn af(&self) -> u16 {
        (self.a as u16) << 8 | (self.f as u16)
    }
    
    pub fn bc(&self) -> u16 {
        (self.b as u16) << 8 | (self.c as u16)
    }
    
    pub fn de(&self) -> u16 {
        (self.d as u16) << 8 | (self.e as u16)
    }
    
    pub fn hl(&self) -> u16 {
        (self.h as u16) << 8 | (self.l as u16)
    }
    
    // 16-bit register pair setters
    pub fn set_af(&mut self, value: u16) {
        self.a = (value >> 8) as u8;
        self.f = (value & 0xF0) as u8;  // Lower 4 bits always 0
    }
    
    pub fn set_bc(&mut self, value: u16) {
        self.b = (value >> 8) as u8;
        self.c = value as u8;
    }
    
    pub fn set_de(&mut self, value: u16) {
        self.d = (value >> 8) as u8;
        self.e = value as u8;
    }
    
    pub fn set_hl(&mut self, value: u16) {
        self.h = (value >> 8) as u8;
        self.l = value as u8;
    }
    
    // Flag getters
    pub fn zero(&self) -> bool {
        self.f & FLAG_ZERO != 0
    }
    
    pub fn subtract(&self) -> bool {
        self.f & FLAG_SUBTRACT != 0
    }
    
    pub fn half_carry(&self) -> bool {
        self.f & FLAG_HALF_CARRY != 0
    }
    
    pub fn carry(&self) -> bool {
        self.f & FLAG_CARRY != 0
    }
    
    // Flag setters
    pub fn set_zero(&mut self, value: bool) {
        if value {
            self.f |= FLAG_ZERO;
        } else {
            self.f &= !FLAG_ZERO;
        }
    }
    
    pub fn set_subtract(&mut self, value: bool) {
        if value {
            self.f |= FLAG_SUBTRACT;
        } else {
            self.f &= !FLAG_SUBTRACT;
        }
    }
    
    pub fn set_half_carry(&mut self, value: bool) {
        if value {
            self.f |= FLAG_HALF_CARRY;
        } else {
            self.f &= !FLAG_HALF_CARRY;
        }
    }
    
    pub fn set_carry(&mut self, value: bool) {
        if value {
            self.f |= FLAG_CARRY;
        } else {
            self.f &= !FLAG_CARRY;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_new_registers_initialized_to_zero() {
        let regs = Registers::new();
        assert_eq!(regs.a, 0);
        assert_eq!(regs.f, 0);
        assert_eq!(regs.b, 0);
        assert_eq!(regs.c, 0);
        assert_eq!(regs.d, 0);
        assert_eq!(regs.e, 0);
        assert_eq!(regs.h, 0);
        assert_eq!(regs.l, 0);
        assert_eq!(regs.sp, 0);
        assert_eq!(regs.pc, 0);
    }
    
    #[test]
    fn test_16bit_register_pairs_af() {
        let mut regs = Registers::new();
        regs.a = 0x12;
        regs.f = 0x30;
        assert_eq!(regs.af(), 0x1230);
    }
    
    #[test]
    fn test_16bit_register_pairs_bc() {
        let mut regs = Registers::new();
        regs.b = 0xAB;
        regs.c = 0xCD;
        assert_eq!(regs.bc(), 0xABCD);
    }
    
    #[test]
    fn test_set_16bit_register_pairs() {
        let mut regs = Registers::new();
        regs.set_bc(0x1234);
        assert_eq!(regs.b, 0x12);
        assert_eq!(regs.c, 0x34);
    }
    
    #[test]
    fn test_set_af_masks_lower_4_bits() {
        let mut regs = Registers::new();
        regs.set_af(0x12FF);  // Try to set lower 4 bits
        assert_eq!(regs.f, 0xF0);  // Lower 4 bits should be 0
    }
    
    #[test]
    fn test_zero_flag() {
        let mut regs = Registers::new();
        assert_eq!(regs.zero(), false);
        
        regs.set_zero(true);
        assert_eq!(regs.zero(), true);
        assert_eq!(regs.f, 0b1000_0000);
        
        regs.set_zero(false);
        assert_eq!(regs.zero(), false);
        assert_eq!(regs.f, 0b0000_0000);
    }
    
    #[test]
    fn test_carry_flag() {
        let mut regs = Registers::new();
        assert_eq!(regs.carry(), false);
        
        regs.set_carry(true);
        assert_eq!(regs.carry(), true);
        assert_eq!(regs.f, 0b0001_0000);
    }
    
    #[test]
    fn test_multiple_flags() {
        let mut regs = Registers::new();
        regs.set_zero(true);
        regs.set_carry(true);
        
        assert_eq!(regs.zero(), true);
        assert_eq!(regs.carry(), true);
        assert_eq!(regs.subtract(), false);
        assert_eq!(regs.half_carry(), false);
        assert_eq!(regs.f, 0b1001_0000);
    }
    
    #[test]
    fn test_flag_independence() {
        let mut regs = Registers::new();
        regs.set_zero(true);
        regs.set_carry(true);
        
        // Setting zero to false shouldn't affect carry
        regs.set_zero(false);
        assert_eq!(regs.carry(), true);
    }
}
```

**Pros:**
- Clear, readable field names
- Straightforward access
- Easy to debug
- Natural Rust idioms

**Cons:**
- More verbose
- Repeated patterns in flag operations

**When to use:** Always, unless you have a specific reason not to

---

### Design Approach 2: Array-Based Registers (Alternative)

```rust
pub struct Registers {
    // Index: 0=B, 1=C, 2=D, 3=E, 4=H, 5=L, 6=(HL), 7=A
    pub r: [u8; 8],
    pub f: u8,
    pub sp: u16,
    pub pc: u16,
}

impl Registers {
    pub fn a(&self) -> u8 { self.r[7] }
    pub fn b(&self) -> u8 { self.r[0] }
    pub fn c(&self) -> u8 { self.r[1] }
    // ... etc
    
    pub fn set_a(&mut self, value: u8) { self.r[7] = value; }
    // ... etc
}
```

**Pros:**
- Compact representation
- Can use index for decoding (some opcodes encode register in bits)
- Slightly more memory efficient

**Cons:**
- Less readable
- Magic indices (what's register 3?)
- More prone to off-by-one errors
- Harder to debug

**When to use:** If you're optimizing for size or implementing a specific decoding scheme

**Recommendation:** Start with Approach 1 (individual fields). You can always refactor later if needed.

---

## CPU Module Structure

Now let's connect the registers to a CPU struct.

**File: `src/cpu/mod.rs`**

```rust
mod registers;
pub use registers::Registers;

pub struct Cpu {
    pub registers: Registers,
    // We'll add more fields later (MMU, etc.)
}

impl Cpu {
    pub fn new() -> Self {
        Cpu {
            registers: Registers::new(),
        }
    }
    
    /// Execute a single instruction
    /// Returns the number of CPU cycles consumed
    pub fn step(&mut self) -> u8 {
        // TODO: Fetch, decode, execute
        // For now, just a placeholder
        4  // NOP takes 4 cycles
    }
    
    /// Reset the CPU to initial state
    pub fn reset(&mut self) {
        self.registers = Registers::new();
        // Gameboy starts execution at 0x0100
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
```

**File: `src/main.rs`**

```rust
mod cpu;

fn main() {
    let mut cpu = cpu::Cpu::new();
    cpu.reset();
    
    println!("Gameman - Gameboy Emulator");
    println!("CPU initialized. PC: 0x{:04X}", cpu.registers.pc);
}
```

**TDD Checkpoint:**

Run `cargo test` - you should see all tests passing!

```
running 12 tests
test cpu::tests::test_cpu_new ... ok
test cpu::tests::test_cpu_reset ... ok
test cpu::registers::tests::test_new_registers_initialized_to_zero ... ok
test cpu::registers::tests::test_16bit_register_pairs_af ... ok
...
test result: ok. 12 passed; 0 failed
```

---

## First Instructions

Now for the exciting part - implementing actual CPU instructions! We'll use TDD throughout.

### Understanding Opcodes

Each instruction has an **opcode** (operation code) - a byte value that identifies it. For example:
- `0x00` = NOP (No Operation)
- `0x3E` = LD A, n (Load immediate byte into A)
- `0x87` = ADD A, A (Add A to itself)

### Instruction 1: NOP (0x00)

The simplest instruction - it does nothing!

**Hardware Behavior:**
- Does nothing
- Takes 4 CPU cycles
- PC advances by 1

**TDD Test First:**

```rust
// Add to src/cpu/mod.rs tests
#[test]
fn test_execute_nop() {
    let mut cpu = Cpu::new();
    cpu.reset();
    
    let cycles = cpu.execute(0x00);  // NOP opcode
    
    assert_eq!(cycles, 4);
    assert_eq!(cpu.registers.pc, 0x0101);  // PC advanced by 1
}
```

**Implementation:**

```rust
// Add to impl Cpu in src/cpu/mod.rs
pub fn execute(&mut self, opcode: u8) -> u8 {
    match opcode {
        0x00 => self.nop(),
        _ => panic!("Unimplemented opcode: 0x{:02X}", opcode),
    }
}

fn nop(&mut self) -> u8 {
    self.registers.pc += 1;
    4  // Cycles
}
```

Run `cargo test` - your test should pass!

---

### Instruction 2: LD r, n (Load immediate)

Load a byte directly into a register.

**Example: LD A, n (0x3E)**
- Opcode: 0x3E
- Reads next byte from memory
- Stores it in register A
- PC advances by 2 (opcode + data byte)
- Takes 8 cycles

**TDD Test:**

```rust
#[test]
fn test_execute_ld_a_immediate() {
    let mut cpu = Cpu::new();
    cpu.reset();
    
    // We need a way to provide the immediate value
    // For now, let's pass it as a parameter
    let cycles = cpu.execute_with_byte(0x3E, 0x42);
    
    assert_eq!(cpu.registers.a, 0x42);
    assert_eq!(cpu.registers.pc, 0x0102);  // Advanced by 2
    assert_eq!(cycles, 8);
}
```

**Implementation:**

```rust
// We need to handle instructions that read additional bytes
// For now, we'll add a helper method
pub fn execute_with_byte(&mut self, opcode: u8, byte: u8) -> u8 {
    match opcode {
        0x00 => self.nop(),
        0x3E => self.ld_a_n(byte),
        0x06 => self.ld_b_n(byte),
        0x0E => self.ld_c_n(byte),
        0x16 => self.ld_d_n(byte),
        0x1E => self.ld_e_n(byte),
        0x26 => self.ld_h_n(byte),
        0x2E => self.ld_l_n(byte),
        _ => panic!("Unimplemented opcode: 0x{:02X}", opcode),
    }
}

fn ld_a_n(&mut self, n: u8) -> u8 {
    self.registers.a = n;
    self.registers.pc += 2;
    8
}

fn ld_b_n(&mut self, n: u8) -> u8 {
    self.registers.b = n;
    self.registers.pc += 2;
    8
}

// Add similar methods for C, D, E, H, L
```

**More Tests:**

```rust
#[test]
fn test_execute_ld_b_immediate() {
    let mut cpu = Cpu::new();
    cpu.reset();
    
    cpu.execute_with_byte(0x06, 0xAB);
    
    assert_eq!(cpu.registers.b, 0xAB);
    assert_eq!(cpu.registers.pc, 0x0102);
}

#[test]
fn test_execute_multiple_loads() {
    let mut cpu = Cpu::new();
    cpu.reset();
    
    cpu.execute_with_byte(0x3E, 0x12);  // LD A, 0x12
    cpu.execute_with_byte(0x06, 0x34);  // LD B, 0x34
    
    assert_eq!(cpu.registers.a, 0x12);
    assert_eq!(cpu.registers.b, 0x34);
    assert_eq!(cpu.registers.pc, 0x0104);
}
```

---

### Instruction 3: LD r, r (Register to Register)

Copy value from one register to another.

**Example: LD A, B (0x78)**
- Copy value from B into A
- PC advances by 1
- Takes 4 cycles

**TDD Tests:**

```rust
#[test]
fn test_ld_a_b() {
    let mut cpu = Cpu::new();
    cpu.reset();
    cpu.registers.b = 0x42;
    
    cpu.execute(0x78);  // LD A, B
    
    assert_eq!(cpu.registers.a, 0x42);
    assert_eq!(cpu.registers.b, 0x42);  // B unchanged
    assert_eq!(cpu.registers.pc, 0x0101);
}

#[test]
fn test_ld_b_c() {
    let mut cpu = Cpu::new();
    cpu.reset();
    cpu.registers.c = 0x99;
    
    cpu.execute(0x41);  // LD B, C
    
    assert_eq!(cpu.registers.b, 0x99);
}
```

**Implementation:**

```rust
pub fn execute(&mut self, opcode: u8) -> u8 {
    match opcode {
        0x00 => self.nop(),
        
        // LD A, r
        0x7F => self.ld_a_a(),
        0x78 => self.ld_a_b(),
        0x79 => self.ld_a_c(),
        0x7A => self.ld_a_d(),
        0x7B => self.ld_a_e(),
        0x7C => self.ld_a_h(),
        0x7D => self.ld_a_l(),
        
        // LD B, r
        0x47 => self.ld_b_a(),
        0x40 => self.ld_b_b(),
        0x41 => self.ld_b_c(),
        // ... etc for all combinations
        
        _ => panic!("Unimplemented opcode: 0x{:02X}", opcode),
    }
}

fn ld_a_b(&mut self) -> u8 {
    self.registers.a = self.registers.b;
    self.registers.pc += 1;
    4
}

fn ld_b_c(&mut self) -> u8 {
    self.registers.b = self.registers.c;
    self.registers.pc += 1;
    4
}

// Note: This is tedious! There are 7*7=49 combinations
// We'll refactor this with macros or a better pattern in Phase 3
// For now, implement a few key ones
```

**Pattern Recognition:**

Notice the opcode pattern for LD r, r:
- Format: `0b01DDDSSS`
- DDD = destination register (3 bits)
- SSS = source register (3 bits)

Register encoding:
- 000 = B
- 001 = C
- 010 = D
- 011 = E
- 100 = H
- 101 = L
- 110 = (HL) - special case
- 111 = A

We'll use this pattern later for automatic decoding!

---

### Instruction 4: INC r (Increment Register)

Increment a register by 1 and set flags.

**Example: INC A (0x3C)**
- A = A + 1
- Flags: Z, N, H (C unaffected)
- Z = 1 if result is 0
- N = 0 (not a subtraction)
- H = 1 if carry from bit 3 to bit 4

**Half Carry Explanation:**

Half carry happens when the lower nibble (4 bits) overflows.

```
  0x0F (0000_1111)
+    1
-----------------
  0x10 (0001_0000)
       ^
       Half carry occurred here
```

**TDD Tests:**

```rust
#[test]
fn test_inc_a_normal() {
    let mut cpu = Cpu::new();
    cpu.reset();
    cpu.registers.a = 0x05;
    
    cpu.execute(0x3C);  // INC A
    
    assert_eq!(cpu.registers.a, 0x06);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
}

#[test]
fn test_inc_a_sets_zero_flag() {
    let mut cpu = Cpu::new();
    cpu.reset();
    cpu.registers.a = 0xFF;
    
    cpu.execute(0x3C);  // INC A
    
    assert_eq!(cpu.registers.a, 0x00);  // Wraps around
    assert_eq!(cpu.registers.zero(), true);
}

#[test]
fn test_inc_a_sets_half_carry() {
    let mut cpu = Cpu::new();
    cpu.reset();
    cpu.registers.a = 0x0F;
    
    cpu.execute(0x3C);  // INC A
    
    assert_eq!(cpu.registers.a, 0x10);
    assert_eq!(cpu.registers.half_carry(), true);
}

#[test]
fn test_inc_does_not_affect_carry_flag() {
    let mut cpu = Cpu::new();
    cpu.reset();
    cpu.registers.a = 0xFF;
    cpu.registers.set_carry(true);
    
    cpu.execute(0x3C);  // INC A
    
    // Carry flag should remain set
    assert_eq!(cpu.registers.carry(), true);
}
```

**Implementation:**

```rust
pub fn execute(&mut self, opcode: u8) -> u8 {
    match opcode {
        // ... previous opcodes
        0x3C => self.inc_a(),
        0x04 => self.inc_b(),
        0x0C => self.inc_c(),
        // ... etc
        _ => panic!("Unimplemented opcode: 0x{:02X}", opcode),
    }
}

fn inc_a(&mut self) -> u8 {
    self.registers.a = self.inc_u8(self.registers.a);
    self.registers.pc += 1;
    4
}

fn inc_b(&mut self) -> u8 {
    self.registers.b = self.inc_u8(self.registers.b);
    self.registers.pc += 1;
    4
}

// Helper method for increment logic
fn inc_u8(&mut self, value: u8) -> u8 {
    let result = value.wrapping_add(1);
    
    // Set flags
    self.registers.set_zero(result == 0);
    self.registers.set_subtract(false);
    self.registers.set_half_carry((value & 0x0F) == 0x0F);
    // Carry flag is NOT affected by INC
    
    result
}
```

**Key Points:**
- Use `wrapping_add` to handle overflow (0xFF + 1 = 0x00)
- Half carry: `(value & 0x0F) == 0x0F` checks if lower nibble is all 1s
- N flag always set to 0 for increment
- Carry flag is NOT modified

---

### Instruction 5: DEC r (Decrement Register)

Similar to INC, but subtracts 1.

**TDD Tests:**

```rust
#[test]
fn test_dec_a_normal() {
    let mut cpu = Cpu::new();
    cpu.reset();
    cpu.registers.a = 0x05;
    
    cpu.execute(0x3D);  // DEC A
    
    assert_eq!(cpu.registers.a, 0x04);
    assert_eq!(cpu.registers.subtract(), true);  // N flag set for subtraction
}

#[test]
fn test_dec_a_sets_zero_flag() {
    let mut cpu = Cpu::new();
    cpu.reset();
    cpu.registers.a = 0x01;
    
    cpu.execute(0x3D);  // DEC A
    
    assert_eq!(cpu.registers.a, 0x00);
    assert_eq!(cpu.registers.zero(), true);
}

#[test]
fn test_dec_a_wraps_around() {
    let mut cpu = Cpu::new();
    cpu.reset();
    cpu.registers.a = 0x00;
    
    cpu.execute(0x3D);  // DEC A
    
    assert_eq!(cpu.registers.a, 0xFF);  // Wraps to 255
    assert_eq!(cpu.registers.zero(), false);
}

#[test]
fn test_dec_a_sets_half_carry() {
    let mut cpu = Cpu::new();
    cpu.reset();
    cpu.registers.a = 0x10;
    
    cpu.execute(0x3D);  // DEC A
    
    assert_eq!(cpu.registers.a, 0x0F);
    assert_eq!(cpu.registers.half_carry(), true);  // Borrow from bit 4
}
```

**Implementation:**

```rust
fn dec_a(&mut self) -> u8 {
    self.registers.a = self.dec_u8(self.registers.a);
    self.registers.pc += 1;
    4
}

fn dec_u8(&mut self, value: u8) -> u8 {
    let result = value.wrapping_sub(1);
    
    // Set flags
    self.registers.set_zero(result == 0);
    self.registers.set_subtract(true);  // This is a subtraction
    self.registers.set_half_carry((value & 0x0F) == 0x00);  // Borrow occurred
    // Carry flag is NOT affected
    
    result
}
```

**Half Carry for Subtraction:**

For DEC, half carry is set when borrowing from bit 4:
```
  0x10 (0001_0000)
-    1
-----------------
  0x0F (0000_1111)
       ^
       Borrow occurred
```

Check: `(value & 0x0F) == 0x00` (lower nibble is 0000)

---

### Instruction 6: ADD A, r

Add a register to A and set flags.

**Example: ADD A, B (0x80)**
- A = A + B
- Flags: Z, N, H, C all affected

**TDD Tests:**

```rust
#[test]
fn test_add_a_b_normal() {
    let mut cpu = Cpu::new();
    cpu.reset();
    cpu.registers.a = 0x05;
    cpu.registers.b = 0x03;
    
    cpu.execute(0x80);  // ADD A, B
    
    assert_eq!(cpu.registers.a, 0x08);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
}

#[test]
fn test_add_sets_carry_flag() {
    let mut cpu = Cpu::new();
    cpu.reset();
    cpu.registers.a = 0xFF;
    cpu.registers.b = 0x02;
    
    cpu.execute(0x80);
    
    assert_eq!(cpu.registers.a, 0x01);  // Wraps around
    assert_eq!(cpu.registers.carry(), true);
}

#[test]
fn test_add_sets_half_carry_flag() {
    let mut cpu = Cpu::new();
    cpu.reset();
    cpu.registers.a = 0x0F;
    cpu.registers.b = 0x01;
    
    cpu.execute(0x80);
    
    assert_eq!(cpu.registers.a, 0x10);
    assert_eq!(cpu.registers.half_carry(), true);
}

#[test]
fn test_add_sets_zero_flag() {
    let mut cpu = Cpu::new();
    cpu.reset();
    cpu.registers.a = 0xFF;
    cpu.registers.b = 0x01;
    
    cpu.execute(0x80);
    
    assert_eq!(cpu.registers.a, 0x00);
    assert_eq!(cpu.registers.zero(), true);
}
```

**Implementation:**

```rust
fn add_a_b(&mut self) -> u8 {
    let b_val = self.registers.b;
    self.add_a(b_val);
    self.registers.pc += 1;
    4
}

fn add_a(&mut self, value: u8) {
    let a = self.registers.a;
    let result = a.wrapping_add(value);
    
    // Set flags
    self.registers.set_zero(result == 0);
    self.registers.set_subtract(false);
    self.registers.set_half_carry((a & 0x0F) + (value & 0x0F) > 0x0F);
    self.registers.set_carry(a as u16 + value as u16 > 0xFF);
    
    self.registers.a = result;
}
```

**Flag Calculations Explained:**

1. **Zero Flag**: `result == 0`
2. **Subtract Flag**: Always `false` for addition
3. **Half Carry**: `(a & 0x0F) + (value & 0x0F) > 0x0F`
   - Add lower nibbles
   - If sum > 15, there was a carry from bit 3
4. **Carry Flag**: `a as u16 + value as u16 > 0xFF`
   - Cast to u16 to detect overflow
   - If sum > 255, carry occurred

---

## Testing Strategy

### Unit Test Organization

Keep tests close to implementation:

```rust
// In src/cpu/registers.rs
#[cfg(test)]
mod tests {
    use super::*;
    // Tests for Registers struct
}

// In src/cpu/mod.rs
#[cfg(test)]
mod tests {
    use super::*;
    // Tests for CPU execution
}
```

### Test Patterns

**Pattern 1: Single Instruction Test**

```rust
#[test]
fn test_opcode_name_specific_case() {
    // Setup
    let mut cpu = Cpu::new();
    cpu.reset();
    cpu.registers.a = 0x05;  // Initial state
    
    // Execute
    cpu.execute(0x3C);  // INC A
    
    // Assert
    assert_eq!(cpu.registers.a, 0x06);
    assert_eq!(cpu.registers.zero(), false);
}
```

**Pattern 2: Flag Boundary Test**

```rust
#[test]
fn test_opcode_boundary_condition() {
    let mut cpu = Cpu::new();
    cpu.reset();
    cpu.registers.a = 0xFF;  // Edge case value
    
    cpu.execute(0x3C);
    
    // Check wrap-around and flags
    assert_eq!(cpu.registers.a, 0x00);
    assert_eq!(cpu.registers.zero(), true);
}
```

**Pattern 3: Multi-Instruction Sequence**

```rust
#[test]
fn test_instruction_sequence() {
    let mut cpu = Cpu::new();
    cpu.reset();
    
    cpu.execute_with_byte(0x3E, 0x05);  // LD A, 5
    cpu.execute(0x3C);                   // INC A
    cpu.execute(0x3C);                   // INC A
    
    assert_eq!(cpu.registers.a, 0x07);
}
```

### TDD Workflow

1. **Write Test First** - Think about the behavior you want
2. **Run Test** - It should fail (red)
3. **Implement Minimum Code** - Make it pass
4. **Run Test** - It should pass (green)
5. **Refactor** - Clean up code
6. **Run Tests Again** - Ensure still passing

---

## Milestones

Track your progress through Phase 1:

### Milestone 1: Register Foundation
- [ ] Create `src/cpu/registers.rs`
- [ ] Implement `Registers` struct with all fields
- [ ] Implement 16-bit register pair getters/setters
- [ ] Implement flag getters/setters
- [ ] Write 10+ unit tests for registers
- [ ] All tests passing

**Checkpoint:** `cargo test` shows all register tests passing

---

### Milestone 2: CPU Structure
- [ ] Create `src/cpu/mod.rs`
- [ ] Implement `Cpu` struct
- [ ] Implement `new()` and `reset()` methods
- [ ] Create `execute()` method skeleton
- [ ] Write tests for CPU initialization

**Checkpoint:** CPU can be created and reset

---

### Milestone 3: First Instructions (NOP, LD)
- [ ] Implement NOP (0x00)
- [ ] Implement LD r, n (immediate loads) for A, B, C, D, E, H, L
- [ ] Implement LD r, r (register to register) for key combinations
- [ ] Write 5+ tests per instruction
- [ ] All tests passing

**Checkpoint:** Can load values into registers

---

### Milestone 4: Arithmetic (INC, DEC)
- [ ] Implement INC r for all registers
- [ ] Implement DEC r for all registers
- [ ] Properly handle flag setting
- [ ] Test edge cases (0xFF + 1, 0x00 - 1)
- [ ] Test half carry conditions

**Checkpoint:** Can increment and decrement with correct flags

---

### Milestone 5: Addition (ADD)
- [ ] Implement ADD A, r for all registers
- [ ] Implement ADD A, n (immediate)
- [ ] Properly handle all flags (Z, N, H, C)
- [ ] Test boundary conditions
- [ ] Test complex sequences

**Checkpoint:** Can perform addition with correct flag behavior

---

### Milestone 6: Integration Test
- [ ] Write a test that uses multiple instructions
- [ ] Verify PC advances correctly
- [ ] Verify flag interactions
- [ ] Test a mini "program" (sequence of instructions)

**Example Integration Test:**

```rust
#[test]
fn test_mini_program() {
    let mut cpu = Cpu::new();
    cpu.reset();
    
    // Mini program:
    // LD A, 5
    // INC A
    // LD B, 3
    // ADD A, B
    // Result: A should be 9
    
    cpu.execute_with_byte(0x3E, 0x05);  // LD A, 5
    assert_eq!(cpu.registers.a, 5);
    
    cpu.execute(0x3C);                   // INC A
    assert_eq!(cpu.registers.a, 6);
    
    cpu.execute_with_byte(0x06, 0x03);  // LD B, 3
    assert_eq!(cpu.registers.b, 3);
    
    cpu.execute(0x80);                   // ADD A, B
    assert_eq!(cpu.registers.a, 9);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.carry(), false);
}
```

**Checkpoint:** Multi-instruction programs work correctly

---

## Phase 1 Complete!

When you've completed all milestones:

- [ ] All tests passing (40+ tests)
- [ ] Can execute NOP, LD, INC, DEC, ADD instructions
- [ ] Flags working correctly
- [ ] PC advancing properly
- [ ] Code well-organized and documented

**Next Steps:**
- Move to [Phase 2: Memory Management](./phase2-memory-management.md)
- Implement MMU to read actual ROM data
- Connect CPU to memory for real instruction fetching

---

## Common Pitfalls

### Pitfall 1: Forgetting to Advance PC

```rust
// WRONG
fn nop(&mut self) -> u8 {
    4  // Forgot to increment PC!
}

// CORRECT
fn nop(&mut self) -> u8 {
    self.registers.pc += 1;
    4
}
```

### Pitfall 2: Incorrect Half Carry Logic

```rust
// WRONG - checking wrong bit
self.registers.set_half_carry((a & 0x80) + (value & 0x80) > 0x80);

// CORRECT - check bit 3 to 4 carry
self.registers.set_half_carry((a & 0x0F) + (value & 0x0F) > 0x0F);
```

### Pitfall 3: Not Masking Flag Register

```rust
// WRONG - lower 4 bits can be set
pub fn set_af(&mut self, value: u16) {
    self.a = (value >> 8) as u8;
    self.f = value as u8;  // Bug!
}

// CORRECT
pub fn set_af(&mut self, value: u16) {
    self.a = (value >> 8) as u8;
    self.f = (value & 0xF0) as u8;  // Mask lower 4 bits
}
```

### Pitfall 4: Forgetting Wrapping Arithmetic

```rust
// WRONG - will panic in debug mode
let result = value + 1;

// CORRECT
let result = value.wrapping_add(1);
```

---

## Resources for Phase 1

- **CPU Instruction Reference**: See [resources.md](./resources.md) for opcode tables
- **Flag Behavior**: Pan Docs has detailed flag descriptions
- **Test Ideas**: Look at other emulators' test suites

---

**Congratulations on completing Phase 1!** You now have a working CPU foundation. Take a moment to appreciate what you've built - you can execute real CPU instructions!

When you're ready, move on to [Phase 2: Memory Management](./phase2-memory-management.md) where we'll add the ability to read instructions from actual ROM files.
