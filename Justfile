_default:
    @just --list

build:
    cargo build --release

run *args: build
    cargo run --release -- run {{args}}
