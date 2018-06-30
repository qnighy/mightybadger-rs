#!/usr/bin/env bash
set -ue
set -o pipefail

cargos=(Cargo.toml honeybadger-actix-web/Cargo.toml honeybadger-gotham/Cargo.toml honeybadger-rocket/Cargo.toml)

for cargo in "${cargos[@]}"; do
  sed -e '/^\[dependencies\]$/,/^[.*]$/s/"\([0-9]\)/"=\1/g' -i.bak $cargo
done
