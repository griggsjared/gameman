set dotenv-load

_default:
    just --list

check:
    cargo check

test:
    cargo test

lint:
    cargo clippy --all-targets --all-features -- -D clippy::all -W clippy::pedantic

fmt:
    cargo fmt

fmt-check:
    cargo fmt -- --check

run:
    cargo run

# Download Blargg test ROMs (needed for blargg tests)
test-roms:
    mkdir -p test-roms
    curl -L -o /tmp/cpu_instrs.zip \
      https://gbdev.gg8.se/files/roms/blargg-gb-tests/cpu_instrs.zip
    unzip -o /tmp/cpu_instrs.zip -d test-roms/
    rm /tmp/cpu_instrs.zip

# Run Blargg CPU test ROMs (download first via `just test-roms`)
blargg:
    cargo test --test blargg -- --nocapture
