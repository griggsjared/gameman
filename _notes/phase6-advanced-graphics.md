# Phase 6: Advanced Graphics

**Status**: Coming Soon - Expand after completing Phase 5

In Phase 6, you'll implement sprites (OAM), the window layer, and complete the PPU for full graphics capability.

## Overview

**Goal**: Complete PPU implementation with sprites and window layer

**What You'll Build:**
- Sprite rendering (OAM - Object Attribute Memory)
- Sprite attributes (flip, palette, priority)
- Window layer rendering
- Sprite priorities and per-scanline limits
- Advanced scrolling
- All LCD modes properly implemented

**Milestones:**
- [ ] OAM structure and parsing
- [ ] Sprite rendering engine
- [ ] Sprite attributes (flip X/Y, palette, priority)
- [ ] 10 sprites per scanline limit
- [ ] Sprite-background priority
- [ ] Window layer implementation
- [ ] Pass dmg-acid2 test
- [ ] Games look correct!

## Topics Covered

1. **OAM (Object Attribute Memory)** - 40 sprites, 4 bytes each
2. **Sprite Format** - Y, X, tile index, attributes
3. **Sprite Rendering** - Per-scanline sprite search
4. **Sprite Limits** - 10 sprites per line, priority
5. **Sprite Attributes** - Palettes, flip, behind-background
6. **Window Layer** - Second background layer
7. **Layer Composition** - Background, window, sprites
8. **Visual Tests** - dmg-acid2, visual accuracy

## Why This Phase is Important

After Phase 6, you'll have:
- Complete graphics rendering
- Games that look correct
- Full PPU implementation
- Visual accuracy comparable to real hardware

## Coming Soon

Detailed documentation will include:
- OAM structure and parsing
- Sprite rendering algorithms
- Attribute handling (flip, palette, priority)
- Window layer implementation
- Layer composition order
- Per-scanline sprite limits
- Visual debugging tools
- TDD tests for sprites and window

**Complete Phase 5 first**, then this guide will be expanded.

---

**Estimated Time**: 1-2 weeks

**Next**: [Phase 7: Input & Polish](./phase7-input-polish.md)
