# Architecture Overview

This document outlines the high-level architecture of the Gameman emulator, design principles, and how the major subsystems interact.

## Table of Contents

1. [System Overview](#system-overview)
2. [Core Components](#core-components)
3. [Data Flow](#data-flow)
4. [Design Principles](#design-principles)
5. [Module Organization](#module-organization)
6. [Key Design Decisions](#key-design-decisions)

---

## System Overview

The Gameboy emulator consists of several interconnected subsystems that work together to simulate the original hardware:

```
┌─────────────────────────────────────────────────┐
│                   Emulator                      │
│  ┌──────────┐     ┌──────────┐    ┌─────────┐  │
│  │   CPU    │────▶│   MMU    │◀──▶│   ROM   │  │
│  │ (LR35902)│     │(Memory)  │    │         │  │
│  └────┬─────┘     └─────┬────┘    └─────────┘  │
│       │                 │                       │
│       │                 ▼                       │
│       │          ┌──────────┐                   │
│       └─────────▶│   PPU    │──────▶ Display   │
│                  │(Graphics)│                   │
│                  └──────────┘                   │
│       ┌──────────────┴─────────────┐            │
│       │                            │            │
│  ┌────▼────┐  ┌──────────┐  ┌────▼──────┐      │
│  │ Timers  │  │   APU    │  │ Interrupts│      │
│  └─────────┘  │ (Audio)  │  └───────────┘      │
│               └──────────┘                      │
│  ┌──────────┐                                   │
│  │  Joypad  │◀──── Input                        │
│  └──────────┘                                   │
└─────────────────────────────────────────────────┘
```

### Hardware Emulated

- **CPU**: Sharp LR35902 (8-bit, Z80-like)
- **Memory**: 64KB address space with banking
- **Graphics**: 160x144 LCD with tile-based rendering
- **Sound**: 4-channel APU
- **Input**: 8-button joypad
- **Timing**: ~4.19 MHz clock

---

## Core Components

### 1. CPU (Central Processing Unit)

**Location**: `src/cpu/`

**Responsibilities:**
- Execute instructions (fetch-decode-execute cycle)
- Manage registers (A, F, B, C, D, E, H, L, SP, PC)
- Handle flags (Zero, Subtract, Half Carry, Carry)
- Track cycle timing
- Process interrupts

**Key Files:**
- `mod.rs` - CPU struct and main execution logic
- `registers.rs` - Register implementation and flag operations
- `opcodes.rs` - Instruction implementations (Phase 3+)
- `decode.rs` - Instruction decoding logic (Phase 3+)

**Public Interface:**
```rust
impl Cpu {
    pub fn new() -> Self;
    pub fn step(&mut self) -> u8;  // Execute one instruction, return cycles
    pub fn reset(&mut self);
}
```

---

### 2. MMU (Memory Management Unit)

**Location**: `src/mmu/`

**Responsibilities:**
- Route memory reads/writes to appropriate hardware
- Manage ROM banking (MBC1/MBC3/MBC5)
- Provide VRAM, WRAM, HRAM access
- Handle memory-mapped I/O
- Enforce access restrictions

**Key Files:**
- `mod.rs` - MMU struct and memory routing
- `cartridge.rs` - ROM/RAM banking logic (Phase 3+)

**Memory Map:**
```
0x0000-0x3FFF: ROM Bank 0
0x4000-0x7FFF: ROM Bank 1-N (switchable)
0x8000-0x9FFF: VRAM
0xA000-0xBFFF: External RAM
0xC000-0xDFFF: WRAM
0xFE00-0xFE9F: OAM
0xFF00-0xFF7F: I/O Registers
0xFF80-0xFFFE: HRAM
0xFFFF: Interrupt Enable
```

**Public Interface:**
```rust
impl Mmu {
    pub fn new() -> Self;
    pub fn read(&self, addr: u16) -> u8;
    pub fn write(&mut self, addr: u16, value: u8);
    pub fn load_rom<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()>;
}
```

---

### 3. PPU (Picture Processing Unit)

**Location**: `src/ppu/`

**Responsibilities:**
- Render background layer
- Render window layer
- Render sprites (OAM)
- Manage LCD modes (HBlank, VBlank, OAM Search, Drawing)
- Generate VBlank interrupt
- Output 160x144 pixel framebuffer

**Key Files:**
- `mod.rs` - PPU state machine and LCD control
- `tiles.rs` - Tile decoding and rendering
- `sprites.rs` - Sprite (OAM) handling

**LCD Modes:**
```
Mode 0: HBlank  (204 cycles)
Mode 1: VBlank  (4560 cycles, 10 scanlines)
Mode 2: OAM     (80 cycles)
Mode 3: Drawing (172 cycles)

One scanline: Mode 2 → Mode 3 → Mode 0 (456 cycles)
One frame: 144 scanlines + 10 VBlank lines (70224 cycles)
```

**Public Interface:**
```rust
impl Ppu {
    pub fn new() -> Self;
    pub fn step(&mut self, cycles: u8) -> bool;  // Returns true on VBlank
    pub fn get_framebuffer(&self) -> &[u8];
}
```

---

### 4. Timer

**Location**: `src/timer.rs`

**Responsibilities:**
- Increment DIV register at 16384 Hz
- Increment TIMA register at configurable frequency
- Generate timer interrupt on TIMA overflow

**Registers:**
- `0xFF04`: DIV - Divider (increments automatically)
- `0xFF05`: TIMA - Timer counter
- `0xFF06`: TMA - Timer modulo (reload value)
- `0xFF07`: TAC - Timer control

---

### 5. Interrupts

**Location**: `src/interrupts.rs`

**Responsibilities:**
- Manage interrupt enable (IE) and interrupt flags (IF)
- Handle interrupt priority
- Execute interrupt service routines

**Interrupt Types (Priority Order):**
1. VBlank (0x40)
2. LCD STAT (0x48)
3. Timer (0x50)
4. Serial (0x58)
5. Joypad (0x60)

**Flow:**
```
1. Interrupt occurs → Set bit in IF (0xFF0F)
2. Check IE (0xFFFF) - is this interrupt enabled?
3. Check IME (Interrupt Master Enable) - are interrupts globally enabled?
4. If yes: Push PC, jump to handler, clear IME
```

---

### 6. Joypad

**Location**: `src/joypad.rs`

**Responsibilities:**
- Track button states (Up, Down, Left, Right, A, B, Start, Select)
- Handle P1 register (0xFF00) reads/writes
- Generate joypad interrupt on button press

---

### 7. APU (Audio Processing Unit)

**Location**: `src/apu/` (Phase 8)

**Responsibilities:**
- Generate 4 audio channels
- Mix audio output
- Manage sound registers

**Channels:**
1. Pulse with sweep
2. Pulse
3. Wave
4. Noise

---

## Data Flow

### Fetch-Decode-Execute Cycle

```
1. CPU fetches opcode from MMU
   ├─▶ MMU.read(PC)
   └─▶ PC += 1

2. CPU decodes opcode
   ├─▶ Match on opcode value
   └─▶ Determine operation and operands

3. CPU executes instruction
   ├─▶ Read operands from MMU if needed
   ├─▶ Perform operation
   ├─▶ Update registers/flags
   ├─▶ Write results to MMU if needed
   └─▶ Return cycle count

4. Update subsystems
   ├─▶ PPU.step(cycles)
   ├─▶ Timer.step(cycles)
   └─▶ Check for interrupts

5. Handle interrupts if pending
   ├─▶ Check IF & IE & IME
   ├─▶ Push PC to stack
   ├─▶ Jump to interrupt handler
   └─▶ Clear IME

6. Repeat
```

### Frame Rendering Flow

```
1. CPU executes instructions
   └─▶ Accumulates cycles

2. PPU.step(cycles)
   ├─▶ Mode 2: OAM Search (80 cycles)
   ├─▶ Mode 3: Drawing (172 cycles)
   │   ├─▶ Read tile data from VRAM
   │   ├─▶ Render background
   │   ├─▶ Render window
   │   └─▶ Render sprites
   ├─▶ Mode 0: HBlank (204 cycles)
   └─▶ Repeat for 144 scanlines

3. After line 144: Enter VBlank
   ├─▶ Mode 1: VBlank (4560 cycles)
   ├─▶ Set VBlank interrupt
   └─▶ CPU can safely access VRAM

4. Display framebuffer
   └─▶ Send to frontend (SDL, terminal, etc.)

5. Repeat at ~59.7 FPS
```

---

## Design Principles

### 1. Modularity

Each subsystem should be:
- **Self-contained**: Minimal dependencies on other modules
- **Testable**: Can be unit tested in isolation
- **Replaceable**: Could swap implementations without breaking others

Example: PPU doesn't directly access CPU registers, only communicates via MMU and interrupts.

### 2. Separation of Concerns

```
CPU: Execution logic only
MMU: Memory routing only
PPU: Rendering logic only
```

Don't mix concerns. For example:
- CPU doesn't know about pixel rendering
- PPU doesn't execute instructions
- MMU doesn't generate audio

### 3. Test-Driven Development

- Write tests before implementation
- Every public function should have tests
- Test edge cases and boundary conditions
- Use integration tests for subsystem interaction

### 4. Progressive Enhancement

Start simple, add complexity gradually:
- Phase 1: Basic CPU + registers
- Phase 2: Add memory
- Phase 3: Complete CPU
- Phase 4: Add graphics
- ...

Don't try to build everything at once.

### 5. Accuracy vs Performance

Priority: **Accuracy first, optimize later**

- Start with cycle-accurate timing
- Profile before optimizing
- Document any accuracy tradeoffs
- Keep accurate reference implementation

### 6. Clear Interfaces

Use clear, documented public APIs:

```rust
// GOOD - Clear what this does
pub fn execute_instruction(&mut self) -> u8 { /* ... */ }

// BAD - Unclear what 'tick' means
pub fn tick(&mut self) -> u8 { /* ... */ }
```

---

## Module Organization

### Recommended Structure

```
gameman/
├── src/
│   ├── main.rs              # Entry point
│   ├── lib.rs               # Library entry (optional)
│   │
│   ├── cpu/
│   │   ├── mod.rs           # CPU struct, execute loop
│   │   ├── registers.rs     # Register implementation
│   │   ├── opcodes.rs       # Opcode implementations
│   │   └── decode.rs        # Instruction decoder
│   │
│   ├── mmu/
│   │   ├── mod.rs           # Memory routing
│   │   └── cartridge.rs     # ROM/RAM banking (MBC)
│   │
│   ├── ppu/
│   │   ├── mod.rs           # PPU state machine
│   │   ├── tiles.rs         # Tile rendering
│   │   └── sprites.rs       # Sprite (OAM) handling
│   │
│   ├── apu/
│   │   ├── mod.rs           # APU main
│   │   ├── channel1.rs      # Pulse with sweep
│   │   ├── channel2.rs      # Pulse
│   │   ├── channel3.rs      # Wave
│   │   └── channel4.rs      # Noise
│   │
│   ├── timer.rs             # Timer implementation
│   ├── interrupts.rs        # Interrupt controller
│   └── joypad.rs            # Input handling
│
├── tests/
│   ├── cpu_tests.rs         # CPU integration tests
│   ├── ppu_tests.rs         # PPU integration tests
│   └── blargg/              # Test ROM results
│
├── roms/                    # Test ROMs and games
│   ├── test/                # Test ROMs (Blargg, etc.)
│   └── games/               # Actual games
│
└── _notes/                  # This documentation!
```

### Module Dependencies

```
main.rs
  └─▶ cpu::Cpu
       ├─▶ cpu::Registers
       ├─▶ mmu::Mmu
       │    └─▶ mmu::Cartridge
       ├─▶ ppu::Ppu
       │    ├─▶ ppu::Tiles
       │    └─▶ ppu::Sprites
       ├─▶ timer::Timer
       ├─▶ interrupts::InterruptController
       └─▶ joypad::Joypad
```

**Key Rule**: Avoid circular dependencies. MMU should not import CPU.

---

## Key Design Decisions

### Decision 1: Struct vs Trait for MMU

**Choice**: Struct-based MMU (not trait-based)

**Rationale:**
- Simpler for single emulator implementation
- No virtual dispatch overhead
- Easier for beginners to understand
- Can refactor to trait later if needed

**Alternative**: Trait-based would allow mock MMU for testing, but adds complexity.

---

### Decision 2: Cycle Timing Approach

**Choice**: Instruction-level cycle counting

**Implementation:**
```rust
pub fn step(&mut self) -> u8 {
    let opcode = self.fetch_byte();
    let cycles = self.execute(opcode);
    
    self.ppu.step(cycles);
    self.timer.step(cycles);
    
    cycles
}
```

**Rationale:**
- Accurate enough for most games
- Simpler than T-cycle accuracy
- Easier to implement initially

**Alternative**: T-cycle accuracy (counting individual machine cycles) is more accurate but much more complex.

---

### Decision 3: Register Storage

**Choice**: Individual fields (not array-based)

```rust
pub struct Registers {
    pub a: u8,
    pub f: u8,
    pub b: u8,
    // ...
}
```

**Rationale:**
- More readable
- Easier to debug
- Natural Rust idioms
- Clear field names

**Alternative**: Array-based is more compact but uses magic indices.

---

### Decision 4: Instruction Implementation

**Phase 1-2**: Separate method per instruction
```rust
fn ld_a_b(&mut self) -> u8 { /* ... */ }
fn ld_a_c(&mut self) -> u8 { /* ... */ }
```

**Phase 3+**: Refactor to generic helpers
```rust
fn ld_r_r(&mut self, dst: Register, src: Register) -> u8 { /* ... */ }
```

**Rationale:**
- Start explicit for clarity
- Refactor when patterns emerge
- Avoid premature abstraction

---

### Decision 5: Error Handling

**Choice**: Panic on unimplemented opcodes (early phases), graceful handling later

```rust
// Phase 1-2
_ => panic!("Unimplemented opcode: 0x{:02X}", opcode),

// Phase 3+
_ => {
    eprintln!("Unknown opcode: 0x{:02X} at PC: 0x{:04X}", opcode, self.registers.pc);
    0  // NOP
}
```

**Rationale:**
- Early: Fail fast to catch bugs
- Later: Graceful degradation for compatibility

---

### Decision 6: Graphics Output

**Phase 4**: Start with simple output (file, terminal)
```rust
// Write framebuffer to PPM file
pub fn save_frame(&self, path: &str) { /* ... */ }
```

**Phase 7**: Add real-time rendering (SDL2)
```rust
// Render to SDL2 window
pub fn render(&self, canvas: &mut Canvas<Window>) { /* ... */ }
```

**Rationale:**
- Start simple to focus on PPU logic
- Add complexity (SDL, input) later
- Can test rendering without GUI

---

## Communication Between Subsystems

### CPU ↔ MMU

```rust
// CPU reads/writes through MMU
let value = self.mmu.read(addr);
self.mmu.write(addr, value);
```

**Direction**: CPU owns MMU, calls read/write

---

### CPU ↔ PPU

```rust
// CPU advances PPU by cycle count
self.ppu.step(cycles);

// PPU signals VBlank via interrupt
if self.ppu.is_vblank() {
    self.interrupts.request(Interrupt::VBlank);
}
```

**Direction**: CPU owns PPU, drives timing. PPU requests interrupts.

---

### CPU ↔ Interrupts

```rust
// Check for pending interrupts
if let Some(interrupt) = self.interrupts.pending() {
    self.handle_interrupt(interrupt);
}
```

**Direction**: CPU polls interrupts, handles when ready

---

### MMU ↔ PPU

```rust
// MMU routes VRAM reads to PPU (or internal VRAM)
// PPU can access VRAM directly or through MMU
match addr {
    0x8000..=0x9FFF => self.vram[(addr - 0x8000) as usize],
}
```

**Direction**: Shared VRAM access, MMU arbitrates

---

## Performance Considerations

### Optimization Strategy

1. **Don't optimize prematurely**: Get it working first
2. **Profile before optimizing**: Use `cargo flamegraph` or `perf`
3. **Optimize hot paths**: CPU execute loop, PPU rendering
4. **Cache when possible**: Decoded instructions, tile data

### Common Optimizations (Later Phases)

**Instruction Dispatch:**
```rust
// Instead of giant match
match opcode { /* 256 arms */ }

// Use lookup table
let handler = OPCODE_TABLE[opcode as usize];
handler(self)
```

**PPU Rendering:**
```rust
// Instead of decoding tiles every frame
// Cache decoded tiles when VRAM changes
if self.vram_dirty {
    self.decode_tiles();
    self.vram_dirty = false;
}
```

---

## Testing Architecture

### Unit Tests

Each module has its own tests:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    // Test individual methods
}
```

### Integration Tests

Test subsystem interaction:
```rust
// tests/cpu_tests.rs
#[test]
fn test_cpu_executes_from_rom() {
    let mut cpu = Cpu::new();
    cpu.mmu.load_rom_from_bytes(rom_data);
    cpu.step();
    assert_eq!(cpu.registers.a, expected_value);
}
```

### Test ROMs

Use community test ROMs:
- Blargg's CPU instruction tests
- Blargg's mem timing tests
- dmg-acid2 (graphics test)
- Mooneye Test Suite

---

## Future Expansion

### Gameboy Color Support

Add to later phases:
- Double CPU speed mode
- Color palettes
- Additional WRAM banks
- HDMA (DMA during HBlank)

### Save States

```rust
pub fn save_state(&self) -> State { /* ... */ }
pub fn load_state(&mut self, state: State) { /* ... */ }
```

### Debugging Tools

```rust
pub struct Debugger {
    pub breakpoints: Vec<u16>,
    pub watchpoints: Vec<u16>,
    pub step_mode: bool,
}
```

---

This architecture provides a solid foundation for building a complete, maintainable Gameboy emulator. Follow the phase guides to implement each subsystem incrementally!
