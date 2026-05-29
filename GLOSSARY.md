# Glossary

This document defines common abbreviations used throughout the emulator code and notes.

## System Terms

| Term | Meaning |
| --- | --- |
| APU | Audio Processing Unit (Game Boy sound hardware). |
| Bus | Main memory-routing layer for CPU and devices (MMU-style role). |
| CGB | Game Boy Color hardware family. |
| DMG | Original monochrome Game Boy hardware family. |
| LCD | Liquid Crystal Display subsystem and its control registers. |
| LR35902 | CPU core used by the Game Boy. |
| MMIO | Memory-Mapped I/O (device registers at memory addresses). |
| PPU | Picture Processing Unit (graphics hardware). |
| ROM | Read-Only Memory. |
| RAM | Random-Access Memory. |

## Memory and Cartridge Terms

| Term | Meaning |
| --- | --- |
| Boot ROM | 256-byte startup ROM mapped at `0x0000..=0x00FF` until disabled. |
| DMA | Direct Memory Access; here used mainly for OAM DMA. |
| Echo RAM | `0xE000..=0xFDFF`, mirror of part of WRAM (`0xC000..=0xDDFF`). |
| External RAM | Cartridge RAM window at `0xA000..=0xBFFF`. |
| HRAM | High RAM (`0xFF80..=0xFFFE`). |
| MBC | Memory Bank Controller (cartridge mapper). |
| MBC1 | Mapper type with switchable ROM/RAM banking. |
| OAM | Object Attribute Memory (`0xFE00..=0xFE9F`) for sprite attributes. |
| OAM DMA | 160-byte transfer into OAM, triggered by write to `0xFF46`. |
| Open Bus | Read behavior where hardware returns a default value (often `0xFF`). |
| VRAM | Video RAM (`0x8000..=0x9FFF`). |
| WRAM | Work RAM (`0xC000..=0xDFFF`). |

## Interrupt and Timing Terms

| Term | Meaning |
| --- | --- |
| IE | Interrupt Enable register at `0xFFFF`. |
| IF | Interrupt Flag register at `0xFF0F`. |
| IME | Interrupt Master Enable CPU state bit. |
| IRQ | Interrupt Request. |
| ISR | Interrupt Service Routine. |
| DIV | Divider timer register (`0xFF04`). |
| TIMA | Timer counter register (`0xFF05`). |
| TMA | Timer modulo register (`0xFF06`). |
| TAC | Timer control register (`0xFF07`). |
| VBlank | Vertical blank period after visible scanlines. |
| HBlank | Horizontal blank period after each visible scanline. |
| M-cycle | Machine cycle unit used for instruction timing in this project. |
| T-cycle | CPU clock cycle (finer-grained than M-cycles, used for higher accuracy). |
| FPS | Frames per second. |

## LCD/PPU Register Terms

| Term | Meaning |
| --- | --- |
| LCDC | LCD control register (`0xFF40`). |
| STAT | LCD status register (`0xFF41`). |
| SCY | Scroll Y register (`0xFF42`). |
| SCX | Scroll X register (`0xFF43`). |
| LY | Current scanline register (`0xFF44`). |
| LYC | LY compare register (`0xFF45`). |
| WY | Window Y position register (`0xFF4A`). |
| WX | Window X position register (`0xFF4B`). |
| BGP | Background palette register (`0xFF47`) on DMG. |
| OBP0 | Object/sprite palette 0 register (`0xFF48`) on DMG. |
| OBP1 | Object/sprite palette 1 register (`0xFF49`) on DMG. |

## CPU Register and Opcode Terms

| Term | Meaning |
| --- | --- |
| A, B, C, D, E, H, L | 8-bit CPU general-purpose registers. |
| F | Flags register (status bits). |
| AF, BC, DE, HL | 16-bit register pairs built from 8-bit registers. |
| SP | Stack Pointer register. |
| PC | Program Counter register. |
| Z | Zero flag. |
| N | Subtract flag. |
| H | Half-carry flag. |
| C | Carry flag. |
| CB-prefixed | Extended opcode page reached by prefix byte `0xCB`. |
| HALT | CPU low-power wait state until interrupt condition. |
| STOP | Deeper CPU stop state (wake behavior depends on interrupt/input state). |

## Input and Serial Terms

| Term | Meaning |
| --- | --- |
| JOYP / P1 | Joypad input register at `0xFF00`. |
| SB | Serial transfer data register (`0xFF01`). |
| SC | Serial transfer control register (`0xFF02`). |
