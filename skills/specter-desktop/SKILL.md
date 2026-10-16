---
name: specter-desktop
description: >
  Manage Tuquet Tauri Desktop runtime lifecycle, in-process Axum bridge daemon,
  Tauri v2 core configuration, native IPC, and zero-zombie process supervision.
  Trigger: /specter-desktop, "specter desktop", "tauri dev", "build desktop", "desktop doctor", "package tuquet".
argument-hint: "[dev|build|doctor|clean|audit]"
license: MIT
---

# Specter Desktop Runtime (`specter-desktop`)

Dedicated Tauri v2 enterprise desktop runtime manager. Integrates native in-process Rust crates (`automa_core`, `specter-browser`, `specter-runner`), in-process Axum daemon (`127.0.0.1:8765`), and zero-zombie process supervision.

## Code & Specification Pointers (Code-as-Docs)

Read native source definitions on demand:
- Tauri Application Entrypoint: [desktop/src-tauri/src/main.rs](../../../desktop/src-tauri/src/main.rs)
- Core App Initialization: [desktop/src-tauri/src/lib.rs](../../../desktop/src-tauri/src/lib.rs)
- In-Process Tokio Daemon: [desktop/src-tauri/src/daemon.rs](../../../desktop/src-tauri/src/daemon.rs)
- Process Supervision & Graceful Cleanup: [desktop/src-tauri/src/supervisor.rs](../../../desktop/src-tauri/src/supervisor.rs)
- Native IPC Commands: [desktop/src-tauri/src/commands.rs](../../../desktop/src-tauri/src/commands.rs)
- Tauri Cargo Crate Linkage: [desktop/src-tauri/Cargo.toml](../../../desktop/src-tauri/Cargo.toml)
- Tauri Security Capabilities: [desktop/src-tauri/capabilities/default.json](../../../desktop/src-tauri/capabilities/default.json)
- Tauri Configuration: [desktop/src-tauri/tauri.conf.json](../../../desktop/src-tauri/tauri.conf.json)
- Official Documentation & Portal: https://specter.tuquet.com
- Embedded BFF & OpenAPI Generator Reference: [BFF Architecture Reference](./references/bff_architecture.md)
- Vendor Reference Skills: [vendor/claude-code-tauri-skills/tauri/](../../../vendor/claude-code-tauri-skills/tauri/) and [vendor/full-stack-tauri-skills/skills/](../../../vendor/full-stack-tauri-skills/skills/)

## The Ladder

Execute desktop tasks in strict order:
1. **Pre-flight Doctor**: Verify Node.js (>=18), pnpm, Rust 2024 toolchain, and OS Webview (WebView2 on Windows).
2. **SSOT Root Check**: Ensure `~/.specter/desktop/` canonical storage directory exists.
3. **Port Probe**: Verify port `8765` is either free or owned by an active Specter daemon.
4. **Crate Invariants**: Verify `automa_core`, `specter-browser`, and `runner` are linked in-process.
5. **Supervision Verification**: Verify process group cleanup hooks are active.

## Native Commands

```powershell
# 1. Inspect desktop environment health, webview engine, and crate dependencies
specter desktop doctor

# 2. Launch hot-reloading development shell
pnpm tauri dev

# 3. Compile optimized production installer (Windows NSIS .exe / MSI)
pnpm tauri build

# 4. Clean desktop build caches, webview state, and stale target artifacts
pnpm tauri clean
```

## Output Contract

Report status strictly in this format:

```text
[DESKTOP] Engine: Tauri v2 Core (Rust Edition 2024)
• Webview:    <WebView2 | WebKit> (<version>) (<READY|MISSING>)
• Daemon:     127.0.0.1:8765 (<IN-PROCESS|STANDALONE|PORT_BUSY>)
• Rust Core:  automa_core (<LINKED|ERR>), specter-browser (<LINKED|ERR>)
• SSOT Root:  ~/.specter/desktop/ (<OK>)
• Supervisor: <Active (Job Object / command-group) | Inactive>
Verdict: Desktop core runtime nominal. Ready to dev/ship.
```

## Architecture Invariants

1. **In-Process Embedding Invariant:**
   `desktop/src-tauri` MUST depend on library crates (`automa_core`, `specter-browser`, `specter-runner`) directly. NEVER spawn `specter.exe` as an untracked external sidecar.
2. **Zero-Zombie Process Guarantee:**
   When the Tauri window closes or an exit signal is received, all child Chromium processes, proxy bridges, and CDP sessions MUST be terminated synchronously via `command-group` or Windows Job Object.
3. **Dual-Channel IPC Convention:**
   - **Local Axum Bridge (`127.0.0.1:8765`)**: Embedded BFF Gateway used for all domain queries, state mutations, and SSE auto-invalidation.
   - **Tauri IPC (`tauri::command`)**: Exclusively used for native OS capabilities (window decorations, system tray, native file dialogs, global hotkeys).
4. **BFF Gateway & Zero-Supabase Invariant:**
   Frontend UI (React/Vue/Svelte) NEVER imports `@supabase/supabase-js` or talks directly to Supabase. All domain data routes exclusively through `http://127.0.0.1:8765/api/v1`.
5. **Local-First Speed & Silent Cloud Sync:**
   Reads and writes hit local SQLite (`~/.specter/automa/automa.sqlite`) instantly with sub-millisecond latency. The Rust daemon synchronizes state asynchronously with the Supabase Cloud Control Plane.
6. **Contract-First Code Generation:**
   Frontend TypeScript types and React Query hooks MUST be auto-generated from `http://127.0.0.1:8765/api-docs/openapi.json` via `orval` or `@hey-api/openapi-ts`. Manual API writing is prohibited.
7. **Terminology Guard:**
   Always use **`Tenant`** (never Workspace/Organization), **`Browser`** (never Profile), and **`Runner`** (never Worker/Bot).

## Boundaries

- Do NOT spawn standalone `specter.exe` subprocesses.
- Do NOT touch host machine's personal browser profiles (Edge, personal Chrome).
- Do NOT bypass `~/.specter/desktop/` for caching or local config.
- Do NOT import `@supabase/supabase-js` or execute direct Supabase queries in Desktop UI.
