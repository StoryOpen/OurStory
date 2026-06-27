# Deployment

Automated CI/CD pipeline for building and deploying `wz-server` (ARM64 binary)
and the wasm client (static files) to an OCI free-tier ARM VM.

## Architecture

```
[git tag v0.1.0] → [GitHub Actions]
                        │
                        ├── cross-compile wz-server (x86_64 → aarch64)
                        ├── build wasm client via trunk
                        ├── create GitHub Release with both assets
                        └── SSH into VM
                             ├── download & install wz-server binary
                             ├── download & extract wasm client to /home/ubuntu/www/
                             └── systemctl restart wz-server
```

The wz-server serves both the API (`/wz/...`) and the wasm client static files (`/`).

---

## Prerequisites

- OCI free-tier ARM VM (provisioned in `ap-singapore-1`)
- GitHub repo: `github.com/StoryOpen/OurStory`
- SSH access to the VM configured

---

## One-time Setup

### 1. Install WZ files on the VM

SSH into the VM and ensure WZ files are at `/home/ubuntu/wz/Base.wz`:

```bash
ssh -i ~/.ssh/oci_free_key ubuntu@213.35.123.95
# Copy or mount WZ files to /home/ubuntu/wz/
```

### 2. Build the search index

```bash
ssh -i ~/.ssh/oci_free_key ubuntu@213.35.123.95
cd /home/ubuntu
/home/ubuntu/.cargo/bin/wz-server --build-index --index-path ./wz/search-index.json
```

### 3. Set up systemd service on the VM

Create `/etc/systemd/system/wz-server.service`:

```ini
[Unit]
Description=wz-server
After=network.target

[Service]
ExecStart=/home/ubuntu/.cargo/bin/wz-server \
  --bind 0.0.0.0:3000 \
  --www-dir /home/ubuntu/www
Restart=always
User=ubuntu
WorkingDirectory=/home/ubuntu

[Install]
WantedBy=multi-user.target
```

Enable and start it:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now wz-server
```

### 4. Configure GitHub secrets

```bash
# Set the VM public IP
gh secret set OCI_VM_HOST --body "213.35.123.95"

# Set the SSH private key
gh secret set VM_SSH_KEY < ~/.ssh/oci_free_key
```

> **Note:** If the VM's ephemeral public IP changes, update `OCI_VM_HOST`.

---

## Triggering a Release

Push a tag matching `v*`:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The CI will:
1. Cross-compile `wz-server` for `aarch64-unknown-linux-gnu`
2. Build the wasm client via `trunk build --release`
3. Create a GitHub Release with both assets
4. SSH into the VM, install the binary, extract the wasm client, restart the service

---

## What gets deployed

| Asset | Source | Destination on VM |
|---|---|---|
| `wz-server-aarch64-unknown-linux-gnu.tar.gz` | `cargo build --target aarch64-unknown-linux-gnu` | `/home/ubuntu/.cargo/bin/wz-server` |
| `client-wasm.tar.gz` | `trunk build` from `crates/client/` | `/home/ubuntu/www/` (extracted) |

The wz-server serves:
- `/wz/...` — API endpoints (node data, images, search, structured data)
- `/` — wasm client static files (index.html, .wasm, .js)

---

## Troubleshooting

| Problem | Likely cause | Fix |
|---|---|---|
| SSH connection refused | VM IP changed (ephemeral) | Update `OCI_VM_HOST` secret |
| wz-server won't start | Missing WZ files or search index | Run `--build-index` on the VM |
| Wasm client shows blank page | CORS or wrong API URL | Check browser console for errors |
| Workflow fails at build | Missing deps | Check `gcc-aarch64-linux-gnu` or trunk install |
