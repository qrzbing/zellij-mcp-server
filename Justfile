_default:
    @just --list

build:
    cargo build --release

build-static:
    cargo build --release --locked --target=x86_64-unknown-linux-musl

tool *args: build
    cargo run --release -- {{args}}
