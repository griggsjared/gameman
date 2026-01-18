# Gameman - Gameboy Emulator Development Guide

Welcome to your Gameboy emulator learning journey! This documentation is designed to guide you through building a complete Gameboy emulator from scratch in Rust, using Test-Driven Development (TDD) and a gradual, educational approach.

## Philosophy

This project is about **learning by doing**. Rather than diving into a complete emulator implementation, we'll build each subsystem independently, understand how it works, test it thoroughly, and then integrate it with other components. Each phase is designed to be achievable and educational.

## Project Goals

- Build a functional Gameboy (DMG) emulator
- Understand CPU emulation, memory management, and hardware simulation
- Practice Test-Driven Development in Rust
- Create a well-architected, maintainable codebase
- Learn about computer architecture through hands-on implementation

## Documentation Structure

This guide is organized into the following documents:

### Core Planning Documents

1. **[Architecture Overview](./architecture.md)** - High-level system design, module organization, and design principles
2. **[Testing Strategy](./testing-strategy.md)** - TDD workflow, testing patterns, and best practices
3. **[Resources & References](./resources.md)** - External documentation, test ROMs, and learning materials

### Implementation Phases

Each phase builds on the previous ones and includes:
- Hardware background and theory
- Multiple implementation approaches with pros/cons
- Complete code examples
- TDD test cases
- Milestones and checkpoints

#### **[Phase 1: CPU Foundation](./phase1-cpu-foundation.md)** ⭐ START HERE
The heart of the emulator. Build the basic CPU structure, registers, and implement your first instructions.

**Milestones:**
- [ ] CPU struct with 8-bit registers
- [ ] Flag register with bit manipulation
- [ ] Program counter and stack pointer
- [ ] Basic instruction execution (NOP, LD, ADD, INC, DEC)
- [ ] Simple instruction decoder
- [ ] 20+ unit tests passing

**Key Concepts:** Register design, instruction decoding, flag operations, TDD workflow

---

#### **[Phase 2: Memory Management](./phase2-memory-management.md)**
Implement the Memory Management Unit (MMU) to handle memory mapping, ROM loading, and I/O.

**Milestones:**
- [ ] Memory map structure (64KB address space)
- [ ] ROM loading from file
- [ ] Memory read/write operations
- [ ] Banking basics (MBC1 preparation)
- [ ] Memory-mapped I/O stub

**Key Concepts:** Memory mapping, address spaces, banking, ROM formats

---

#### **[Phase 3: Complete CPU](./phase3-complete-cpu.md)**
Finish all 256 main opcodes plus 256 CB-prefixed instructions. This is the biggest phase.

**Milestones:**
- [ ] All arithmetic/logic instructions
- [ ] All load/store variants
- [ ] All jump/call/return instructions
- [ ] CB-prefixed bit operations
- [ ] Interrupt handling basics
- [ ] Pass Blargg's CPU instruction tests

**Key Concepts:** Complete instruction set, cycle timing, interrupts

---

#### **[Phase 4: Graphics Basics](./phase4-graphics-basics.md)**
Implement the Picture Processing Unit (PPU) for rendering tiles and backgrounds.

**Milestones:**
- [ ] PPU state machine
- [ ] VRAM access
- [ ] Tile data decoding
- [ ] Background layer rendering
- [ ] Simple framebuffer output

**Key Concepts:** Tile-based graphics, color palettes, scanline rendering

---

#### **[Phase 5: Integration & Timing](./phase5-integration.md)**
Connect all subsystems with accurate timing, interrupts, and timers.

**Milestones:**
- [ ] CPU cycle counting
- [ ] Timer registers
- [ ] Interrupt system (VBlank, LCD, Timer)
- [ ] Synchronized PPU/CPU execution
- [ ] Accurate frame timing

**Key Concepts:** Cycle accuracy, interrupt priorities, system synchronization

---

#### **[Phase 6: Advanced Graphics](./phase6-advanced-graphics.md)**
Add sprites (OAM), window layer, and scrolling for complete graphics support.

**Milestones:**
- [ ] Sprite rendering (OAM)
- [ ] Sprite priorities and attributes
- [ ] Window layer
- [ ] Scrolling (SCX/SCY registers)
- [ ] LCD control modes

**Key Concepts:** Sprite system, layer composition, LCD timing modes

---

#### **[Phase 7: Input & Polish](./phase7-input-polish.md)**
Add input handling, debugging tools, and quality-of-life features.

**Milestones:**
- [ ] Joypad input handling
- [ ] Save/load states
- [ ] Debug UI (register viewer, memory inspector)
- [ ] Performance profiling
- [ ] ROM compatibility testing

**Key Concepts:** Input systems, serialization, debugging tools

---

#### **[Phase 8: Audio (Optional)](./phase8-audio.md)**
Implement the Audio Processing Unit (APU) for sound emulation.

**Milestones:**
- [ ] Channel 1: Pulse with sweep
- [ ] Channel 2: Pulse
- [ ] Channel 3: Wave
- [ ] Channel 4: Noise
- [ ] Audio mixing and output

**Key Concepts:** Sound synthesis, audio timing, sample generation

---

## Current Status

**Current Phase:** Phase 1 - CPU Foundation  
**Next Milestone:** Create CPU struct with registers

Track your progress by checking off milestones as you complete them!

## Quick Start

1. Read [Architecture Overview](./architecture.md) to understand the big picture
2. Review [Testing Strategy](./testing-strategy.md) to set up your TDD workflow
3. Start with [Phase 1: CPU Foundation](./phase1-cpu-foundation.md)
4. Follow the TDD examples to implement each feature
5. Run tests frequently (`cargo test`)
6. Move to the next milestone when tests pass

## Development Principles

1. **Test First**: Write tests before implementation (TDD)
2. **Incremental Progress**: Small, working steps are better than big leaps
3. **Understanding Over Speed**: Take time to understand how the hardware works
4. **Refactor Confidently**: Good tests let you refactor without fear
5. **Document as You Go**: Future you will thank present you

## Project Structure (Target)

```
gameman/
├── src/
│   ├── main.rs           # Entry point, emulator loop
│   ├── cpu/
│   │   ├── mod.rs        # CPU public interface
│   │   ├── registers.rs  # Register implementation
│   │   ├── opcodes.rs    # Instruction implementations
│   │   └── decode.rs     # Instruction decoder
│   ├── mmu/
│   │   ├── mod.rs        # Memory management
│   │   └── cartridge.rs  # ROM/RAM banking
│   ├── ppu/
│   │   ├── mod.rs        # Graphics processing
│   │   ├── tiles.rs      # Tile rendering
│   │   └── sprites.rs    # Sprite handling
│   ├── apu/
│   │   └── mod.rs        # Audio processing
│   ├── timer.rs          # Timer implementation
│   ├── interrupts.rs     # Interrupt controller
│   └── joypad.rs         # Input handling
├── tests/
│   ├── cpu_tests.rs      # CPU integration tests
│   └── blargg/           # Test ROM results
├── roms/                 # Test ROMs and games
└── _notes/               # This documentation!
```

## Getting Help

- Stuck on a concept? Check the [Resources](./resources.md) for links to Gameboy documentation
- Test failing? Review the [Testing Strategy](./testing-strategy.md) for debugging tips
- Architecture questions? See [Architecture Overview](./architecture.md)
- Need motivation? Remember: every emulator starts with a single NOP instruction!

## Community Resources

- Pan Docs: The definitive Gameboy hardware reference
- Blargg's Test ROMs: Industry-standard CPU/PPU tests
- Awesome Gameboy Development: Curated list of resources
- /r/EmuDev: Emulation development community

---

**Ready to begin?** Head over to [Phase 1: CPU Foundation](./phase1-cpu-foundation.md) and let's build your first CPU instruction!
