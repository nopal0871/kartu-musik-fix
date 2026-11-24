#!/usr/bin/env bash
# Build WebAssembly + susun bundel statis ke ./dist (siap di-host di GitHub Pages / server statis mana pun).
#   ./scripts/build.sh            build debug (cepat)
#   ./scripts/build.sh --release  build rilis (LTO, wasm-opt bila tersedia)
# INSTALL_TOOLS=1 memasang wasm-bindgen-cli (versi disamakan dengan Cargo.lock) dan binaryen bila belum ada.
set -euo pipefail
cd "$(dirname "$0")/.."

PROFILE=debug; CARGO_FLAGS=()
if [[ "${1:-}" == "--release" ]]; then PROFILE=release; CARGO_FLAGS=(--release); fi

WB_VER=$(awk '/^name = "wasm-bindgen"$/{getline; gsub(/[^0-9.]/,"",$0); print; exit}' Cargo.lock)
if [[ -z "$WB_VER" ]]; then echo "Versi wasm-bindgen tidak ditemukan di Cargo.lock" >&2; exit 1; fi

if [[ "${INSTALL_TOOLS:-0}" == "1" ]]; then
  if ! command -v wasm-bindgen >/dev/null || [[ "$(wasm-bindgen --version | awk '{print $2}')" != "$WB_VER" ]]; then
    cargo install wasm-bindgen-cli --version "$WB_VER" --locked
  fi
  if [[ "$PROFILE" == "release" ]] && ! command -v wasm-opt >/dev/null && command -v apt-get >/dev/null; then
    sudo apt-get update -qq && sudo apt-get install -y -qq binaryen
  fi
fi

command -v wasm-bindgen >/dev/null || { echo "wasm-bindgen-cli $WB_VER belum terpasang (jalankan: cargo install wasm-bindgen-cli --version $WB_VER --locked)" >&2; exit 1; }
[[ "$(wasm-bindgen --version | awk '{print $2}')" == "$WB_VER" ]] || { echo "Versi wasm-bindgen-cli harus $WB_VER (sama dengan Cargo.lock)" >&2; exit 1; }

cargo build --lib --target wasm32-unknown-unknown --locked "${CARGO_FLAGS[@]}"

rm -rf dist && mkdir -p dist
cp -R web/. dist/
wasm-bindgen --target web --no-typescript --out-dir dist/pkg --out-name kartu_musik \
  "target/wasm32-unknown-unknown/$PROFILE/kartu_musik.wasm"

if [[ "$PROFILE" == "release" ]] && command -v wasm-opt >/dev/null; then
  wasm-opt -O3 --all-features dist/pkg/kartu_musik_bg.wasm -o dist/pkg/kartu_musik_bg.wasm
fi

touch dist/.nojekyll
echo "Selesai: dist/ ($(du -sh dist | cut -f1)). Coba lokal: ./scripts/serve.sh"
