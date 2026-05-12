#!/usr/bin/env bash
set -euo pipefail

install_deno() {
  if command -v deno >/dev/null 2>&1; then
    return
  fi

  curl -fsSL https://deno.land/install.sh | sh -s -- -y
  export DENO_INSTALL="${DENO_INSTALL:-$HOME/.deno}"
  export PATH="$DENO_INSTALL/bin:$PATH"
}

install_rust() {
  if ! command -v rustup >/dev/null 2>&1; then
    curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs \
      | sh -s -- -y --profile minimal --default-toolchain stable
  fi

  if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck source=/dev/null
    source "$HOME/.cargo/env"
  fi
  rustup target add wasm32-unknown-unknown
}

install_deno
install_rust

deno task build
