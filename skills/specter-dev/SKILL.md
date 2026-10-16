---
name: specter-dev
description: >
  Orchestrate local development and preview servers with LAN and Tailscale mesh port exposure (--host 0.0.0.0) for live browser inspection on phone and workstation.
  Supports both fast HMR dev mode and ultra-fast bundled preview mode to eliminate remote network latency.
  Enforces strict Local-First verification and prohibits unprompted git push or GitHub Actions CI/CD abuse.
  Trigger: /specter-dev, "specter dev", "local dev", "dev server", "tailscale dev", "preview", "preview local", "test local", "lan check".
  Do NOT use for production release tagging (use /specter-cicd) or headless browser automation (use /specter-automa).
argument-hint: "[dev|preview|stop|status|tailscale] [admin|automa|web|docs] [--port <port>]"
license: MIT
---

# Specter Local Dev & Preview Verification (`specter-dev`)

Fast local dev & preview orchestrator with LAN and Tailscale mesh port binding (`--host 0.0.0.0`). Delivers instant live UI review on mobile phones and workstations without exposing public ports or wasting GitHub Actions CI/CD minutes.

## The Runtime Modes: Dev vs. Preview vs. Persistent Nginx

| Mode | Command Pattern | Best For | Network Profile |
| :--- | :--- | :--- | :--- |
| **`dev`** | `vite --host 0.0.0.0` | Active coding with instant Hot Module Replacement (HMR). | Serves unbundled ES modules. May feel sluggish over mobile WAN/Tailscale due to high round-trip requests. |
| **`preview`** | `vite build && vite preview --host 0.0.0.0` | Reviewing & mobile testing when Nginx is inactive. | Serves pre-bundled, minified chunks via temporary Node.js process. |
| **`persistent` (Nginx)** | `pnpm build` (Auto-served by Nginx) | **Standard Mobile & Desktop Review**: Zero background task overhead, 24/7 liveness. | Serves production dist via native Nginx daemon on port 80 (Tailscale mesh). ~2MB RAM, zero lag, instant live updates upon build. |

## The Invariants

1. **Strict Push Prohibition**: NEVER run `git push` or trigger remote GitHub Actions CI/CD unless the user explicitly commands "push", "release", "ship", or "deploy".
2. **Private Mesh Over Public Internet**: Prioritize Tailscale private mesh (`100.x.y.z`) for phone and remote workstation access. Eliminates opening public firewall ports, maintaining zero-trust server hardening.
3. **0.0.0.0 Interface Binding**: Always bind dev/preview servers to `0.0.0.0` so HTTP traffic routes seamlessly across Tailscale mesh, LAN, and local loopback.
4. **Local Quality Gates**: Validate linting (`oxfmt`, `cargo check`, `vue-tsc`) and tests on the machine before requesting user sign-off. Never debug via remote CI runners.
5. **Sign-off Loop**: Provide both Tailscale mesh URL (for phone/laptop) and localhost URL. Await explicit user confirmation before committing or shipping.

## Network IP Resolution Hierarchy

When reporting dev/preview URLs, resolve IP addresses in strict priority order:

1. **Tailscale Mesh IP** (`100.x.y.z`): Direct encrypted peer-to-peer connection between VPS, phone, and laptop.
   - Command: `tailscale ip -4 2>/dev/null || ip -br addr show tailscale0 2>/dev/null | awk '{print $3}' | cut -d/ -f1`
2. **Host / LAN IP** (`hostname -I | awk '{print $1}'`): Fallback for local physical network or direct host routing.
3. **Local Loopback** (`127.0.0.1`): Localhost verification on the machine itself.

## Supported Services & Default Ports

| Service Key | Target Repository | Default Port | Dev Command (HMR) | Preview Command (Bundled) |
| :--- | :--- | :---: | :--- | :--- |
| **`admin`** | `/root/tuquet/admin` | `5555` | `pnpm -F @tuquet/admin-web run dev -- --host 0.0.0.0 --port 5555` | `pnpm -F @tuquet/admin-web run build && pnpm --filter @tuquet/admin-web exec vite preview --host 0.0.0.0 --port 5555` |
| **`automa`** | `/root/tuquet/automa` | `3000` | `pnpm dev --host 0.0.0.0 --port 3000` | `pnpm build && pnpm preview --host 0.0.0.0 --port 3000` |
| **`web`** | `/root/tuquet/tuquet.com` | `3333` | `pnpm dev --host 0.0.0.0 --port 3333` | `pnpm build && pnpm preview --host 0.0.0.0 --port 3333` |
| **`docs`** | `/root/tuquet/docs` | `5173` | `pnpm docs:dev --host 0.0.0.0 --port 5173` | `pnpm docs:build && pnpm docs:preview --host 0.0.0.0 --port 5173` |
| **`api`** | `/root/tuquet/cli` | `8765` | `cargo run --bin specter -- api --port 8765` | *(Native binary)* |

## Execution Workflow

1. **Detect Network Mesh**: Query Tailscale IP and Host IP.
2. **Check Port Availability**: Verify target port is unallocated (`ss -tulpn | grep :<port>`).
3. **Select Mode**: Use `dev` for active code edits; use `preview` for mobile inspection and zero-lag review.
4. **Launch Server**: Spawn daemon with `--host 0.0.0.0` in background task (`IsDaemon: true`).
5. **Emit Contract**: Output Tailscale URL (for phone), Host URL, and Local URL.
6. **Await User Sign-off**: Do NOT commit or push until user verifies UI on phone/desktop.

## Commands

### 1. Detect Network & Tailscale IPs
```bash
TS_IP=$(tailscale ip -4 2>/dev/null || ip -br addr show tailscale0 2>/dev/null | awk '{print $3}' | cut -d/ -f1)
HOST_IP=$(hostname -I | awk '{print $1}')

echo "Tailscale Mesh IP: ${TS_IP:-Not Connected}"
echo "Host / LAN IP:     $HOST_IP"
```

### 2. Start Preview Mode (Fast Mobile Review)
```bash
# Build bundled production assets and start high-performance preview server:
cd /root/tuquet/admin
pnpm -F @tuquet/admin-web run build
pnpm --filter @tuquet/admin-web exec vite preview --host 0.0.0.0 --port 5555
```

### 3. Start Dev Mode (Active Code Editing)
```bash
# Launch Admin Dashboard with instant HMR:
cd /root/tuquet/admin
pnpm -F @tuquet/admin-web run dev -- --host 0.0.0.0 --port 5555
```

### 4. Stop Server
```bash
# Terminate server process on specific port:
fuser -k 5555/tcp 2>/dev/null || true
```

## Deterministic Output Contract

Report server state strictly:
```text
[DEV|PREVIEW] Service: <admin|automa|web|docs> | Port: <port> (<ONLINE|STOPPED>)
• Mode:           <PREVIEW (Production-bundled, zero-lag) | DEV (Unbundled HMR)>
• Tailscale Mesh: http://<TAILSCALE_IP>:<port> (Direct access from Phone & Workstation)
• Host / LAN URL: http://<HOST_IP>:<port>
• Local URL:      http://127.0.0.1:<port>
• Process PID:    <pid>
• Tailscale:      CONNECTED (100.x.y.z) / DISCONNECTED
• CI Gate:        STRICT LOCAL (Zero GH push / zero cloud runner waste)
• Action:         Inspect on phone/desktop via Tailscale -> Await confirmation before commit/push.
```

## Boundaries

Scope: Local development & preview runtimes, Tailscale mesh and LAN port binding, mobile/desktop live inspection, and CI budget preservation.
- Do NOT run `git push` or trigger remote GitHub Actions without explicit user request.
- Do NOT use remote CI/CD as a scratchpad or visual testing environment.
- Do NOT bind exclusively to `127.0.0.1` when user asks for phone or remote review (use `0.0.0.0`).
- Do NOT compile release binaries (`cargo build --release`) on VPS dev environments.
