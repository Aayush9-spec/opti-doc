#!/usr/bin/env bash

set -euo pipefail

APP_NAME="optidock"
INSTALL_ROOT="${OPTIDOCK_INSTALL_ROOT:-$HOME/.optidock}"
BIN_DIR="${OPTIDOCK_BIN_DIR:-$HOME/.local/bin}"

say() {
  printf '[optidock-uninstall] %s\n' "$1"
}

remove_if_exists() {
  if [[ -e "$1" ]]; then
    rm -rf "$1"
    say "Removed $1"
  else
    say "Skipped missing path: $1"
  fi
}

main() {
  remove_if_exists "${BIN_DIR}/${APP_NAME}"
  remove_if_exists "${INSTALL_ROOT}"
  cat <<EOF

OptiDock uninstall finished.

You may still want to remove any PATH entry you added manually for:
  ${BIN_DIR}

EOF
}

main "$@"
