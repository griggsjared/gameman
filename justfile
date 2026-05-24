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
