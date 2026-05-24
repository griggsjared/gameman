# Gameman - Game Boy Emulator

A Game Boy (DMG) emulator written in Rust.

## Project Conventions

- **Language:** Rust
- **Testing:** Write tests to cover code you implement. Aim for good coverage of boundary conditions and edge cases.
- **Code organization:** One concern per file/module; keep modules small and focused.
- **Error handling:** Panic on unimplemented opcodes early in development. Move to graceful error handling in later phases.

## Core Standards

- **Register design:** Individual fields, not arrays (e.g., `registers.a`, not `registers[0]`).
- **Naming:** Clear, descriptive names (e.g., `step`, not `tick`).
- **Cycle timing:** Instruction-level counting initially, not T-cycle accuracy yet.
