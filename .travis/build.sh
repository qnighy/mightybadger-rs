#!/usr/bin/env bash
set -ue
set -o pipefail

reset_fixvar() {
  for cargo_toml in $(find . -name Cargo.toml); do
    if [[ -e $cargo_toml.bak ]]; then
      mv $cargo_toml.bak $cargo_toml
    fi
  done
}

trap 'reset_fixvar' 1 2 3 15

for cargo_toml in $(find . -name Cargo.toml); do
  if [[ ${MINVER:-false} = true ]]; then
    sed -e '/^\[dependencies\]$/,/^[.*]$/s/"\([0-9]\)/"=\1/g' -i.bak $cargo_toml
  else
    cp $cargo_toml $cargo_toml.bak
  fi
done

if .travis/build-impl.sh; then
  result=0
else
  result=$?
fi

reset_fixvar

exit $result
