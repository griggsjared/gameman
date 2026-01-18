# Phase 5: Integration & Timing

**Status**: Coming Soon - Expand after completing Phase 4

In Phase 5, you'll connect all subsystems with accurate timing, implement the timer and interrupt system, and synchronize CPU and PPU execution.

## Overview

**Goal**: Integrate CPU, MMU, PPU with accurate timing and interrupts

**What You'll Build:**
- CPU cycle counting
- Timer registers (DIV, TIMA, TMA, TAC)
- Interrupt controller (IE, IF registers)
- Interrupt handling (priority, IME flag)
- Synchronized CPU-PPU execution
- Frame timing (59.7 FPS)

**Milestones:**
- [ ] Accurate CPU cycle tracking
- [ ] Timer implementation (DIV, TIMA auto-increment)
- [ ] Interrupt system (VBlank, LCD, Timer, Serial, Joypad)
- [ ] Interrupt priorities and handling
- [ ] CPU-PPU synchronization
- [ ] Accurate frame timing
- [ ] Pass timing test ROMs

## Topics Covered

1. **Cycle Counting** - Tracking CPU cycles accurately
2. **Timer System** - DIV, TIMA, TMA, TAC registers
3. **Interrupts** - IE, IF, IME, interrupt vectors
4. **Interrupt Priorities** - Which interrupt fires first
5. **Interrupt Handling** - Push PC, jump to handler, RETI
6. **PPU Timing** - Mode durations, scanline timing
7. **System Synchronization** - Running PPU with CPU cycles
8. **Frame Rate** - Maintaining 59.7 FPS output

## Why This Phase is Important

After Phase 5, you'll have:
- A properly synchronized emulator
- Working interrupts (games depend on these!)
- Accurate timing (critical for game compatibility)
- Foundation for real-time emulation

## Coming Soon

Detailed documentation will include:
- Cycle counting implementation
- Timer register behavior
- Interrupt controller implementation
- Interrupt service routine examples
- CPU-PPU synchronization code
- Frame pacing strategies
- Timing test ROM usage
- TDD tests for timing

**Complete Phase 4 first**, then this guide will be expanded.

---

**Estimated Time**: 1 week

**Next**: [Phase 6: Advanced Graphics](./phase6-advanced-graphics.md)
