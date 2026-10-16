---
name: specter-bridge
description: >
  Manage network bridge tunnels, SOCKS5 proxy (1080), HTTP adapter (8118), and local SSH (2222).
  Trigger: /specter-bridge, "specter bridge", "start bridge", "stop bridge", "bridge status", "git spush".
argument-hint: "[status|start|stop|deploy|check] [server]"
license: MIT
---

# Specter Network Bridge (`specter-bridge`)

Controls encrypted egress tunnels, SOCKS5 proxy (1080), HTTP-to-SOCKS5 adapter (8118), and local SSH port forwarding (2222). Workstation outbound firewall blocks direct `git push`; always route via bridge.

## Code & Specification Pointers (Code-as-Docs)

Read native source definitions on demand:
- Bridge CLI Controller: [cli/src/commands/bridge.rs](../../src/commands/bridge.rs)
- Bridge Config Definition: [cli/src/config/bridge.rs](../../src/config/bridge.rs)
- Deployment Runbook & Diag: [Deployment Reference](./references/deployment.md)

## Native Commands

```powershell
# 1. Health Probe: Check dashboard and active ports (1080, 8118, 2222)
specter bridge status

# 2. Start Tunnels: SOCKS5 proxy (1080), SSH (2222), or HTTP adapter (8118)
specter bridge start
specter bridge start --ssh
specter bridge start --http
specter bridge start --ssh --http

# 3. Stop Tunnels: Terminate active daemons cleanly
specter bridge stop

# 4. Config Validation: Check ~/.specter/bridge/bridge.json
specter bridge check
```

## Routing Invariants

- **`git pull` / `git fetch`**: Direct HTTPS (443). Do NOT use proxy.
- **`git push`**: Blocked by firewall. Always use `git spush` (routes via 127.0.0.1:1080).
- **Supabase CLI**: Requires HTTP adapter on 8118 (`specter bridge start --http`).

## Output Contract

Report status strictly:
```text
[BRIDGE] Server: <name> (<target_endpoint>)
• Port 2222 (Local SSH):    <OPEN|CLOSED>
• Port 1080 (SOCKS5 Proxy): <ONLINE|OFFLINE> -> IP: <egress_ip>
• Port 8118 (HTTP Adapter): <ONLINE|OFFLINE>
Verdict: Push ready via 'git spush'. / Action required: run 'specter bridge start --ssh'.
```

## Boundaries

- **Lock Invariant**: Always execute `specter bridge stop` before recompiling CLI binaries to avoid Windows file locks.
- **SSOT**: All configurations reside under `~/.specter/bridge/` (`bridge.json`, `pids/`).
