# Resources & References

This document contains links to essential documentation, tools, test ROMs, and community resources for Gameboy emulator development.

## Table of Contents

1. [Official Documentation](#official-documentation)
2. [Instruction References](#instruction-references)
3. [Test ROMs](#test-roms)
4. [Development Tools](#development-tools)
5. [Community Resources](#community-resources)
6. [Example Emulators](#example-emulators)
7. [Graphics & PPU Resources](#graphics--ppu-resources)
8. [Audio & APU Resources](#audio--apu-resources)

---

## Official Documentation

### Pan Docs

**The definitive Gameboy technical reference**

- **URL**: https://gbdev.io/pandocs/
- **What it covers**: Everything - CPU, memory, graphics, sound, timing
- **Why it's essential**: Most complete and accurate documentation available
- **Best for**: Looking up specific hardware behavior, register descriptions, timing

**Key Sections:**
- CPU Instructions: Complete opcode table with cycles and flags
- Memory Map: Detailed address space breakdown
- Graphics: PPU modes, tiles, sprites, palettes
- Audio: APU channels and registers
- Cartridge Types: MBC1, MBC3, MBC5, etc.

### Gameboy CPU Manual

- **URL**: http://marc.rawer.de/Gameboy/Docs/GBCPUman.pdf
- **What it covers**: CPU instructions, opcodes, flags
- **Best for**: Quick reference for instruction behavior

### Gameboy Programming Manual

- **URL**: https://ia801906.us.archive.org/19/items/GameBoyProgManVer1.1/GameBoyProgManVer1.1.pdf
- **What it covers**: Official Nintendo programming guide
- **Best for**: Understanding how real games used the hardware

---

## Instruction References

### Opcode Tables

**Gameboy Opcode Table**
- **URL**: https://www.pastraiser.com/cpu/gameboy/gameboy_opcodes.html
- **Format**: Interactive table with all 256 opcodes + CB-prefixed
- **Shows**: Mnemonic, bytes, cycles, flags affected
- **Best for**: Quick lookup during implementation

**Example:**
```
Opcode: 0x80
Mnemonic: ADD A, B
Bytes: 1
Cycles: 4
Flags: Z 0 H C
```

### Opcode Reference by Category

**Arithmetic Operations:**
- ADD, ADC (with carry), SUB, SBC, CP (compare)
- INC, DEC

**Logical Operations:**
- AND, OR, XOR
- CPL (complement), CCF (complement carry), SCF (set carry)

**Rotate/Shift:**
- RLC, RRC, RL, RR (rotate)
- SLA, SRA, SRL (shift)
- SWAP

**Load/Store:**
- LD r, r' (register to register)
- LD r, n (immediate)
- LD r, (HL) (indirect)
- LD (HL), r
- LD A, (BC/DE/C/nn)
- LDH (0xFF00+n), A

**Jump/Call:**
- JP nn, JP cc, JP (HL)
- JR e, JR cc, e
- CALL nn, CALL cc, nn
- RET, RET cc, RETI
- RST n

**Stack:**
- PUSH rr, POP rr

**Bit Operations (CB prefix):**
- BIT n, r (test bit)
- SET n, r (set bit)
- RES n, r (reset bit)

---

## Test ROMs

### Blargg's Test Suite

**Most widely used test ROMs for emulator validation**

- **URL**: https://gbdev.gg8.se/files/roms/blargg-gb-tests/
- **Download**: Individual test ROMs

**Tests Available:**

1. **cpu_instrs.gb**
   - Tests all CPU instructions
   - 11 sub-tests (one per instruction category)
   - Outputs "Passed" or specific failure
   - **Use in**: Phase 3 (Complete CPU)

2. **instr_timing.gb**
   - Tests instruction timing accuracy
   - **Use in**: Phase 5 (Integration & Timing)

3. **mem_timing.gb**
   - Tests memory access timing
   - **Use in**: Phase 5

4. **mem_timing-2.gb**
   - More detailed memory timing tests
   - **Use in**: Phase 5

5. **dmg_sound/** (directory)
   - Tests all 4 audio channels
   - **Use in**: Phase 8 (Audio)

**How to use:**
```rust
#[test]
#[ignore]
fn test_blargg_cpu_instrs() {
    let mut cpu = Cpu::new();
    cpu.mmu.load_rom("roms/test/cpu_instrs.gb").unwrap();
    cpu.reset();
    
    // Run until completion
    // Check serial output for "Passed"
}
```

### Mooneye Test Suite

**More granular, specific tests**

- **URL**: https://github.com/Gekkio/mooneye-test-suite
- **Format**: Individual test ROMs for specific behaviors
- **Best for**: Testing edge cases and specific hardware quirks

**Test Categories:**
- Acceptance tests (core functionality)
- Emulator-only tests
- Manual tests
- Madness tests (extreme edge cases)

### Acid Tests

**Visual tests for graphics accuracy**

1. **dmg-acid2.gb**
   - **URL**: https://github.com/mattcurrie/dmg-acid2
   - Renders specific pattern to test PPU accuracy
   - Compare output to reference image
   - **Use in**: Phase 6 (Advanced Graphics)

2. **cgb-acid2.gb**
   - Gameboy Color version
   - **Use in**: Future GBC support

### Test ROM Resources

- **gb-test-roms**: https://github.com/retrio/gb-test-roms
  - Collection of many test ROMs in one place
- **Test ROM Results**: https://gbdev.gg8.se/wiki/articles/Test_ROMs
  - Shows which emulators pass which tests

---

## Development Tools

### Debuggers

**BGB (Windows)**
- **URL**: https://bgb.bircd.org/
- **Platform**: Windows (works in Wine on Linux/Mac)
- **Features**:
  - Excellent debugger with breakpoints
  - Memory viewer
  - VRAM viewer
  - Step-by-step execution
  - Accurate timing
- **Use for**: Reference behavior, debugging your emulator

**SameBoy (Cross-platform)**
- **URL**: https://sameboy.github.io/
- **Platform**: Windows, Mac, Linux
- **Features**:
  - Cycle-accurate
  - Built-in debugger
  - Memory viewer
- **Use for**: Testing, reference

### Disassemblers

**mgbdis**
- **URL**: https://github.com/mattcurrie/mgbdis
- **Use**: Disassemble Gameboy ROMs to assembly
- **Helpful for**: Understanding how games work

### ROM Analysis

**GBSpec**
- **URL**: https://gbdev.io/gbspec/
- **Use**: Analyze ROM headers
- **Shows**: Title, ROM size, MBC type, checksums

### Graphics Tools

**Tile Designer**
- **URL**: https://github.com/gingemonster/GameBoyTileDesigner
- **Use**: Create and view Gameboy tiles

**GBTD/GBMB (Gameboy Tile Designer/Map Builder)**
- Classic tools for tile/map editing
- Helps understand tile format

---

## Community Resources

### Websites

**gbdev.io**
- **URL**: https://gbdev.io/
- **Content**: Comprehensive Gameboy development resources
- **Includes**: Tutorials, documentation, tools

**/r/EmuDev (Reddit)**
- **URL**: https://www.reddit.com/r/EmuDev/
- **Community**: Emulator developers
- **Good for**: Questions, sharing progress, debugging help

**EmuDev Discord**
- Active community of emulator developers
- Real-time help and discussion
- Link usually found on /r/EmuDev

### YouTube Channels

**The Ultimate Gameboy Talk (33c3)**
- **URL**: https://www.youtube.com/watch?v=HyzD8pNlpwI
- **Length**: 1 hour
- **Content**: Deep dive into Gameboy hardware
- **Presenter**: Michael Steil (Ultimate Commodore 64 Talk)

**Making a Gameboy Emulator**
- Various YouTube series on GB emulator development
- Search: "gameboy emulator development"

### Blog Posts & Tutorials

**Imran Nazar's Gameboy Emulation in JavaScript**
- **URL**: http://imrannazar.com/GameBoy-Emulation-in-JavaScript
- **Content**: Step-by-step tutorial series
- **Good for**: Understanding the development process

**Writing a Gameboy Emulator (Realboyemulator.wordpress.com)**
- Detailed blog series on GB emulator dev
- Covers CPU, PPU, timing

**Codeslinger**
- **URL**: http://www.codeslinger.co.uk/pages/projects/gameboy.html
- Classic tutorial series

---

## Example Emulators

### Reference Implementations

**SameBoy**
- **Language**: C
- **URL**: https://github.com/LIJI32/SameBoy
- **Quality**: Extremely accurate, passes all tests
- **Good for**: Reference when stuck

**Mooneye GB**
- **Language**: Rust
- **URL**: https://github.com/Gekkio/mooneye-gb
- **Quality**: Accurate, well-structured
- **Good for**: Rust implementation reference

**Gambatte**
- **Language**: C++
- **URL**: https://github.com/sinamas/gambatte
- **Quality**: Very accurate
- **Used by**: RetroArch

### Learning-Focused Emulators

**PyBoy**
- **Language**: Python
- **URL**: https://github.com/Baekalfen/PyBoy
- **Good for**: Understanding concepts (Python is readable)

**gomeboycolor**
- **Language**: Go
- **URL**: https://github.com/djhworld/gomeboycolor
- **Good for**: Clean, well-documented code

**rustboy**
- **Language**: Rust
- **URL**: https://github.com/mvdnes/rboy
- **Good for**: Rust implementation examples

### Simple/Minimal Emulators

**cinoop**
- **Language**: C
- **URL**: https://github.com/CTurt/Cinoop
- **Lines**: ~1000 LOC
- **Good for**: Understanding minimal implementation

---

## Graphics & PPU Resources

### Tile Format

Gameboy tiles are 8x8 pixels, 2 bits per pixel (4 colors).

**Storage Format:**
```
Each tile = 16 bytes
Byte 0-1: Row 0 (lo/hi bits)
Byte 2-3: Row 1
...
Byte 14-15: Row 7

Pixel color = (hi_bit << 1) | lo_bit
```

**Example Tile:**
```
Bytes:  0xFF 0x00 0x7E 0xFF 0x85 0x81 0x89 0x83...
Result: ████████
        █░░░░░██
        █░░░░░██
        ...
```

### PPU Timing

```
OAM Scan  (Mode 2): 80 cycles
Drawing   (Mode 3): 172 cycles
HBlank    (Mode 0): 204 cycles
Total per line: 456 cycles

VBlank    (Mode 1): 4560 cycles (10 lines)
Total per frame: 70224 cycles (~59.7 Hz)
```

### VRAM Layout

```
0x8000-0x87FF: Tile set #0 (tiles 0-127)
0x8800-0x8FFF: Tile set #1 (tiles 128-255)
0x9000-0x97FF: Tile set #1 (tiles 0-127, signed addressing)
0x9800-0x9BFF: Background map #0
0x9C00-0x9FFF: Background map #1
```

### Sprite (OAM) Format

Each sprite = 4 bytes:
```
Byte 0: Y position
Byte 1: X position
Byte 2: Tile index
Byte 3: Attributes (palette, flip, priority)
```

40 sprites max, 10 per scanline

---

## Audio & APU Resources

### APU Channels

**Channel 1: Pulse with Sweep**
- Frequency sweep
- Duty cycle (12.5%, 25%, 50%, 75%)
- Volume envelope

**Channel 2: Pulse**
- No sweep
- Duty cycle
- Volume envelope

**Channel 3: Wave**
- 32 4-bit samples
- Custom waveform

**Channel 4: Noise**
- Pseudo-random noise
- Configurable frequency

### Sound Registers

```
0xFF10-0xFF14: Channel 1 (Pulse with sweep)
0xFF15-0xFF19: Channel 2 (Pulse)
0xFF1A-0xFF1E: Channel 3 (Wave)
0xFF1F-0xFF23: Channel 4 (Noise)
0xFF24-0xFF26: Sound control/status
0xFF30-0xFF3F: Wave pattern RAM
```

### APU Timing

- APU runs at 512 Hz frame sequencer
- Length, volume, sweep clocked at specific rates
- See Pan Docs for detailed timing

---

## Cartridge Types (MBC)

### No MBC
- ROM only, max 32KB
- Example: Tetris

### MBC1 (Most Common)
- ROM: up to 2MB (128 banks)
- RAM: up to 32KB (4 banks)
- Banking modes: Simple, Advanced

**Registers:**
- 0x0000-0x1FFF: RAM enable
- 0x2000-0x3FFF: ROM bank number (lower 5 bits)
- 0x4000-0x5FFF: RAM bank / ROM bank upper bits
- 0x6000-0x7FFF: Banking mode select

### MBC3
- ROM: up to 2MB
- RAM: up to 32KB
- Real-Time Clock (RTC)

### MBC5
- ROM: up to 8MB
- RAM: up to 128KB
- Better banking (9-bit ROM bank select)

---

## Useful Data Tables

### Instruction Cycle Counts

Most instructions: 4-24 cycles

**Common patterns:**
- Register operations: 4 cycles
- Immediate loads: 8 cycles
- Memory operations: 8-16 cycles
- Jumps: 12-16 cycles
- Calls: 24 cycles

See opcode table for specific counts.

### Flag Behavior Quick Reference

```
Instruction | Z | N | H | C
------------|---|---|---|---
ADD         | ✓ | 0 | ✓ | ✓
ADC         | ✓ | 0 | ✓ | ✓
SUB         | ✓ | 1 | ✓ | ✓
SBC         | ✓ | 1 | ✓ | ✓
AND         | ✓ | 0 | 1 | 0
OR          | ✓ | 0 | 0 | 0
XOR         | ✓ | 0 | 0 | 0
CP          | ✓ | 1 | ✓ | ✓
INC         | ✓ | 0 | ✓ | -
DEC         | ✓ | 1 | ✓ | -
```

Legend: ✓ = affected, 0/1 = set to 0/1, - = not affected

### Memory Access Times

- ROM: 4 cycles
- WRAM: 4 cycles
- VRAM: 4 cycles (if PPU not using)
- OAM: 4 cycles (if PPU not using)
- I/O: 4 cycles

PPU blocks VRAM/OAM during mode 3 (Drawing)

---

## ROM Downloads

### Homebrew ROMs (Legal, Free)

**gb-test-roms**
- All test ROMs mentioned above

**Homebrew Games**
- **URL**: https://itch.io/games/tag-game-boy
- Free Gameboy games
- Legal to download and test

### Commercial ROMs

**Note**: Commercial ROMs are copyrighted. Only use ROMs you own legally.

To test with commercial games:
- Use ROMs from your own cartridges
- Backup tools: GBxCart, Joey Jr
- Do not distribute ROMs

---

## Recommended Reading Order

**Starting out:**
1. Pan Docs - CPU section
2. Opcode table (pastraiser.com)
3. Phase 1 guide (this project)

**After Phase 1:**
4. Pan Docs - Memory Map
5. Gameboy CPU Manual
6. Phase 2 guide

**After Phase 2:**
7. Complete opcode implementations
8. Blargg's test ROMs
9. Phase 3 guide

**After Phase 3:**
10. Pan Docs - Graphics section
11. Tile format documentation
12. Phase 4-6 guides

**After Phase 6:**
13. Pan Docs - Sound section
14. APU documentation
15. Phase 8 guide

---

## Quick Reference Cheat Sheet

### Memory Map (Quick)
```
0x0000-0x3FFF: ROM Bank 0
0x4000-0x7FFF: ROM Bank 1-N
0x8000-0x9FFF: VRAM
0xA000-0xBFFF: Cartridge RAM
0xC000-0xDFFF: WRAM
0xFE00-0xFE9F: OAM
0xFF00-0xFF7F: I/O
0xFF80-0xFFFE: HRAM
0xFFFF: IE Register
```

### Important Registers (Quick)
```
0xFF00: Joypad
0xFF04: DIV (Timer divider)
0xFF0F: IF (Interrupt flags)
0xFF40: LCDC (LCD control)
0xFF44: LY (Current scanline)
0xFFFF: IE (Interrupt enable)
```

### Interrupts (Quick)
```
Bit | Address | Type
----|---------|-------
0   | 0x40    | VBlank
1   | 0x48    | LCD STAT
2   | 0x50    | Timer
3   | 0x58    | Serial
4   | 0x60    | Joypad
```

---

This resource guide should provide everything you need to build a complete Gameboy emulator. Bookmark Pan Docs and the opcode table - you'll reference them constantly!

**Happy emulating!**
