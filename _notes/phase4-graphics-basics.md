# Phase 4: Graphics Basics

**Status**: Coming Soon - Expand after completing Phase 3

In Phase 4, you'll implement the Picture Processing Unit (PPU) to render tiles and the background layer. This is where your emulator starts producing visual output!

## Overview

**Goal**: Implement basic PPU to render background tiles

**What You'll Build:**
- PPU state machine (OAM scan, drawing, HBlank, VBlank)
- Tile decoding (2bpp format to pixels)
- Background layer rendering
- LCD control registers (LCDC, STAT, SCY, SCX, LY, etc.)
- Framebuffer output
- Simple display (file output or terminal)

**Milestones:**
- [ ] PPU structure and state machine
- [ ] Tile data decoding
- [ ] Background tilemap interpretation
- [ ] Scanline rendering
- [ ] VBlank interrupt generation
- [ ] Basic framebuffer output
- [ ] First rendered frame!

## Topics Covered

1. **PPU Architecture** - State machine, timing, LCD modes
2. **Tile Format** - 8x8 tiles, 2 bits per pixel, decoding
3. **VRAM Organization** - Tile sets, tilemaps, addressing modes
4. **Background Rendering** - Tilemap lookup, scrolling
5. **LCD Registers** - LCDC, STAT, SCY, SCX, LY, LYC
6. **Timing** - Scanline timing, mode durations, VBlank
7. **Output Methods** - PPM files, terminal, or basic windowing
8. **Testing Graphics** - Visual tests, reference images

## Why This Phase is Important

After Phase 4, you'll:
- See actual graphics from your emulator
- Understand tile-based rendering
- Have a foundation for sprites and advanced graphics
- Be able to see backgrounds in games

## Coming Soon

Detailed documentation will include:
- PPU state machine implementation
- Tile decoding algorithms with examples
- Background rendering step-by-step
- LCD register behavior
- Scanline rendering loop
- Simple output methods (PPM, terminal graphics)
- TDD tests for PPU
- Visual debugging techniques

**Complete Phase 3 first**, then this guide will be expanded.

---

**Estimated Time**: 1-2 weeks

**Next**: [Phase 5: Integration & Timing](./phase5-integration.md)
