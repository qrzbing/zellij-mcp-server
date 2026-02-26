_default:
    @just --list

_fmt:
    cargo fmt

update-docs: _fmt
    cargo run --bin gen_docs

build: update-docs
    cargo build --release

build-static: update-docs
    cargo build --release --locked --target=x86_64-unknown-linux-musl

tool *args: build
    cargo run --release -- {{args}}
