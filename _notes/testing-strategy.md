# Testing Strategy

Test-Driven Development (TDD) is central to building a reliable emulator. This document outlines testing patterns, workflows, and best practices for the Gameman project.

## Table of Contents

1. [Why TDD for Emulators](#why-tdd-for-emulators)
2. [Testing Workflow](#testing-workflow)
3. [Test Organization](#test-organization)
4. [Unit Testing Patterns](#unit-testing-patterns)
5. [Integration Testing](#integration-testing)
6. [Test ROM Usage](#test-rom-usage)
7. [Debugging Failed Tests](#debugging-failed-tests)
8. [Coverage Goals](#coverage-goals)

---

## Why TDD for Emulators

Emulators are perfect candidates for TDD because:

1. **Specification is well-defined**: Hardware behavior is documented
2. **Regressions are common**: Small changes can break working features
3. **Complex interactions**: CPU, memory, graphics, timing all interact
4. **Community test ROMs**: Industry-standard tests exist (Blargg, etc.)
5. **Refactoring is necessary**: Code evolves as you understand patterns

**Benefits:**
- Catch bugs early (before they compound)
- Confidence when refactoring
- Documentation through tests
- Clear success criteria

---

## Testing Workflow

### The TDD Cycle

```
┌─────────────────────────────────────┐
│  1. Write a failing test (RED)      │
│     - Think about desired behavior  │
│     - Write test that expects it    │
│     - Run test → should FAIL        │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│  2. Write minimal code (GREEN)      │
│     - Implement just enough         │
│     - Run test → should PASS        │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│  3. Refactor (REFACTOR)             │
│     - Clean up code                 │
│     - Remove duplication            │
│     - Run tests → still PASS        │
└──────────────┬──────────────────────┘
               │
               └─────────────────────▶ Repeat
```

### Example: Implementing INC A

**Step 1: Write Test (RED)**

```rust
#[test]
fn test_inc_a_normal() {
    let mut cpu = Cpu::new();
    cpu.reset();
    cpu.registers.a = 0x05;
    
    cpu.execute(0x3C);  // INC A
    
    assert_eq!(cpu.registers.a, 0x06);
    assert_eq!(cpu.registers.zero(), false);
}
```

Run: `cargo test test_inc_a_normal`
Result: **FAILS** (opcode 0x3C not implemented)

**Step 2: Implement (GREEN)**

```rust
pub fn execute(&mut self, opcode: u8) -> u8 {
    match opcode {
        // ... existing opcodes
        0x3C => {
            self.registers.a = self.registers.a.wrapping_add(1);
            self.registers.set_zero(self.registers.a == 0);
            self.registers.set_subtract(false);
            self.registers.set_half_carry((self.registers.a & 0x0F) == 0x00);
            self.registers.pc += 1;
            4
        }
        // ...
    }
}
```

Run: `cargo test test_inc_a_normal`
Result: **PASSES**

**Step 3: Refactor**

```rust
// Extract common logic
fn inc_u8(&mut self, value: u8) -> u8 {
    let result = value.wrapping_add(1);
    self.registers.set_zero(result == 0);
    self.registers.set_subtract(false);
    self.registers.set_half_carry((value & 0x0F) == 0x0F);
    result
}

// Use in multiple places
0x3C => {
    self.registers.a = self.inc_u8(self.registers.a);
    self.registers.pc += 1;
    4
}
```

Run: `cargo test`
Result: All tests still **PASS**

---

## Test Organization

### File Structure

```
src/
├── cpu/
│   ├── mod.rs
│   │   #[cfg(test)]
│   │   mod tests { ... }  // Unit tests here
│   └── registers.rs
│       #[cfg(test)]
│       mod tests { ... }  // Unit tests here
├── mmu/
│   └── mod.rs
│       #[cfg(test)]
│       mod tests { ... }
└── ...

tests/
├── cpu_integration.rs     // Integration tests
├── mmu_integration.rs
└── blargg/
    ├── cpu_instrs.rs      // Test ROM runners
    └── ...
```

### Unit vs Integration Tests

**Unit Tests** (`#[cfg(test)]` in source files)
- Test single functions/methods
- Fast (< 1ms each)
- No external dependencies
- Run with `cargo test`

Example:
```rust
#[test]
fn test_register_flag_operations() {
    let mut reg = Registers::new();
    reg.set_zero(true);
    assert_eq!(reg.zero(), true);
    assert_eq!(reg.f, 0b1000_0000);
}
```

**Integration Tests** (`tests/` directory)
- Test multiple subsystems together
- Slower (10-100ms each)
- May load ROM files
- Run with `cargo test --test <name>`

Example:
```rust
#[test]
fn test_cpu_executes_program_from_rom() {
    let mut cpu = Cpu::new();
    let rom = vec![0x3E, 0x42, 0x3C, 0x3C];  // LD A, 0x42; INC A; INC A
    cpu.mmu.load_rom_from_bytes(rom);
    cpu.registers.pc = 0;
    
    cpu.step();  // LD A, 0x42
    cpu.step();  // INC A
    cpu.step();  // INC A
    
    assert_eq!(cpu.registers.a, 0x44);
}
```

---

## Unit Testing Patterns

### Pattern 1: Simple Operation Test

Tests a single operation with normal inputs.

```rust
#[test]
fn test_add_a_b_normal() {
    let mut cpu = Cpu::new();
    cpu.reset();
    cpu.registers.a = 0x10;
    cpu.registers.b = 0x05;
    
    cpu.execute(0x80);  // ADD A, B
    
    assert_eq!(cpu.registers.a, 0x15);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.carry(), false);
}
```

**When to use:** Every instruction needs at least one normal-case test.

---

### Pattern 2: Edge Case Test

Tests boundary conditions and special values.

```rust
#[test]
fn test_add_causes_overflow() {
    let mut cpu = Cpu::new();
    cpu.reset();
    cpu.registers.a = 0xFF;
    cpu.registers.b = 0x02;
    
    cpu.execute(0x80);  // ADD A, B
    
    assert_eq!(cpu.registers.a, 0x01);  // Wraps around
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.carry(), true);  // Overflow flag
}

#[test]
fn test_add_results_in_zero() {
    let mut cpu = Cpu::new();
    cpu.reset();
    cpu.registers.a = 0xFF;
    cpu.registers.b = 0x01;
    
    cpu.execute(0x80);
    
    assert_eq!(cpu.registers.a, 0x00);
    assert_eq!(cpu.registers.zero(), true);  // Zero flag set
}
```

**When to use:** For every flag condition, overflow, underflow, max/min values.

---

### Pattern 3: Flag Independence Test

Verifies one flag doesn't affect another.

```rust
#[test]
fn test_inc_does_not_affect_carry() {
    let mut cpu = Cpu::new();
    cpu.reset();
    cpu.registers.a = 0xFF;
    cpu.registers.set_carry(true);
    
    cpu.execute(0x3C);  // INC A (wraps to 0)
    
    assert_eq!(cpu.registers.a, 0x00);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.carry(), true);  // Should still be set!
}
```

**When to use:** For instructions that don't modify all flags.

---

### Pattern 4: Sequence Test

Tests multiple operations in sequence.

```rust
#[test]
fn test_sequence_load_inc_add() {
    let mut cpu = Cpu::new();
    cpu.reset();
    
    cpu.execute_with_byte(0x3E, 0x05);  // LD A, 5
    cpu.execute(0x3C);                   // INC A (A = 6)
    cpu.execute_with_byte(0x06, 0x04);  // LD B, 4
    cpu.execute(0x80);                   // ADD A, B (A = 10)
    
    assert_eq!(cpu.registers.a, 0x0A);
    assert_eq!(cpu.registers.b, 0x04);
}
```

**When to use:** To verify operations compose correctly.

---

### Pattern 5: State Preservation Test

Ensures operations don't have unexpected side effects.

```rust
#[test]
fn test_add_preserves_other_registers() {
    let mut cpu = Cpu::new();
    cpu.reset();
    cpu.registers.a = 0x10;
    cpu.registers.b = 0x05;
    cpu.registers.c = 0xAA;  // Should not change
    cpu.registers.d = 0xBB;  // Should not change
    
    cpu.execute(0x80);  // ADD A, B
    
    assert_eq!(cpu.registers.c, 0xAA);  // Unchanged
    assert_eq!(cpu.registers.d, 0xBB);  // Unchanged
}
```

**When to use:** For operations that should only affect specific registers.

---

### Pattern 6: Parameterized Tests

Use a helper to test multiple similar cases.

```rust
fn test_ld_register(opcode: u8, get_reg: impl Fn(&Cpu) -> u8) {
    let mut cpu = Cpu::new();
    cpu.reset();
    
    cpu.execute_with_byte(opcode, 0x42);
    
    assert_eq!(get_reg(&cpu), 0x42);
    assert_eq!(cpu.registers.pc, 0x0102);
}

#[test]
fn test_ld_a_n() { test_ld_register(0x3E, |cpu| cpu.registers.a); }

#[test]
fn test_ld_b_n() { test_ld_register(0x06, |cpu| cpu.registers.b); }

#[test]
fn test_ld_c_n() { test_ld_register(0x0E, |cpu| cpu.registers.c); }
```

**When to use:** For repetitive tests with similar structure.

---

## Integration Testing

### Testing CPU + MMU

```rust
// tests/cpu_integration.rs
use gameman::cpu::Cpu;

#[test]
fn test_cpu_fetches_from_rom() {
    let mut cpu = Cpu::new();
    
    let rom = vec![
        0x00,        // 0x0000: NOP
        0x3E, 0x42,  // 0x0001: LD A, 0x42
        0x00,        // 0x0003: NOP
    ];
    
    cpu.mmu.load_rom_from_bytes(rom);
    cpu.registers.pc = 0x0000;
    
    cpu.step();  // NOP
    assert_eq!(cpu.registers.pc, 0x0001);
    
    cpu.step();  // LD A, 0x42
    assert_eq!(cpu.registers.a, 0x42);
    assert_eq!(cpu.registers.pc, 0x0003);
    
    cpu.step();  // NOP
    assert_eq!(cpu.registers.pc, 0x0004);
}
```

### Testing Memory Access

```rust
#[test]
fn test_cpu_reads_writes_wram() {
    let mut cpu = Cpu::new();
    
    let rom = vec![
        0x3E, 0x99,        // LD A, 0x99
        0xEA, 0x00, 0xC0,  // LD (0xC000), A
        0xFA, 0x00, 0xC0,  // LD A, (0xC000)
    ];
    
    cpu.mmu.load_rom_from_bytes(rom);
    cpu.registers.pc = 0x0000;
    
    cpu.step();  // LD A, 0x99
    cpu.step();  // LD (0xC000), A
    
    // Verify write happened
    assert_eq!(cpu.mmu.read(0xC000), 0x99);
    
    // Clear register
    cpu.registers.a = 0x00;
    
    cpu.step();  // LD A, (0xC000)
    
    // Verify read happened
    assert_eq!(cpu.registers.a, 0x99);
}
```

### Testing Bank Switching

```rust
#[test]
fn test_rom_bank_switching() {
    let mut cpu = Cpu::new();
    
    let mut rom = vec![0x00; 0xC000];  // 3 banks
    rom[0x0000] = 0x3E;  // LD A, n
    rom[0x0001] = 0x11;  // n = 0x11 (in bank 0)
    
    rom[0x4000] = 0x22;  // Data in bank 1
    rom[0x8000] = 0x33;  // Data in bank 2
    
    cpu.mmu.load_rom_from_bytes(rom);
    cpu.registers.pc = 0x0000;
    
    // Read from bank 1 (default)
    assert_eq!(cpu.mmu.read(0x4000), 0x22);
    
    // Switch to bank 2
    cpu.mmu.write(0x2000, 0x02);
    assert_eq!(cpu.mmu.read(0x4000), 0x33);
}
```

---

## Test ROM Usage

### What are Test ROMs?

Test ROMs are specially crafted Gameboy programs that test specific hardware behavior. They're the gold standard for emulator accuracy.

### Common Test ROM Suites

**1. Blargg's Test ROMs**
- `cpu_instrs.gb` - Tests all CPU instructions
- `mem_timing.gb` - Tests memory access timing
- `instr_timing.gb` - Tests instruction timing

**Usage:**
```rust
#[test]
#[ignore]  // Long-running test
fn test_blargg_cpu_instrs() {
    let mut cpu = Cpu::new();
    cpu.mmu.load_rom("roms/test/cpu_instrs.gb").unwrap();
    cpu.reset();
    
    // Run for many cycles
    for _ in 0..10_000_000 {
        cpu.step();
    }
    
    // Check result in memory (specific location varies by test)
    // Usually writes "Passed" or "Failed" to serial output
    let result = read_serial_output(&cpu);
    assert!(result.contains("Passed"));
}
```

**2. Mooneye Test Suite**
- More granular tests
- Tests specific edge cases
- Individual test per behavior

**3. dmg-acid2**
- Graphics test
- Renders specific pattern
- Compare output frame to reference

### Running Test ROMs

```rust
// tests/blargg/cpu_instrs.rs
use std::fs;

#[test]
#[ignore]  // Mark as long-running
fn test_cpu_instrs() {
    let rom_path = "roms/test/blargg/cpu_instrs.gb";
    
    // Skip if ROM not present
    if !Path::new(rom_path).exists() {
        println!("Skipping test - ROM not found: {}", rom_path);
        return;
    }
    
    let mut cpu = Cpu::new();
    cpu.mmu.load_rom(rom_path).unwrap();
    cpu.reset();
    
    // Run until test completes (or timeout)
    let mut cycles = 0;
    let max_cycles = 100_000_000;  // Timeout
    
    while cycles < max_cycles {
        cpu.step();
        cycles += 1;
        
        // Check for test completion signal
        if cpu.test_completed() {
            break;
        }
    }
    
    assert!(cpu.test_passed(), "CPU instruction test failed!");
}
```

**Note:** Test ROMs are used in later phases (Phase 3+) after implementing more CPU instructions.

---

## Debugging Failed Tests

### Strategy 1: Isolate the Failure

```rust
#[test]
fn test_add_with_carry() {
    let mut cpu = Cpu::new();
    cpu.reset();
    cpu.registers.a = 0xFF;
    cpu.registers.b = 0x01;
    
    println!("Before: A={:02X}, F={:08b}", cpu.registers.a, cpu.registers.f);
    
    cpu.execute(0x80);  // ADD A, B
    
    println!("After: A={:02X}, F={:08b}", cpu.registers.a, cpu.registers.f);
    println!("Z={}, N={}, H={}, C={}",
             cpu.registers.zero(),
             cpu.registers.subtract(),
             cpu.registers.half_carry(),
             cpu.registers.carry());
    
    assert_eq!(cpu.registers.a, 0x00);
    assert_eq!(cpu.registers.zero(), true);   // This might fail
    assert_eq!(cpu.registers.carry(), true);  // Or this
}
```

Run with: `cargo test test_add_with_carry -- --nocapture`

### Strategy 2: Compare with Reference

If you have a working emulator (or real hardware trace):

```rust
#[test]
fn test_matches_reference() {
    let mut cpu = Cpu::new();
    // Setup state...
    
    cpu.execute(0x80);
    
    // Compare with known-good values
    assert_eq!(cpu.registers.a, 0x15, "A register mismatch");
    assert_eq!(cpu.registers.f, 0b0000_0000, "Flags mismatch");
}
```

### Strategy 3: Step-by-Step Trace

```rust
#[test]
fn test_with_trace() {
    let mut cpu = Cpu::new();
    cpu.enable_trace();  // Log every instruction
    
    let rom = vec![/* program */];
    cpu.mmu.load_rom_from_bytes(rom);
    cpu.registers.pc = 0;
    
    for i in 0..10 {
        println!("Step {}: PC={:04X}", i, cpu.registers.pc);
        cpu.step();
    }
}
```

### Strategy 4: Minimal Reproduction

Create the smallest test that reproduces the bug:

```rust
#[test]
fn minimal_bug_repro() {
    let mut cpu = Cpu::new();
    cpu.registers.a = 0x0F;  // Specific value that triggers bug
    
    cpu.execute(0x3C);  // INC A
    
    // Expected: half_carry = true (carry from bit 3 to 4)
    assert_eq!(cpu.registers.half_carry(), true, "Half carry not set!");
}
```

---

## Coverage Goals

### Phase 1: CPU Foundation
- **Target**: 90%+ coverage of CPU and register code
- Every instruction implemented has 3+ tests
- All flag operations tested
- Edge cases covered (overflow, underflow, zero)

### Phase 2: Memory Management
- **Target**: 85%+ coverage of MMU code
- All memory regions tested
- Banking logic tested
- ROM loading tested

### Phase 3: Complete CPU
- **Target**: Pass Blargg's CPU tests
- All 256 opcodes tested
- All CB-prefixed opcodes tested
- Complex instruction sequences tested

### Measuring Coverage

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Run coverage
cargo tarpaulin --out Html --output-dir coverage

# Open coverage/index.html to see results
```

---

## Best Practices

### 1. Test Names Should Be Descriptive

```rust
// GOOD
#[test]
fn test_inc_a_sets_zero_flag_on_overflow() { /* ... */ }

// BAD
#[test]
fn test_inc_1() { /* ... */ }
```

### 2. One Assertion Per Concept

```rust
// GOOD - Clear what failed if it fails
#[test]
fn test_add_result() {
    // ... setup ...
    assert_eq!(cpu.registers.a, 0x15);
}

#[test]
fn test_add_zero_flag() {
    // ... setup ...
    assert_eq!(cpu.registers.zero(), false);
}

// ACCEPTABLE - Related assertions
#[test]
fn test_add_flags() {
    // ... setup ...
    assert_eq!(cpu.registers.zero(), false, "Zero flag incorrect");
    assert_eq!(cpu.registers.carry(), false, "Carry flag incorrect");
}
```

### 3. Use Setup Functions for Common Patterns

```rust
fn setup_cpu_with_rom(rom: Vec<u8>) -> Cpu {
    let mut cpu = Cpu::new();
    cpu.mmu.load_rom_from_bytes(rom);
    cpu.reset();
    cpu
}

#[test]
fn test_something() {
    let mut cpu = setup_cpu_with_rom(vec![0x00, 0x3E, 0x42]);
    // ...
}
```

### 4. Test Both Success and Failure

```rust
#[test]
fn test_load_valid_rom() {
    let mut mmu = Mmu::new();
    let rom = vec![0; 0x8000];
    
    mmu.load_rom_from_bytes(rom);
    
    assert_eq!(mmu.read(0x0000), 0);
}

#[test]
#[should_panic(expected = "ROM too small")]
fn test_load_invalid_rom() {
    let mut mmu = Mmu::new();
    let rom = vec![0; 100];  // Too small
    
    mmu.load_rom_from_bytes(rom);  // Should panic
}
```

### 5. Keep Tests Fast

```rust
// GOOD - Fast unit test
#[test]
fn test_add() {
    let mut cpu = Cpu::new();
    cpu.registers.a = 5;
    cpu.registers.b = 3;
    cpu.execute(0x80);
    assert_eq!(cpu.registers.a, 8);
}

// SLOW - Mark as ignored
#[test]
#[ignore]
fn test_full_game_boot() {
    // This takes 10 seconds
    let mut cpu = Cpu::new();
    cpu.mmu.load_rom("tetris.gb").unwrap();
    for _ in 0..1_000_000 {
        cpu.step();
    }
}
```

Run fast tests: `cargo test`
Run all tests: `cargo test -- --ignored`

---

## Summary

**TDD Workflow:**
1. Write test first (RED)
2. Implement minimal code (GREEN)
3. Refactor (REFACTOR)
4. Repeat

**Test Types:**
- Unit tests: Fast, isolated, in source files
- Integration tests: Slower, multi-component, in `tests/`
- Test ROMs: Gold standard, used for final validation

**Coverage Goals:**
- Every instruction has 3+ tests
- Edge cases and flags tested
- Aim for 85%+ code coverage
- Pass community test ROMs

**When Stuck:**
- Add print statements
- Create minimal reproduction
- Compare with reference
- Step through with debugger

**Remember:** Tests are your safety net. Write them first, trust them, and use them to guide refactoring!
