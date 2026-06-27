# Deployment

Automated CI/CD pipeline for building and deploying `wz-server` (ARM64 binary)
and the wasm client (static files) to an OCI free-tier ARM VM.

## Architecture

```
                        ┌── Deploy Server ──► cross-compile wz-server (aarch64)
                        │                          │
[git tag v0.1.0] ───────┤                          └──► VM: /home/ubuntu/.cargo/bin/wz-server
                        │
                        └── Deploy Wasm Client ──► trunk build
                                                      │
                                                      └──► VM: /home/ubuntu/www/
```

Nginx on the VM acts as a reverse proxy:
- `/wz/...` → `127.0.0.1:3000` (wz-server API)
- `/` → `/home/ubuntu/www/` (wasm client static files)
- Port 3000 exposed only to `127.0.0.1` (not public)

**Native Rust client** deploys separately (not via this pipeline).

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

### 3. Set up nginx

```bash
ssh -i ~/.ssh/oci_free_key ubuntu@213.35.123.95
sudo apt-get install -y nginx
```

Create `/etc/nginx/sites-available/wz-server`:

```nginx
server {
    listen 80;
    server_name _;

    # Wasm client static files
    root /home/ubuntu/www;
    index index.html;

    # API proxy to wz-server
    location /wz/ {
        proxy_pass http://127.0.0.1:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

Enable it:

```bash
sudo ln -sf /etc/nginx/sites-available/wz-server /etc/nginx/sites-enabled/default
sudo systemctl enable --now nginx
```

### 4. Set up wz-server systemd service

Create `/etc/systemd/system/wz-server.service`:

```ini
[Unit]
Description=wz-server
After=network.target

[Service]
ExecStart=/home/ubuntu/.cargo/bin/wz-server --bind 127.0.0.1:3000
Restart=always
User=ubuntu
WorkingDirectory=/home/ubuntu

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now wz-server
```

### 5. Configure GitHub secrets

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

Two workflows run in parallel:

| Workflow | Builds | Deploys to |
|---|---|---|
| `deploy-server.yml` | `wz-server` for `aarch64-unknown-linux-gnu` | `/home/ubuntu/.cargo/bin/wz-server` |
| `deploy-wasm-client.yml` | `trunk build --release` from `crates/client/` | `/home/ubuntu/www/` |

Each creates a GitHub Release asset and then SSHes into the VM to update its respective artifact.

---

## What gets deployed

| Asset | Source | Destination on VM | Served by |
|---|---|---|---|
| `wz-server-aarch64-...tar.gz` | `cargo build --target aarch64...` | `/home/ubuntu/.cargo/bin/wz-server` | nginx → `127.0.0.1:3000` |
| `client-wasm.tar.gz` | `trunk build --release` | `/home/ubuntu/www/` | nginx (static files) |

Nginx serves:
- `/wz/...` → proxy to wz-server API
- `/` → wasm client static files from `/home/ubuntu/www/`

Port 3000 is **not** exposed to the internet — only nginx on port 80 is public.

---

## Troubleshooting

| Problem | Likely cause | Fix |
|---|---|---|
| SSH connection refused | VM IP changed (ephemeral) | Update `OCI_VM_HOST` secret |
| wz-server won't start | Missing WZ files or search index | Run `--build-index` on the VM |
| Wasm client shows blank page | Nginx config or wrong www path | Check nginx error logs on VM |
| nginx fails to proxy | wz-server not running | `sudo systemctl status wz-server` |
| Workflow fails at build | Missing deps | Check `gcc-aarch64-linux-gnu` or trunk install |
