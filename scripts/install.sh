#!/usr/bin/env bash

set -euo pipefail

APP_NAME="optidock"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INSTALL_ROOT="${OPTIDOCK_INSTALL_ROOT:-$HOME/.optidock}"
BIN_DIR="${OPTIDOCK_BIN_DIR:-$HOME/.local/bin}"
PROFILE_TARGET="${OPTIDOCK_PROFILE_TARGET:-$HOME/.zshrc}"
ENV_TARGET="${OPTIDOCK_ENV_TARGET:-$INSTALL_ROOT/.env}"
SOURCE_ENV_TEMPLATE="${REPO_ROOT}/.env.example"
CLI_MANIFEST="${REPO_ROOT}/crates/optidock-cli/Cargo.toml"

say() {
  printf '[optidock-installer] %s\n' "$1"
}

fail() {
  printf '[optidock-installer] error: %s\n' "$1" >&2
  exit 1
}

ensure_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    fail "Required command not found: $1"
  fi
}

ensure_directory() {
  mkdir -p "$1"
}

install_binary() {
  if command -v cargo >/dev/null 2>&1; then
    say "Building OptiDock with cargo"
    cargo build --release --manifest-path "$CLI_MANIFEST"
    cp "${REPO_ROOT}/target/release/${APP_NAME}" "${BIN_DIR}/${APP_NAME}"
    chmod +x "${BIN_DIR}/${APP_NAME}"
    say "Installed binary to ${BIN_DIR}/${APP_NAME}"
    return
  fi

  fail "cargo is not installed. Install Rust with rustup, then re-run this installer."
}

install_env_template() {
  ensure_directory "$INSTALL_ROOT"

  if [[ ! -f "$ENV_TARGET" ]]; then
    cp "$SOURCE_ENV_TEMPLATE" "$ENV_TARGET"
    say "Created environment template at $ENV_TARGET"
  else
    say "Environment file already exists at $ENV_TARGET"
  fi
}

ensure_path_hint() {
  if [[ ! -f "$PROFILE_TARGET" ]]; then
    touch "$PROFILE_TARGET"
  fi

  if ! grep -Fq "$BIN_DIR" "$PROFILE_TARGET"; then
    {
      printf '\n# OptiDock\n'
      printf 'export PATH="%s:$PATH"\n' "$BIN_DIR"
    } >>"$PROFILE_TARGET"
    say "Added ${BIN_DIR} to PATH in ${PROFILE_TARGET}"
  else
    say "PATH already includes ${BIN_DIR} in ${PROFILE_TARGET}"
  fi
}

print_next_steps() {
  cat <<EOF

OptiDock installation complete.

Next steps:
  1. Restart your shell or run: source "${PROFILE_TARGET}"
  2. Edit your provider config in: ${ENV_TARGET}
  3. Try:
     ${APP_NAME} providers
     ${APP_NAME} analyze .
     ${APP_NAME} pipeline .

EOF
}

main() {
  say "Preparing installation directories"
  ensure_command cp
  ensure_command mkdir
  ensure_directory "$BIN_DIR"
  install_binary
  install_env_template
  ensure_path_hint
  print_next_steps
}

main "$@"
