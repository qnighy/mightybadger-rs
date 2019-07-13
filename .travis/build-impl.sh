#!/usr/bin/env bash
set -ue
set -o pipefail

PACKAGES=(honeybadger honeybadger-actix-web honeybadger-gotham)
if [[ ${RUSTUP_TOOLCHAIN:-} == nightly* || ${TRAVIS_RUST_VERSION:-} == nightly* ]]; then
  PACKAGES+=(honeybadger-rocket)
fi

for package in "${PACKAGES[@]}"; do
  rm -f Cargo.lock
  for file in \
    Cargo.toml \
    honeybadger-actix-web/Cargo.toml \
    honeybadger-gotham/Cargo.toml \
    honeybadger-rocket/Cargo.toml \
  ; do
    cat "$file.bak" | head -n 4 > "$file"
  done

  case "$package" in
    honeybadger)
      dep_cargo_tomls=()
      cargo_toml=Cargo.toml
    ;;
    *)
      dep_cargo_tomls=(Cargo.toml)
      cargo_toml="$package/Cargo.toml"
    ;;
  esac
  dep_cargo_tomls+=(Cargo.toml) # bash < 4.4 hack
  for dep_cargo_toml in "${dep_cargo_tomls[@]}"; do
    cp "$dep_cargo_toml.bak" "$dep_cargo_toml"
  done
  if [[ ${MINVER:-false} = true ]]; then
    sed -e '/^\[dependencies\]$/,/^[.*]$/s/"\([0-9]\)/"=\1/g' < "$cargo_toml.bak" > "$cargo_toml"
  else
    cp "$cargo_toml.bak" "$cargo_toml"
  fi

  cargo build -p $package --examples --verbose
  cargo test -p $package --verbose

  cp "$cargo_toml.bak" "$cargo_toml"
done

if [[ ${TRAVIS_RUST_VERSION:-} = stable ]]; then
  cargo fmt --all -- --check
fi
