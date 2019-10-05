#!/usr/bin/env bash
set -ue
set -o pipefail

rm -f Cargo.lock

if [[ ${MINVER:-false} = true ]]; then
  sed -e '/^\[dependencies\]$/,/^[.*]$/s/"\([0-9]\)/"=\1/g' < Cargo.toml.bak > Cargo.toml
fi

cargo build --examples --verbose
cargo test --verbose

cp Cargo.toml.bak Cargo.toml

if [[ ${TRAVIS_RUST_VERSION:-} = stable ]]; then
  cargo fmt --all -- --check
fi
