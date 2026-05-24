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

## CI/CD & Code Quality

- **GitHub Actions:** CI runs on `push` to `main` and all PRs. It enforces `cargo fmt`, `cargo clippy --all-targets --all-features -- -D clippy::all -W clippy::pedantic`, `cargo check`, and `cargo test`.
- **Justfile:** Use `just` for common tasks:
  - `just check` — `cargo check`
  - `just test` — `cargo test`
  - `just lint` — `cargo clippy` (strict)
  - `just fmt` — `cargo fmt`
  - `just fmt-check` — verify formatting
  - `just run` — `cargo run`
- **Best practices:** Before finishing work, run `just lint`, `just test`, and `just fmt-check` to ensure CI will pass. Add `#[must_use]` to pure getters/constructors. Use `assert!(x)` not `assert_eq!(x, true)`.
