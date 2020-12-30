#!/usr/bin/env bash
set -ue
set -o pipefail

backup() {
  local file
  for file in \
    Cargo.lock \
    Cargo.toml \
  ; do
    if [[ $1 = backup ]]; then
      cp "$file" "$file.bak" || true
    elif [[ $1 = restore ]]; then
      mv "$file.bak" "$file" || true
    fi
  done
}

trap 'backup restore' 1 2 3 15
backup backup

if ci/build-impl.sh; then
  result=0
else
  result=$?
fi

backup restore

exit $result
