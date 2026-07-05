#!/usr/bin/env bash
set -euo pipefail

# ──────────────────────────────────────────────────────────────
# deploy-dev.sh — Build and deploy dev versions of wz-server and
# the wasm client to the OCI VM, alongside the existing prod.
#
# Dev and prod coexist on the same VM:
#   Prod:  wz-server :3000,  wasm client at /
#   Dev:   wz-server :3001,  wasm client at /dev/
#
# Builds locally and sends only the binaries. Cross-compiles
# wz-server for aarch64 via the musl target (statically linked,
# no external linker needed — uses rust-lld).
#
# Usage:
#   export OCI_VM_HOST="213.35.123.95"
#   export OCI_VM_SSH_KEY="$HOME/.ssh/oci_free_key"
#   ./scripts/deploy-dev.sh
#
# Environment variables:
#   OCI_VM_HOST      Required. VM hostname or IP.
#   OCI_VM_USER      SSH user (default: ubuntu)
#   OCI_VM_SSH_KEY   SSH private key path (default: ~/.ssh/id_rsa)
# ──────────────────────────────────────────────────────────────

# ── Config ────────────────────────────────────────────────────
HOST="${OCI_VM_HOST:?OCI_VM_HOST not set}"
USER="${OCI_VM_USER:-ubuntu}"
SSH_KEY="${OCI_VM_SSH_KEY:-$HOME/.ssh/id_rsa}"
SSH_OPTS=(-o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -i "$SSH_KEY")

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BUILD_DIR="/tmp/wz-dev-deploy-$$"

# Dev paths on VM
DEV_BIN="/home/ubuntu/.cargo/bin/wz-server-dev"
DEV_WWW="/home/ubuntu/www-dev"
DEV_PORT=3001
DEV_SERVICE="wz-server-dev"

# ── Helpers ───────────────────────────────────────────────────
info()  { echo "  → $*"; }
ok()    { echo "  ✓ $*"; }
err()   { echo "  ✗ $*" >&2; }
die()   { err "$*"; exit 1; }

vm_ssh() {
    ssh "${SSH_OPTS[@]}" "${USER}@${HOST}" "$@"
}

vm_scp() {
    scp "${SSH_OPTS[@]}" "$1" "${USER}@${HOST}:$2"
}

cleanup() {
    rm -rf "$BUILD_DIR"
}
trap cleanup EXIT

mkdir -p "$BUILD_DIR"

# ── Pre-flight checks ────────────────────────────────────────
echo ""
echo "╔══════════════════════════════════════════════════════╗"
echo "║         OurStory Dev Deployment Script              ║"
echo "╚══════════════════════════════════════════════════════╝"
echo ""

command -v cargo >/dev/null 2>&1 || die "cargo not found"
command -v wasm-bindgen >/dev/null 2>&1 || die "wasm-bindgen not found (install: cargo install wasm-bindgen-cli)"
command -v ssh >/dev/null 2>&1 || die "ssh not found"
command -v scp >/dev/null 2>&1 || die "scp not found"

# Check SSH connectivity
info "Checking SSH connectivity to ${USER}@${HOST}..."
vm_ssh true || die "Cannot SSH to ${USER}@${HOST}"

# ── Ensure cross-compilation target ──────────────────────────
echo ""
echo "── Setup: cross-compilation target ────────────────────"

# aarch64-unknown-linux-musl is used because:
#   - rust-lld can link it without any external C cross-compiler
#   - Produces a statically linked binary (no runtime deps)
TARGET="aarch64-unknown-linux-musl"

if ! rustup target list --installed 2>/dev/null | grep -q "$TARGET"; then
    info "Installing $TARGET target..."
    rustup target add "$TARGET"
    ok "$TARGET installed"
else
    ok "$TARGET already installed"
fi

# Verify rust-lld is available (ships with rustup)
RUST_LLD="$(rustc --print sysroot)/lib/rustlib/x86_64-unknown-linux-gnu/bin/rust-lld"
if [ ! -f "$RUST_LLD" ]; then
    die "rust-lld not found at $RUST_LLD"
fi
ok "Using rust-lld as linker"

# ── Step 1: Build wz-server (cross-compile aarch64, musl) ───
echo ""
echo "── Step 1: Build wz-server (aarch64 musl) ─────────────"
cd "$PROJECT_ROOT"
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER="$RUST_LLD"
cargo build --release --package wz-server --target "$TARGET" 2>&1
ok "wz-server built for aarch64 (statically linked)"

SERVER_BIN="$PROJECT_ROOT/target/$TARGET/release/wz-server"
file "$SERVER_BIN"

# ── Step 2: Build wasm client ────────────────────────────────
echo ""
echo "── Step 2: Build wasm client ──────────────────────────"
cd "$PROJECT_ROOT"
cargo build --profile release-wasm --package client --target wasm32-unknown-unknown 2>&1
ok "wasm client built"

# ── Step 3: Package wasm client ──────────────────────────────
echo ""
echo "── Step 3: Optimize + Package wasm client ───────────"

# Optimize with wasm-opt before bindgen
WASM_BINARY="$PROJECT_ROOT/target/wasm32-unknown-unknown/release-wasm/client.wasm"
if command -v wasm-opt >/dev/null 2>&1; then
    OPT_WASM="$PROJECT_ROOT/target/wasm32-unknown-unknown/release-wasm/client-opt.wasm"
    wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int \
        -o "$OPT_WASM" "$WASM_BINARY" 2>&1
    WASM_BINARY="$OPT_WASM"
    ok "wasm-opt: $(ls -lh "$WASM_BINARY" | awk '{print $5}')"
else
    echo "  ⚠ wasm-opt not found, skipping optimization"
fi

WASM_DIST="$BUILD_DIR/wasm-dist"
mkdir -p "$WASM_DIST"
wasm-bindgen --target web --out-dir "$WASM_DIST" "$WASM_BINARY"

# Determine the actual JS filename wasm-bindgen produced
# (it derives from the input wasm filename)
WASM_BASENAME="$(basename "$WASM_BINARY" .wasm)"

cp "$PROJECT_ROOT/crates/client/index.html" "$WASM_DIST/"

# Fix the import path to match wasm-bindgen's output filename
sed -i "s|\./client.js|./${WASM_BASENAME}.js|" "$WASM_DIST/index.html"

# Inject <base href="/dev/"> into index.html for dev deployment
sed -i 's|<head>|<head>\n    <base href="/dev/">|' "$WASM_DIST/index.html"

CLIENT_TAR="$BUILD_DIR/client-wasm-dev.tar.gz"
tar czf "$CLIENT_TAR" -C "$WASM_DIST" .
ok "Wasm client packaged with base href /dev/"

# ── Step 4: Upload artifacts to VM ───────────────────────────
echo ""
echo "── Step 4: Upload artifacts to VM ────────────────────"
info "Uploading wz-server binary..."
vm_scp "$SERVER_BIN" "/tmp/wz-server-dev-built"
ok "wz-server binary uploaded"

info "Uploading wasm client..."
vm_scp "$CLIENT_TAR" "/tmp/client-wasm-dev.tar.gz"
ok "Wasm client uploaded"

# ── Step 5: Deploy on VM ─────────────────────────────────────
echo ""
echo "── Step 5: Deploy on VM ───────────────────────────────"

vm_ssh bash -s << 'REMOTE'
set -euo pipefail

DEV_BIN="/home/ubuntu/.cargo/bin/wz-server-dev"
DEV_WWW="/home/ubuntu/www-dev"
DEV_PORT=3001
DEV_SERVICE="wz-server-dev"

# ── 5a: Set up systemd service if not present ────────────────
if [ ! -f "/etc/systemd/system/${DEV_SERVICE}.service" ]; then
    echo "  → Creating systemd service: ${DEV_SERVICE}"
    sudo tee "/etc/systemd/system/${DEV_SERVICE}.service" > /dev/null <<'UNIT'
[Unit]
Description=wz-server (dev)
After=network.target

[Service]
ExecStart=/home/ubuntu/.cargo/bin/wz-server-dev --bind 127.0.0.1:3001
Restart=always
User=ubuntu
WorkingDirectory=/home/ubuntu

[Install]
WantedBy=multi-user.target
UNIT
    sudo systemctl daemon-reload
    echo "  ✓ ${DEV_SERVICE}.service created"
fi

# ── 5b: Install new wz-server-dev binary ─────────────────────
echo "  → Stopping ${DEV_SERVICE}..."
sudo systemctl stop "${DEV_SERVICE}" 2>/dev/null || true

echo "  → Installing new wz-server-dev binary..."
sudo mv /tmp/wz-server-dev-built "$DEV_BIN"
sudo chmod +x "$DEV_BIN"

echo "  → Starting ${DEV_SERVICE}..."
sudo systemctl start "${DEV_SERVICE}"
echo "  ✓ wz-server-dev deployed"

# ── 5c: Deploy wasm client ───────────────────────────────────
echo "  → Extracting wasm client to ${DEV_WWW}..."
rm -rf "$DEV_WWW"
mkdir -p "$DEV_WWW"
tar xzf /tmp/client-wasm-dev.tar.gz -C "$DEV_WWW"
rm -f /tmp/client-wasm-dev.tar.gz
echo "  ✓ Wasm client deployed to ${DEV_WWW}"

# ── 5d: Update nginx config ──────────────────────────────────
NGINX_CONF="/etc/nginx/sites-available/wz-server"
NEED_RELOAD=false

if [ ! -f "$NGINX_CONF" ]; then
    echo "  → Creating nginx config..."
    sudo tee "$NGINX_CONF" > /dev/null <<'NGINX'
server {
    listen 80;
    server_name _;

    auth_basic "OurStory";
    auth_basic_user_file /etc/nginx/.htpasswd;

    # Prod: served at root
    root /home/ubuntu/www;
    index index.html;

    # Dev: served at /dev/
    location /dev/ {
        alias /home/ubuntu/www-dev/;
        index index.html;
    }

    # Prod API
    location /wz/ {
        proxy_pass http://127.0.0.1:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }

    # Dev API
    location /dev-wz/ {
        proxy_pass http://127.0.0.1:3001;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
NGINX
    sudo ln -sf "$NGINX_CONF" /etc/nginx/sites-enabled/default
    NEED_RELOAD=true
else
    # Check if dev locations already exist
    if ! grep -q "location /dev/" "$NGINX_CONF"; then
        echo "  → Adding dev locations to nginx config..."
        sudo sed -i '/^server {/,/^}/{
            /^}$/i\
    # Dev: served at /dev/\
    location /dev/ {\
        alias /home/ubuntu/www-dev/;\
        index index.html;\
    }\
\
    # Dev API\
    location /dev-wz/ {\
        proxy_pass http://127.0.0.1:3001;\
        proxy_set_header Host $host;\
        proxy_set_header X-Real-IP $remote_addr;\
    }
        }' "$NGINX_CONF"
        NEED_RELOAD=true
    fi
fi

echo "  → Reloading nginx..."
sudo systemctl reload nginx
echo "  ✓ nginx reloaded"

# ── 5e: Verify ───────────────────────────────────────────────
echo ""
echo "── Verification ───────────────────────────────────────"
echo "  wz-server-dev: $(systemctl is-active ${DEV_SERVICE})"
echo "  nginx:         $(systemctl is-active nginx)"
REMOTE

# Verify dev endpoints are reachable
info "Checking dev API endpoint..."
if curl -s --max-time 5 "http://${HOST}/dev-wz/" > /dev/null 2>&1; then
    ok "Dev API reachable at http://${HOST}/dev-wz/"
else
    err "Dev API not reachable (expected if auth is enabled)"
fi

info "Checking dev wasm client..."
if curl -s --max-time 5 "http://${HOST}/dev/" > /dev/null 2>&1; then
    ok "Dev wasm client reachable at http://${HOST}/dev/"
else
    err "Dev wasm client not reachable (expected if auth is enabled)"
fi

echo ""
echo "╔══════════════════════════════════════════════════════╗"
echo "║  Dev deployment complete!                           ║"
echo "║                                                     ║"
echo "║  Prod: http://${HOST}/                              ║"
echo "║  Dev:  http://${HOST}/dev/                          ║"
echo "╚══════════════════════════════════════════════════════╝"
echo ""
