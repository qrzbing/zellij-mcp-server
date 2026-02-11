_default:
    @just --list

build:
    cargo build --release

tool *args: build
    cargo run --release -- {{args}}
