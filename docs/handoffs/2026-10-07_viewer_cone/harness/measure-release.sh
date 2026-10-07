#!/usr/bin/env bash
# The viewer cone's payload (mer3ly site canvas plan, Rulings 26, 107, 108):
# release bundles of the full page, the viewer, and the viewer with canvas-gpu,
# with P0's 2026-10-01 settings (opt-level "s", thin LTO, strip "symbols",
# codegen-units 1), measured raw, gzip -9 and brotli -q 11, before and after
# wasm-opt -Os. Writes the log named by $1, refusing to overwrite; one config
# at a time.
set -euo pipefail
LOG=${1:?usage: measure-release.sh LOG [CONFIGS] [JOBS]}
# $2: a space-separated subset of default, viewer, viewer-gpu (all by default);
# $3: cargo jobs (6 by default), fewer when the machine is short of memory.
WANT=${2:-default viewer viewer-gpu}
JOBS=${3:-6}
if [ -e "$LOG" ]; then echo "refusing to overwrite $LOG" >&2; exit 2; fi
OUT=C:/Users/mark_/Code/testing/mere/viewer-cone/release
WEB=C:/Users/mark_/Code/worktrees/mere-viewer-cone/ports/graphshell/web
export CARGO_TARGET_DIR=C:/Users/mark_/Code/worktrees/mere-genet-imgdec/ports/graphshell/web/target
export CARGO_PROFILE_RELEASE_OPT_LEVEL=s
export CARGO_PROFILE_RELEASE_LTO=thin
export CARGO_PROFILE_RELEASE_STRIP=symbols
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
BINDGEN=C:/t/wasm-bindgen-0.2.129/bin/wasm-bindgen.exe
sizes() {
  local f=$1
  printf "raw %s gzip9 %s brotli11 %s" "$(stat -c %s "$f")" "$(gzip -9 -c "$f" | wc -c)" "$(brotli -q 11 -c "$f" | wc -c)"
}
cd "$WEB"
{
  echo "tree $(git rev-parse --short HEAD), $(git status --short | wc -l) changed paths"
  echo "web lock sha256 $(sha256sum Cargo.lock | cut -d' ' -f1)"
  for cfg in "default:" "viewer:--no-default-features" "viewer-gpu:--no-default-features --features canvas-gpu"; do
    name=${cfg%%:*}; flags=${cfg#*:}
    case " $WANT " in *" $name "*) ;; *) continue ;; esac
    start=$(date +%s)
    cargo build -j "$JOBS" --release --locked --offline --target wasm32-unknown-unknown $flags
    secs=$(( $(date +%s) - start ))
    rm -rf "$OUT/$name" && mkdir -p "$OUT/$name"
    "$BINDGEN" --target web --out-dir "$OUT/$name" "$CARGO_TARGET_DIR/wasm32-unknown-unknown/release/graphshell_web.wasm"
    # The features rustc enables by default on wasm32-unknown-unknown (1.82+).
    wasm-opt -Os --enable-bulk-memory --enable-multivalue --enable-mutable-globals --enable-nontrapping-float-to-int --enable-reference-types --enable-sign-ext "$OUT/$name/graphshell_web_bg.wasm" -o "$OUT/$name/graphshell_web_bg.opt.wasm"
    echo "$name (flags '$flags', ${secs}s build)"
    echo "  wasm      $(sizes "$OUT/$name/graphshell_web_bg.wasm")"
    echo "  wasm -Os  $(sizes "$OUT/$name/graphshell_web_bg.opt.wasm")"
    echo "  js        $(sizes "$OUT/$name/graphshell_web.js")"
    echo "  sha256    $(sha256sum "$OUT/$name/graphshell_web_bg.wasm" | cut -d' ' -f1)"
  done
  echo "EXIT 0"
} > "$LOG" 2>&1
