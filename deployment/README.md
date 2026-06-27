# Deployment: wz-server

Automated CI/CD pipeline for deploying `wz-server` to an OCI free-tier ARM VM.

## Architecture

```
[git push] → [GitHub Actions: cross-compile for aarch64] → [GitHub Release asset]
                                                               ↓
[VM: cargo binstall] → [binary in ~/.cargo/bin/] → [systemctl restart wz-server]
```

**Constraints:**
- No direct binary/file copy to the VM (`scp`, `rsync`, etc.)
- Cargo handles distribution (`cargo binstall` downloads from GitHub Releases)
- GitHub handles building (cross-compilation in CI)

---

## Prerequisites

- OCI free-tier ARM VM (already provisioned in `ap-singapore-1`)
- GitHub repo: `github.com/StoryOpen/OurStory`
- SSH access to the VM configured

---

## One-time Setup

### 1. Install `cargo-binstall` on the VM

SSH into the VM and run:

```bash
ssh -i ~/.ssh/oci_free_key ubuntu@213.35.123.95

curl -L --proto '=https' --tlsv1.2 -sSf \
  https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
```

### 2. Set up systemd service on the VM

Create `/etc/systemd/system/wz-server.service`:

```ini
[Unit]
Description=wz-server
After=network.target

[Service]
ExecStart=/home/ubuntu/.cargo/bin/wz-server
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

### 3. Configure GitHub secrets

Run these commands from your local machine (inside the repo):

```bash
# Set the VM public IP
gh secret set VM_HOST --body "213.35.123.95"

# Set the SSH private key (reads the file contents)
gh secret set VM_SSH_KEY < ~/.ssh/oci_free_key
```

These secrets are:
- AES-256 encrypted at rest
- Never exposed in workflow logs
- Only accessible to the Actions workflow

> **Note:** If the VM's ephemeral public IP changes, update `VM_HOST` with the new IP.

---

## CI/CD Pipeline

### Workflow: `.github/workflows/deploy-wz-server.yml`

Triggers on pushes of tags matching `wz-server-v*`.

| Step | Action |
|---|---|
| 1 | Checkout repo |
| 2 | Install Rust 1.95.0 + `aarch64-unknown-linux-gnu` target |
| 3 | Install `gcc-aarch64-linux-gnu` for cross-compilation |
| 4 | `cargo build --release --package wz-server --target aarch64-unknown-linux-gnu` |
| 5 | Package binary as `wz-server-aarch64.tar.gz` |
| 6 | Create GitHub Release with the asset |
| 7 | SSH into VM → `cargo binstall` → `systemctl restart wz-server` |

### Triggering a release

To deploy a new version, push a tag matching `wz-server-v*`:

```bash
git tag wz-server-v0.1.0
git push origin wz-server-v0.1.0
```

The tag name becomes the GitHub Release title and is used by `cargo binstall`
to find the matching binary asset.

### How the update works

1. GitHub Actions cross-compiles `wz-server` for ARM64 on standard x86_64 runners
2. The binary is published as a GitHub Release asset
3. The workflow SSHes into the VM and runs:
   ```bash
   cargo binstall wz-server --git https://github.com/StoryOpen/OurStory --force --no-confirm
   sudo systemctl restart wz-server
   ```
4. `cargo binstall` downloads the matching release asset for `aarch64-unknown-linux-gnu` and places it in `~/.cargo/bin/`

---

## Deployment Flow

```
Developer                        GitHub Actions                        VM
    │                                  │                                │
    ├── git tag wz-server-v0.1.0 ─────►│                                │
    │   git push origin <tag>          │                                │
    │                                  │                                │
    │                                  ├── cross-compile wz-server     │
    │                                  │    (x86_64 → aarch64)         │
    │                                  │                                │
    │                                  ├── create GitHub Release       │
    │                                  │    (attach binary asset)       │
    │                                  │                                │
    │                                  ├── SSH into VM ───────────────►│
    │                                  │                                ├── cargo binstall
    │                                  │                                ├── systemctl restart
    │                                  │◄─────────────── done ─────────┤
    │                                  │                                │
```

---

## Troubleshooting

| Problem | Likely cause | Fix |
|---|---|---|
| SSH connection refused | VM IP changed (ephemeral) | Update `VM_HOST` secret: `gh secret set VM_HOST --body "<new-ip>"` |
| `cargo binstall` fails | No matching release for tag | Check release exists on GitHub with the binary asset |
| Service won't start | Binary path wrong or missing deps | SSH in and run `/home/ubuntu/.cargo/bin/wz-server` manually to see errors |
| Workflow fails at build | Cross-compilation deps missing | Check `gcc-aarch64-linux-gnu` is installed in the runner |
