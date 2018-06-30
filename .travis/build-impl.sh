#!/usr/bin/env bash
set -ue
set -o pipefail

PACKAGES=(honeybadger honeybadger-actix-web honeybadger-gotham honeybadger-rocket)

cargo update --verbose

for package in "${PACKAGES[@]}"; do
  if [[ $package = honeybadger-rocket && ${TRAVIS_RUST_VERSION:-} != nightly ]]; then
    continue
  fi

  cargo build -p $package --examples --verbose
done

if [[ ${TRAVIS_RUST_VERSION:-} = stable ]]; then
  cargo fmt --all -- --write-mode check
fi
