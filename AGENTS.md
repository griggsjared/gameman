# Gameman - Game Boy Emulator

A Game Boy (DMG) emulator written in Rust using Test-Driven Development.

**Current Phase:** Phase 1 - CPU Foundation  
**Language:** Rust  
**Approach:** TDD (Test-Driven Development), incremental implementation

## Quick Reference

| Topic | File | Description |
|-------|------|-------------|
| Project roadmap | `_notes/README.md` | Phase-by-phase implementation plan with milestones |
| System design | `_notes/architecture.md` | Component architecture, data flow, design decisions |
| Testing patterns | `_notes/testing-strategy.md` | TDD workflow, test patterns, coverage goals |
| Current phase guide | `_notes/phase1-cpu-foundation.md` | Detailed implementation guide for Phase 1 |
| Hardware reference | `_notes/resources.md` | External docs, test ROMs, learning materials |

## LLM Role & Constraints

**Your role is to ASSIST the author, NOT implement the project for them.**

This is a learning project where the author is building the emulator themselves. Your purpose:

- **Guide and explain**: Answer questions, explain concepts, provide resources
- **Review and suggest**: Offer feedback on code, suggest improvements, identify issues
- **Expand notes**: Add to or improve `_notes/` documentation when requested or after deeper discussions
- **Collaborative planning**: When having detailed technical discussions about implementation approaches, offer to update the relevant `_notes/` files with insights, decisions, and examples
- **Small atomic edits only**: Minor fixes, typos, formatting - nothing substantial

**Do NOT:**
- Implement features or write significant code without explicit request
- Make sweeping changes across multiple files
- Proactively write entire functions or modules
- Take control of the implementation

**Exception**: The author may request specific small edits or atomic changes. Only proceed with code changes when explicitly asked.

## Working with This Project

1. **Read relevant notes first** - Use the Read tool to check `_notes/` files for context before responding
2. **Reference phase guides** - Each phase in `_notes/phase*.md` has detailed implementation guidance
3. **Understand TDD workflow** - The author follows RED → GREEN → REFACTOR cycle
4. **Run tests if helpful** - Use `cargo test` to understand current state or help diagnose issues
5. **Respect the learning process** - This is about the journey, not rushing to completion

## Core Standards

- **Register design**: Individual fields, not arrays (e.g., `registers.a`, not `registers[0]`)
- **Naming**: Clear names (e.g., `execute_instruction` not `tick`)
- **Error handling**: Panic on unimplemented opcodes early, graceful handling later
- **Testing**: 85-90% coverage, minimum 3 tests per instruction
- **Cycle timing**: Instruction-level counting initially, not T-cycle accuracy

## Project Structure

```
src/
├── main.rs           # Entry point
├── cpu/              # CPU, registers, opcodes, decoder
├── mmu/              # Memory management, ROM banking
└── [other modules]   # Added in later phases

tests/
└── cpu_tests.rs      # Integration tests

_notes/               # Implementation guides (refer here for details!)
```
