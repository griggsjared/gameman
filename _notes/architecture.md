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

The Game Boy Color (GBC) is a **natural extension** of your DMG emulator, not a rewrite. The core architecture you're building now will support GBC with approximately 20-30% additional work. This section explains what changes and how your DMG foundation evolves.

#### Key Principle: Extension, Not Replacement

GBC maintains **full backward compatibility** with DMG. This means:
- Same CPU (Sharp LR35902)
- Same instruction set (all 512 opcodes)
- Same basic memory map structure
- Same rendering concepts (tiles, sprites, etc.)

Your emulator will run in one of two modes:
```rust
enum EmulatorMode {
    Dmg,  // Original Game Boy
    Cgb,  // Game Boy Color
}
```

Most of your code becomes:
```rust
match self.mode {
    EmulatorMode::Dmg => { /* Your existing DMG logic */ }
    EmulatorMode::Cgb => { /* Extended GBC logic */ }
}
```

---

#### What Changes: Component by Component

##### **CPU & Registers: Zero Changes** ✅

The CPU is identical. Your Phase 1 register implementation works for both:
- Same 8-bit registers (A, F, B, C, D, E, H, L)
- Same 16-bit registers (SP, PC)
- Same flags (Z, N, H, C)
- Same instruction execution

**New feature:** CPU speed switching
```rust
// Add to your CPU or system struct
enum CpuSpeed {
    Normal,  // 4.194 MHz (DMG speed)
    Double,  // 8.388 MHz (GBC only)
}

// New I/O register: 0xFF4D (KEY1) - Speed switch
// Writing 0x01 and executing STOP switches speed
```

**Impact:** Minimal. Add speed mode enum, multiply cycle counts when in double speed.

---

##### **MMU: Bank Extensions**

###### **Work RAM (WRAM) Banking**

**DMG:**
```rust
struct Mmu {
    wram: [u8; 0x2000],  // 8KB fixed
}

impl Mmu {
    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0xC000..=0xDFFF => self.wram[(addr - 0xC000) as usize],
            // ...
        }
    }
}
```

**GBC Extension:**
```rust
struct Mmu {
    wram: [[u8; 0x1000]; 8],  // 8 banks of 4KB = 32KB
    wram_bank: u8,             // Current bank (1-7), bank 0 is fixed
}

impl Mmu {
    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0xC000..=0xCFFF => {
                // Bank 0 (fixed)
                self.wram[0][(addr - 0xC000) as usize]
            }
            0xD000..=0xDFFF => {
                // Switchable bank (1-7)
                let bank = if self.mode == EmulatorMode::Cgb {
                    self.wram_bank.max(1) // Banks 1-7
                } else {
                    1 // DMG only has bank 1
                };
                self.wram[bank as usize][(addr - 0xD000) as usize]
            }
            // New register: 0xFF70 (SVBK) - WRAM bank select
            0xFF70 => self.wram_bank,
            // ...
        }
    }
}
```

###### **Video RAM (VRAM) Banking**

**DMG:**
```rust
vram: [u8; 0x2000],  // 8KB single bank
```

**GBC Extension:**
```rust
vram: [[u8; 0x2000]; 2],  // 2 banks of 8KB = 16KB
vram_bank: u8,             // 0 or 1

// New register: 0xFF4F (VBK) - VRAM bank select
// Bank 0: Tile data (same as DMG)
// Bank 1: Tile attributes (GBC only)
```

**Impact:** Moderate. Replace single arrays with banked arrays, add bank selection registers.

---

##### **PPU: Color Palettes and Attributes**

This is the most visible change, but follows DMG concepts closely.

###### **Color System**

**DMG:** 4 shades of gray per palette
```rust
type Color = u8;  // 0-3 (grayscale)

fn lookup_palette(&self, palette_index: u8) -> Color {
    // Convert 2-bit index to grayscale
    match palette_index {
        0 => 255,  // White
        1 => 192,  // Light gray
        2 => 96,   // Dark gray
        3 => 0,    // Black
    }
}
```

**GBC:** 32,768 colors (15-bit RGB)
```rust
#[derive(Copy, Clone)]
struct Rgb {
    r: u8,
    g: u8,
    b: u8,
}

type Color = Rgb;

struct Ppu {
    bg_palette_ram: [u8; 64],   // 8 palettes × 4 colors × 2 bytes
    obj_palette_ram: [u8; 64],  // 8 palettes × 4 colors × 2 bytes
    // ...
}

fn lookup_palette(&self, palette_index: u8, palette_num: u8, is_bg: bool) -> Color {
    let palette_ram = if is_bg { &self.bg_palette_ram } else { &self.obj_palette_ram };
    let offset = (palette_num * 8) + (palette_index * 2);
    
    let lo = palette_ram[offset as usize];
    let hi = palette_ram[(offset + 1) as usize];
    let bgr555 = (hi as u16) << 8 | lo as u16;
    
    // Convert BGR555 to RGB888
    let r = ((bgr555 & 0x001F) << 3) as u8;
    let g = (((bgr555 & 0x03E0) >> 5) << 3) as u8;
    let b = (((bgr555 & 0x7C00) >> 10) << 3) as u8;
    
    Rgb { r, g, b }
}

// New registers:
// 0xFF68 (BCPS): Background palette index
// 0xFF69 (BCPD): Background palette data
// 0xFF6A (OCPS): Object palette index
// 0xFF6B (OCPD): Object palette data
```

###### **Tile Attributes (VRAM Bank 1)**

DMG tiles have no attributes. GBC adds per-tile attributes stored in VRAM bank 1:

```rust
// When rendering a tile in GBC mode
let tile_data = self.vram[0][tile_addr];       // Bank 0: tile pixels
let tile_attr = self.vram[1][tile_addr];       // Bank 1: attributes

// Attribute byte format:
// Bit 7: Priority (BG-to-OAM priority)
// Bit 6: Y flip
// Bit 5: X flip
// Bit 4: VRAM bank (for tile data)
// Bit 3: Palette bank (0 = BCPx 0-7, 1 = BCPx 8-15)
// Bit 2-0: Color palette number (0-7)

let priority = tile_attr & 0x80 != 0;
let y_flip = tile_attr & 0x40 != 0;
let x_flip = tile_attr & 0x20 != 0;
let tile_vram_bank = (tile_attr & 0x10) >> 4;
let palette_num = tile_attr & 0x07;
```

**Impact:** Significant for PPU, but extends existing rendering logic. DMG path remains unchanged.

---

##### **Timing: Double Speed Mode**

**Implementation:**
```rust
struct Cpu {
    speed_mode: CpuSpeed,
    speed_switch_armed: bool,  // KEY1 bit 0
}

fn get_cpu_frequency(&self) -> u32 {
    match self.speed_mode {
        CpuSpeed::Normal => 4_194_304,
        CpuSpeed::Double => 8_388_608,
    }
}

// When STOP instruction executed with speed_switch_armed = true
fn execute_stop(&mut self) {
    if self.speed_switch_armed {
        self.speed_mode = match self.speed_mode {
            CpuSpeed::Normal => CpuSpeed::Double,
            CpuSpeed::Double => CpuSpeed::Normal,
        };
        self.speed_switch_armed = false;
    }
}
```

**Impact:** Minor. Add speed mode tracking, adjust timing calculations.

---

##### **New Registers Summary**

| Address | Name | Purpose |
|---------|------|---------|
| 0xFF4D | KEY1 | CPU speed switch |
| 0xFF4F | VBK | VRAM bank select (0-1) |
| 0xFF68 | BCPS | Background palette index |
| 0xFF69 | BCPD | Background palette data |
| 0xFF6A | OCPS | Object palette index |
| 0xFF6B | OCPD | Object palette data |
| 0xFF70 | SVBK | WRAM bank select (0-7) |

**Impact:** Add ~7 new I/O register handlers to MMU.

---

#### Additional GBC Features

##### **HDMA (HBlank DMA)**

Allows copying data during HBlank for smoother updates:
```rust
// Registers: 0xFF51-0xFF55
struct Hdma {
    source: u16,
    destination: u16,
    length: u16,
    mode: HdmaMode,  // General purpose or HBlank
}

// During HBlank, copy 0x10 bytes
fn hblank_dma_transfer(&mut self) {
    if self.hdma.active {
        for _ in 0..0x10 {
            let byte = self.read(self.hdma.source);
            self.write(self.hdma.destination, byte);
            self.hdma.source += 1;
            self.hdma.destination += 1;
        }
    }
}
```

##### **Infrared Port**

Optional communication feature:
```rust
// Register: 0xFF56 (RP) - Infrared port
// Can be stubbed or fully implemented
```

---

#### Backward Compatibility: Running DMG Games on GBC

GBC hardware can detect DMG cartridges and run in compatibility mode:

```rust
fn detect_mode(cartridge: &Cartridge) -> EmulatorMode {
    // Check cartridge header byte 0x0143
    match cartridge.cgb_flag {
        0x80 => EmulatorMode::Cgb,  // GBC enhanced (supports both)
        0xC0 => EmulatorMode::Cgb,  // GBC only
        _ => EmulatorMode::Dmg,     // DMG only
    }
}

// When running in DMG mode on GBC hardware:
// - Use DMG memory sizes (no banking)
// - Use DMG grayscale palettes
// - Run at normal speed only
// - Ignore GBC-specific registers
```

---

#### Implementation Strategy

##### **Phase 1-7: Build Complete DMG Emulator**

Focus entirely on DMG. Don't add GBC code yet.

**Design choices that help:**
```rust
// Good: Use type alias for color
type Color = u8;  // Easy to change to Rgb later

// Good: Separate palette lookup
fn get_pixel_color(&self, palette_idx: u8) -> Color;

// Good: Use enums for clarity
enum VramBank { Bank0, Bank1 }
```

##### **Post-Phase 7: Add GBC Support**

**Step 1: Add mode enum**
```rust
enum EmulatorMode { Dmg, Cgb }
```

**Step 2: Extend memory**
```rust
// Before: wram: [u8; 0x2000]
// After:  wram: [[u8; 0x1000]; 8]
```

**Step 3: Add color palettes**
```rust
struct Ppu {
    bg_palette_ram: [u8; 64],
    obj_palette_ram: [u8; 64],
}
```

**Step 4: Update rendering**
```rust
match self.mode {
    EmulatorMode::Dmg => { /* existing grayscale logic */ }
    EmulatorMode::Cgb => { /* new color logic */ }
}
```

**Step 5: Add new I/O registers**

**Step 6: Test with GBC ROMs**

---

#### Code Organization for GBC

```rust
// Keep DMG and GBC logic separate
mod ppu {
    mod dmg_render;  // Original DMG rendering
    mod cgb_render;  // GBC color rendering
    
    pub fn render_scanline(&mut self) {
        match self.mode {
            EmulatorMode::Dmg => dmg_render::render_scanline(self),
            EmulatorMode::Cgb => cgb_render::render_scanline(self),
        }
    }
}
```

---

#### Testing GBC

**Test ROMs:**
- DMG games on GBC emulator (backward compatibility)
- GBC-only games
- GBC-enhanced games (support both modes)

**Key test cases:**
- WRAM banking (write to bank 3, switch to bank 5, verify independence)
- VRAM banking (tile data vs attributes)
- Color palette accuracy (compare screenshots)
- Speed switching (timing-sensitive games)

---

#### Effort Estimate

Based on typical emulator development:

| Component | DMG Effort | GBC Additional | % Increase |
|-----------|------------|----------------|------------|
| CPU | 100% | 0% | +0% |
| Registers | 100% | 0% | +0% |
| MMU | 100% | 15% | +15% |
| PPU | 100% | 40% | +40% |
| Timing | 100% | 10% | +10% |
| **Overall** | **100%** | **~25%** | **+25%** |

**Timeline:** If DMG takes 8 weeks, GBC adds approximately 2 weeks.

---

#### When NOT to Add GBC

Don't add GBC support if:
- Your DMG emulator isn't passing Blargg's test ROMs
- DMG graphics aren't rendering correctly
- You haven't completed Phase 7
- You're still learning basic emulation concepts

**Reason:** GBC adds complexity. Master DMG first, then extend.

---

#### Summary: DMG to GBC Evolution

**What stays the same:**
- ✅ CPU architecture
- ✅ Instruction set
- ✅ Basic memory map
- ✅ Rendering concepts
- ✅ Timing concepts

**What extends:**
- 🔄 Memory sizes (banks added)
- 🔄 Color system (grayscale → RGB)
- 🔄 PPU attributes (none → per-tile)
- 🔄 Speed modes (fixed → switchable)

**What's new:**
- ✨ Palette RAM
- ✨ HDMA transfers
- ✨ Mode detection
- ✨ ~7 new registers

**Architecture impact:** Your well-structured DMG emulator naturally extends to GBC. Think of it as adding features, not rebuilding.

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
