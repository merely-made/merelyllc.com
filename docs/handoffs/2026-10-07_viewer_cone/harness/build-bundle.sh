#!/usr/bin/env bash
# The viewer-cone lane's bundle build (2026-10-06), on G9's build-wasm-fresh
# pattern: dev profile without debug info, locked, then the wasm-bindgen 0.2.129
# CLI into the worktree's ignored pkg/. $2 is "default" or cargo feature flags
# for the viewer (e.g. "--no-default-features"). The getrandom cfg comes from
# the committed ports/graphshell/web/.cargo/config.toml (ruling 558).
# $3, optional, is the bindgen output directory (default pkg/).
# Writes the log named by $1, refusing to overwrite.
set -euo pipefail
LOG=${1:?usage: build-bundle.sh LOG FLAGS}
FLAGS=${2:?usage: build-bundle.sh LOG FLAGS}
if [ -e "$LOG" ]; then echo "refusing to overwrite $LOG" >&2; exit 2; fi
WEB=C:/Users/mark_/Code/worktrees/mere-viewer-cone/ports/graphshell/web
export CARGO_TARGET_DIR=C:/Users/mark_/Code/worktrees/mere-genet-imgdec/ports/graphshell/web/target
export CARGO_PROFILE_DEV_DEBUG=0
BINDGEN=C:/t/wasm-bindgen-0.2.129/bin/wasm-bindgen.exe
[ "$FLAGS" = "default" ] && FLAGS=""
cd "$WEB"
{
  echo "tree $(git rev-parse --short HEAD), $(git status --short | wc -l) changed paths; flags '${FLAGS}'"
  echo "web lock sha256 $(sha256sum Cargo.lock | cut -d' ' -f1)"
  cargo build -j 6 --locked --offline --target wasm32-unknown-unknown $FLAGS
  "$BINDGEN" --version
  OUT=${3:-pkg}
  "$BINDGEN" --target web --no-demangle --out-dir "$OUT" "$CARGO_TARGET_DIR/wasm32-unknown-unknown/debug/graphshell_web.wasm"
  echo "bundle sha256 $(sha256sum "$OUT"/graphshell_web_bg.wasm | cut -d' ' -f1) size $(stat -c %s "$OUT"/graphshell_web_bg.wasm) into $OUT"
  echo "EXIT 0"
} > "$LOG" 2>&1
