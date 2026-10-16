---
name: specter-help
description: >
  Quick-reference card and master cheatsheet for all Specter CLI commands, interactive shell scopes,
  and canonical SSOT microservice pillars under ~/.specter/. One-shot display.
  Trigger: /specter-help, "specter help", "what specter commands", "how do I use specter", "specter cheatsheet".
argument-hint: ""
license: MIT
---

# Specter Master Cheatsheet & Help Card (`specter-help`)

One card. Zero fluff. All commands, interactive REPL scopes, and SSOT pillars. Display this reference card when invoked. One-shot, changes nothing.

## Available Skills (All 5 Pillars Covered)

<!-- SKILLS_CATALOG_START -->
| Skill | Trigger | Argument Hint | Scope & Purpose |
| :--- | :--- | :--- | :--- |
| **specter** | `/specter` | `[status\|dashboard]` | Master orchestrator and platform health dashboard (`specter status`). |
| **specter-admin** | `/specter-admin` | `[preferences\|widgets\|diff\|test\|build]` | Master runbook for Tuquet Admin Dashboard, Preferences engine, and upstream diff triage. |
| **specter-automa** | `/specter-automa` | `[run\|list\|inspect\|studio] [workflow.json]` | Headless automation runner and visual DAG engine for browser tasks via native CDP and local SQLite store. |
| **specter-bot** | `/specter-bot` | `[status\|logs\|restart\|test\|run\|deploy]` | Telegram ChatOps assistant daemon, server health monitoring, and GitHub Actions notification engine. |
| **specter-bridge** | `/specter-bridge` | `[status\|start\|stop\|deploy\|check] [server]` | Manage network bridge tunnels, SOCKS5 proxy (1080), HTTP adapter (8118), and local SSH (2222). |
| **specter-browser** | `/specter-browser` | `[status\|install\|list\|use\|clean\|path]` | Manage Antidetect Chromium runtimes, hardware emulation seeds, profile sandboxes, and proxy routing. |
| **specter-cicd** | `/specter-cicd` | `[preflight\|plan\|tag\|status\|dispatch] [repo] [version]` | CI/CD pipeline orchestration, local pre-flight gatekeeping, semantic version tagging, and GitHub Actions budget control. |
| **specter-cloud** | `/specter-cloud` | `[whoami\|login\|logout\|config\|db]` | Manage Specter Cloud control plane, device fleet enrollment, and Supabase database migrations. |
| **specter-desktop** | `/specter-desktop` | `[dev\|build\|doctor\|clean\|audit]` | Manage Tuquet Tauri Desktop runtime lifecycle, in-process Axum bridge daemon,. |
| **specter-dev** | `/specter-dev` | `[dev\|preview\|stop\|status\|tailscale] [admin\|automa\|web\|docs] [--port <port>]` | Orchestrate local development and preview servers with LAN and Tailscale mesh port exposure (--host 0.0.0.0) for live browser inspection on phone and workstation. |
| **specter-faker** | `/specter-faker` | `[generate\|card\|config] [-n count] [-d domain]` | Synthetic persona generator with compliant Vietnamese CCCD validation,. |
| **specter-help** | `/specter-help` | *(None)* | Quick-reference card and master cheatsheet for all Specter CLI commands, interactive shell scopes,. |
| **specter-runner** | `/specter-runner` | `[status\|start\|stop\|restart\|logs\|probe]` | Kernel-level Win32 Job Object supervisor & daemon controller (port 8765). |
| **specter-security** | `/specter-security` | `[audit\|harden\|check-ports]` | Enterprise standard and autonomous runbook for cloud server hardening,. |
<!-- SKILLS_CATALOG_END -->

## CLI Cheatsheet

```powershell
# Health, Diagnostics & Dependencies
specter doctor                          # Full dependency check (Scoop, Cloudflared, SSH, Antidetect Chromium)
specter status                          # Holistic ecosystem health dashboard (Cloud, Bridge, Browser, Runner)

# Cloud Control Plane & Fleet Pairing
specter cloud whoami                    # Inspect paired device identity & tenant
specter cloud login --name "Node-1"     # Pair workstation with Tuquet Cloud fleet
supabase db push                       # Apply remote schema migrations (requires bridge --http)

# Network Bridge & Tunnels
specter bridge status                   # Check active tunnels & ports (1080, 8118, 2222)
specter bridge start [server]           # Start SOCKS5 proxy on 127.0.0.1:1080
specter bridge start [server] --http    # Start with embedded HTTP adapter (8118)
specter bridge stop [server | all]      # Terminate active tunnels cleanly
specter bridge check                    # Validate bridge.json schema against conflicts

# Process Supervisor & Daemon (Port 8765)
specter runner status                   # Check worker daemon on port 8765
specter runner start -d                 # Spawn background worker daemon (Detached Process)
specter runner logs -f                  # Stream worker execution traces in real-time
specter runner stop                     # Gracefully terminate supervisor & child process tree
specter runner probe                    # Query hardware specs and driver capabilities

# Antidetect Chromium Runtime
specter browser status                  # Inspect Antidetect engine, active version & path
specter browser list                    # List installed versions with active badge (148, LTS)
specter browser search                  # Query upstream manifest releases & stability status
specter browser use <ver>               # Switch active Antidetect version (e.g. 148, lts)
specter browser install [ver]           # Download & install Antidetect Chromium (defaults to 148)
specter browser path                    # Print binary path for CDP automation drivers
specter browser clean                   # Reclaim disk space by purging runtime binaries

# Automa Workflow Engine
specter automa list                     # List locally registered workflows
specter automa run <flow.json>          # Execute workflow headlessly (--headless --timeout 60)
specter automa inspect <flow.json>      # Validate workflow DAG syntax without browser
specter automa studio                   # Launch visual Web Studio in browser

# Synthetic Persona & Identity (Faker)
specter faker card                      # Display rich identity card with valid CCCD
specter faker generate -n 10 -f table   # Generate 10 personas in clean terminal table
specter faker generate -n 100 -f csv -o seed.csv  # Export personas to CSV
specter faker config -d flowup.io.vn    # Update default email domain pool

# Universal & System
specter upgrade                         # Update CLI to latest release via Scoop
specter mcp                             # Start Model Context Protocol (MCP) stdio server
specter shell [scope]                   # Launch interactive REPL directly in target scope
```

## Interactive Scoped REPL

Launch interactive shell: `specter`
```text
specter> use bridge        # Switch to bridge scope
specter(bridge)> start     # Start default VPS bridge
specter(bridge)> back      # Return to root shell
specter> exit              # Exit shell
```

### REPL Shortcuts
| Command / Key | Target / Behavior | Example |
| :--- | :--- | :--- |
| `use <scope>` | Switch active service context | `use bridge`, `use automa`, `use faker` |
| `<scope>` | Direct switch shortcut at Global scope | `bridge`, `automa`, `runner`, `cloud`, `browser`, `faker` |
| `back` / `cd ..` | Return to previous / Global scope | `back` (from `specter(bridge)>` to `specter(global)>`) |
| `exit` / `quit` | Exit sub-scope (or exit CLI if in Global) | `exit` or `Ctrl+D` |
| `clear` / `cls` | Clear terminal screen | `clear` |
| `Tab` | Context-aware autocompletion | Auto-scans workflows in `~/.specter/automa/workflows/` |
| **Prefix Tolerance** | Inside `specter(bridge)>`, both `status` and `bridge status` work identically |

## SSOT 5 Pillars (`~/.specter/`)

All ecosystem data resolves strictly to:
- `~/.specter/system/`: Machine ID (`.machine_id`), device token (`.identity.json`), CLI history.
- `~/.specter/bridge/`: Multi-VPS mesh configuration (`bridge.json`), daemon PIDs (`pids/`).
- `~/.specter/browser/`: Chromium binaries (`runtimes/`), sandboxed profiles (`profiles/`).
- `~/.specter/automa/`: Workflow definitions (`workflows/`), execution DB (`automa.sqlite`).
- `~/.specter/faker/`: Synthetic data schemas (`faker.json`), demographic templates.

## 📖 Full Documentation Portal

Comprehensive installation SOPs, architecture deep-dives, and complete 68-command reference:
👉 **[https://docs.tuquet.com/en/specter/commands/](https://docs.tuquet.com/en/specter/commands/)**

## Boundaries

One-shot report, changes nothing. Do not write flags, alter files, or modify system configuration.

