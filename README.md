<div align="center">
  <img src="./assets/logo.svg" width="76" height="76" alt="CLI Logo" />
  <h1>CLI</h1>
  <p><strong>Interactive Scoped Shell &amp; Unified Automation Terminal in Rust</strong></p>

  <p>
    <a href="https://github.com/tuquet/scoop-bucket"><img src="https://img.shields.io/badge/Scoop-tuquet-brightgreen.svg" alt="Scoop" /></a>
    <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-Clap%2FRatatui-orange.svg" alt="Rust" /></a>
    <img src="https://img.shields.io/badge/Shell-Interactive%20Scoped-blue.svg" alt="Interactive Shell" />
    <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License" /></a>
  </p>
</div>

---

> High-performance developer CLI & interactive scoped shell written in Rust, providing a unified terminal interface to orchestrate and manage services across the automation ecosystem.

## 🚀 Installation

### 1. Windows via Scoop (Recommended)
```powershell
scoop bucket add tuquet https://github.com/tuquet/scoop-bucket
scoop install tuquet
```

### 2. Build from Source (Cargo)
```powershell
git clone https://github.com/tuquet/cli.git
cd cli
cargo build --release
# Binary generated at: target/release/tuquet.exe
```

---

## 🖥️ Interactive Scoped Shell

Launch the interactive shell by running `tuquet` in your terminal. Features **Smart Tab-Completion**, hierarchical scope management, and command history persistence:

```console
tuquet
```

### 1. Direct Scoped Launch
Open the shell and enter a specific service context directly:

```console
tuquet automa    # Enter Automa scope: tuquet(automa)>
tuquet runner    # Enter Runner scope: tuquet(runner)>
tuquet cloud     # Enter Cloud scope:  tuquet(cloud)>
tuquet browser   # Enter Browser scope: tuquet(browser)>
```

### 2. Shell Navigation & Keyboard Shortcuts
- **Switch Scope**: `use <automa | runner | cloud | browser | global>`
- **Return to Global Scope**: Type `back`, `cd ..`, or `exit` (when in a sub-scope)
- **Exit Program**: Type `exit` or `quit` at the Global scope (or press `Ctrl+D`)
- **Clear Screen**: `clear` or `cls`
- **Context Help**: `help` or `?`
- **Smart Autocomplete (Tab)**: Auto-suggests commands, argument flags (`--headless`, `--timeout`), and **dynamically scans workflows** in `~/.tuquet/workflows/`.
- **Prefix Tolerance**: When inside `tuquet(automa)>`, both `run flow.json` and `automa run flow.json` execute accurately.

---

## 📋 CLI Command Reference

Supports direct execution from scripts, terminals, or CI pipelines without entering the interactive shell:

### 1. Global & State Management
```console
tuquet status               # Comprehensive health check: Browser, Runner Daemon, Cloud Pairing
tuquet login [token]        # Authenticate workstation with Cloud Control Plane
tuquet whoami               # Inspect workstation identity and pairing credentials
tuquet logout               # Disconnect and revoke local cloud session credentials
```

### 2. Browser Automation (Automa Engine)
```console
# Run a workflow (supports .json file path or workflow ID stored in vault/DB)
tuquet automa run ./my_workflow.json --headless
tuquet automa run <workflow-id> --timeout 60

# Manage workflow vault
tuquet automa list                      # List workflows in vault and local database
tuquet automa list "scraping"           # Search workflows by keyword
tuquet automa inspect ./my_flow.json    # Validate workflow node-graph structure
tuquet automa import ./backup.json      # Import workflow into local vault storage
tuquet automa export <workflow-id>      # Export workflow to JSON file
tuquet automa delete <workflow-id>      # Remove workflow from local storage

# Launch visual Web Studio in browser
tuquet automa studio
```

### 3. Daemon & Cloud Worker (Runner Engine)
```console
tuquet runner start --port 8765         # Start Runner Daemon in foreground
tuquet runner status                    # Check local Runner daemon health
tuquet runner probe                     # Inspect hardware specs & driver capabilities
tuquet runner export-openapi spec.json  # Export OpenAPI v3 specification to file
tuquet runner setup-ext                 # Developer utility to launch browser pre-loaded with extension
```

### 4. Isolated Chromium Runtime Management
Tuquet manages a dedicated, pure open-source Chromium LTS runtime, completely decoupled from the OS default browser:

```console
tuquet browser status                   # Check runtime version, install path, and disk usage
tuquet browser install                  # Automatically download and configure isolated Chromium
tuquet browser install --force          # Reinstall runtime if corrupted
tuquet browser path                     # Print absolute executable path to chromium
tuquet browser clean                    # Purge Chromium runtime to free disk space
```

### 5. Model Context Protocol (MCP) Server
Tuquet ships with a native, zero-dependency MCP stdio server complying with JSON-RPC 2.0 (spec `2024-11-05`), empowering AI Coding Agents (such as Google Antigravity, Claude Code, Cursor) to autonomously orchestrate browser automations and probe runner nodes:

```console
tuquet mcp                              # Launch JSON-RPC 2.0 stdio MCP server
```

**Exposed MCP Tools:**
- `tuquet_status`: Inspect unified health across Cloud, Runner, and Browser.
- `tuquet_workflow_list`: Query workflows stored in SQLite database and Vault (`~/.tuquet/workflows/`).
- `tuquet_workflow_inspect`: Deeply analyze node connections, triggers, variables, and parameters of any workflow.
- `tuquet_workflow_run`: Execute automation workflows in headless or visible browser with runtime variables.
- `tuquet_runner_probe`: Query hardware specifications and driver capabilities.
- `tuquet_cloud_whoami`: Query cloud pairing identity and tenant enrollment.
- `tuquet_browser_status`: Inspect dedicated Chromium runtime path, version, and footprint.

---

## 🏛️ Single Source of Truth (SSOT) & Storage Hierarchy

All configuration, runtimes, and local data across the ecosystem resolve strictly to the canonical directory:

```
~/.tuquet/
├── workflows/           # Local Workflow Vault (.json scenario files)
├── runtimes/            # Dedicated Isolated Open-Source Chromium Runtimes
├── data/
│   └── tuquet.sqlite    # Embedded SQLite database (Jobs, Logs, Variables, Profiles)
└── history.txt          # Command history for Tuquet Interactive Shell
```

---

## 🌐 Dev Tooling & Interactive Endpoints

When the Runner Daemon is active (default port `8765`), interactive developer interfaces are immediately accessible:

| Interface / Endpoint | URL Address | Description |
| :--- | :--- | :--- |
| 📑 **Swagger UI (API Docs)** | **`http://127.0.0.1:8765/swagger-ui`** | Interactive OpenAPI v3 interface to test REST APIs live. |
| 📄 **OpenAPI Spec (JSON)** | **`http://127.0.0.1:8765/api-docs/openapi.json`** | Raw OpenAPI JSON spec for codegen or Postman/Bruno sync. |
| 🎨 **Web Studio Canvas** | **`http://127.0.0.1:8765/studio/`** | Visual node-graph canvas for drag-and-drop workflow editing. |
| 📡 **SSE Telemetry** | **`http://127.0.0.1:8765/api/v1/events`** | Server-Sent Events stream for real-time job execution logs. |
| ⚡ **WebSocket Control** | **`ws://127.0.0.1:8765/api/v1/ws`** | Low-latency bi-directional control channel (Pause/Resume/Kill). |

---

## 🛑 Architectural Design Principles

1. **Decoupled Layers**:
   - `core`: Pure business logic, independent of outer I/O layers.
   - `infrastructure`: Implements SQLite (`rusqlite`), file I/O, and Chromium process management.
   - `api`: Axum HTTP, WebSocket, SSE endpoints, and request validation.
   - `commands`: Command handlers return `Result<(), Box<dyn Error>>` without calling `std::process::exit` to protect shell session continuity.
2. **Async Concurrency**:
   - Executes on top of the `tokio` multi-threaded runtime.
   - CPU-bound tasks are offloaded via `tokio::task::spawn_blocking`.
3. **Zero-Knowledge Security**:
   - Credentials stored with AES-256 encryption and HMAC-SHA256 integrity verification.
   - Browser extensions decrypt sensitive secrets in-memory using user passphrase.

---

## 🔗 Ecosystem References

CLI is the primary terminal orchestrator connecting the specialized modules of the automation ecosystem:

| Repository / Module | GitHub Repository & README | Core Role & Architecture Link |
| :--- | :--- | :--- |
| **Automa** | [📘 `github.com/tuquet/automa`](https://github.com/tuquet/automa#readme) | Browser Extension manifest, Background Worker & Web Studio canvas. |
| **Runner** | [📘 `github.com/tuquet/runner`](https://github.com/tuquet/runner#readme) | Universal distributed execution node (`runner`), kernel process supervision & driver router. |
| **Browser** | [📘 `github.com/tuquet/browser`](https://github.com/tuquet/browser#readme) | Chromium LTS runtime management, multi-profile sandbox & CDP stealth engine. |
| **Cloud** | [📘 `github.com/tuquet/cloud`](https://github.com/tuquet/cloud#readme) | Supabase Multi-Tenant foundation, device enrollment RPC, and pairing management. |
| **Lib** | [📘 `github.com/tuquet/lib`](https://github.com/tuquet/lib#readme) | TypeScript & Vue 3 shared monorepo (`vue-ui`, `vue-table`, `md-export`, `extension-runner`, `lunar`). |
| **Scoop Bucket** | [📘 `github.com/tuquet/scoop-bucket`](https://github.com/tuquet/scoop-bucket#readme) | Official Scoop distribution channel for Tuquet unified CLI package. |
| **Claude-Agy** | [📘 `github.com/tuquet/claude-agy`](https://github.com/tuquet/claude-agy#readme) | Claude Code CLI integration powered by Google Antigravity OAuth quotas. |
| **Skills** | [📘 `github.com/tuquet/skills`](https://github.com/tuquet/skills#readme) | Automation skillsets, runbooks, and recipes for AI Coding Agents. |

---

## 📄 License

Distributed under the [MIT License](LICENSE).

---

<div align="center">
  <samp>
    <a href="https://tuquet.github.io">Portfolio</a> •
    <a href="https://tuquet.github.io/cv">CV &amp; Resume</a> •
    <a href="https://tuquet.github.io/automa">Automa Studio</a> •
    <a href="https://tuquet.github.io/lib">Component Lab</a> •
    <a href="https://github.com/tuquet/scoop-bucket">Scoop Bucket</a>
  </samp>
</div>
