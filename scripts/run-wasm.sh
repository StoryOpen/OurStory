#!/usr/bin/env bash
set -euo pipefail

# run-wasm.sh — One-stop shop: build wasm client + wz-server + serve
#
# Usage:
#   ./scripts/run-wasm.sh
#
# Requires: wasm-bindgen-cli (cargo install wasm-bindgen-cli)
#
# This script:
#   1. Builds the wasm client for wasm32-unknown-unknown
#   2. Packages it with wasm-bindgen
#   3. Builds wz-server (native host)
#   4. Kills any existing wz-server on port 3001
#   5. Starts wz-server serving both the wasm client and the WZ API

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WASM_DIST="/tmp/wasm-dist"
WASM_TARGET="wasm32-unknown-unknown"
WASM_PROFILE="release-wasm"
WASM_BINARY="$PROJECT_ROOT/target/$WASM_TARGET/$WASM_PROFILE/client.wasm"

echo "╔══════════════════════════════════════════════════╗"
echo "║  Build wasm client + wz-server + serve          ║"
echo "╚══════════════════════════════════════════════════╝"
echo ""

# ── Pre-flight checks ────────────────────────────────────────
command -v wasm-bindgen >/dev/null 2>&1 || {
    echo "✗ wasm-bindgen not found. Install: cargo install wasm-bindgen-cli"
    exit 1
}

# ── Step 1: Build wasm client ──────────────────────────────
echo "── Step 1: Build wasm client ────────────────────────"
cd "$PROJECT_ROOT"
cargo build --profile release-wasm --package client --target "$WASM_TARGET" 2>&1
echo "  ✓ wasm client built"
echo ""

# ── Step 2: Optimize with wasm-opt ───────────────────────────
echo "── Step 2: Optimize with wasm-opt ────────────────────"
if command -v wasm-opt >/dev/null 2>&1; then
    OPT_WASM="$PROJECT_ROOT/target/$WASM_TARGET/$WASM_PROFILE/client-opt.wasm"
    wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int \
        -o "$OPT_WASM" "$WASM_BINARY" 2>&1
    WASM_BINARY="$OPT_WASM"
    echo "  ✓ wasm-opt: $(ls -lh "$WASM_BINARY" | awk '{print $5}')"
else
    echo "  ⚠ wasm-opt not found, skipping optimization"
fi
echo ""

# ── Step 3: Package with wasm-bindgen ───────────────────────—
echo "── Step 3: Package with wasm-bindgen ─────────────────"
rm -rf "$WASM_DIST"
mkdir -p "$WASM_DIST"
wasm-bindgen --target web --out-dir "$WASM_DIST" "$WASM_BINARY"
cp "$PROJECT_ROOT/crates/client/index.html" "$WASM_DIST/"
echo "  ✓ wasm client packaged to $WASM_DIST"
echo ""

# ── Step 4: Build wz-server (native host) ────────────────────
echo "── Step 4: Build wz-server ──────────────────────────"
cd "$PROJECT_ROOT"
cargo build --release --package wz-server 2>&1
echo "  ✓ wz-server built"
echo ""

# ── Step 5: Kill existing wz-server on 3001 ─────────────────
echo "── Step 5: Start wz-server on port 3001 ─────────────"
pkill -f "wz-server.*3001" 2>/dev/null || true
sleep 0.5

# ── Step 6: Start wz-server ─────────────────────────────────
cd "$PROJECT_ROOT"
WZ_PATH=./wz/Base.wz exec ./target/release/wz-server \
    --bind 127.0.0.1:3001 \
    --index-path ./wz/search-index.json \
    --serve-dir "$WASM_DIST"
