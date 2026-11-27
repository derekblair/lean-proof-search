#!/bin/sh
set -eu
export PATH=/root/.cargo/bin:/root/.elan/bin:$PATH
cd "$(dirname "$0")/.."
cargo fmt --check
cargo build --release
cargo test
cargo clippy --all-targets --all-features -- -D warnings
sh bench/prepare-search.sh
rustfmt --edition 2024 --check --config skip_children=true bench/search/main.rs bench/search_adapter.rs
cargo build --release --manifest-path bench/search/Cargo.toml
cargo test --manifest-path bench/search/Cargo.toml
cargo clippy --manifest-path bench/search/Cargo.toml --all-targets --all-features -- -D warnings
