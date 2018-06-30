#!/usr/bin/env bash
set -ue
set -o pipefail

PACKAGES=(honeybadger honeybadger-actix-web honeybadger-gotham honeybadger-rocket)

for package in "${PACKAGES[@]}"; do
  if [[ $package = honeybadger-rocket && $TRAVIS_RUST_VERSION != nightly ]]; then
    continue
  fi

  if [[ $package = honeybadger ]]; then
    package_dir=.
  else
    package_dir="$package"
  fi

  if [[ ${MINVER:-false} = true ]]; then
    sed -e '/^\[dependencies\]$/,/^[.*]$/s/"\([0-9]\)/"=\1/g' -i.bak $package_dir/Cargo.toml
  else
    cp $package_dir/Cargo.toml $package_dir/Cargo.toml.bak
  fi

  cargo build -p $package --examples --verbose

  mv $package_dir/Cargo.toml.bak $package_dir/Cargo.toml
done

if [[ $TRAVIS_RUST_VERSION = stable ]]; then
  cargo fmt --all -- --write-mode check
fi
