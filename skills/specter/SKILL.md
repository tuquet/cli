---
name: specter
description: >
  Master orchestrator and platform health dashboard (`specter status`). Inspects
  SSOT root `~/.specter/`, machine identity, active listeners, and database state.
  Trigger: /specter, "specter status", "tuquet health", "platform status", "check tuquet".
argument-hint: "[status|dashboard]"
license: MIT
---

# Specter Platform Health (`specter`)

Zero fluff. Holistic platform heartbeat in one glance. Inspects all services, machine identity, and canonical storage pillars under `~/.specter/`.

## The Ladder

Check state in order:
1. **SSOT Root**: `~/.specter/` directory structure must exist and be accessible.
2. **Machine Identity**: `.machine_id` and `.identity.json` must be valid.
3. **Active Listeners**: SOCKS5 (1080), HTTP adapter (8118), SSH (2222).
4. **Daemons & Databases**: Runner supervisor, SQLite database.

## Scan

Run holistic diagnostics:
```powershell
$ErrorActionPreference = 'SilentlyContinue'; specter status
```

Or verify local SSOT directories directly:
```powershell
Test-Path "$HOME\.specter\system", "$HOME\.specter\automa", "$HOME\.specter\browser", "$HOME\.specter\bridge", "$HOME\.specter\faker"
```

## Output Contract

Report strictly in this format:

```text
[SSOT] Root: ~/.specter/ (OK) | Identity: <machine_id>
[BRIDGE] 1080: <UP|DOWN> | 8118: <UP|DOWN> | 2222: <UP|DOWN>
[AUTOMA] Workflows: <N> | SQLite: <OK|ERR>
[BROWSER] Chromium: <READY|MISSING> (<version>)
[RUNNER] Daemon: <ACTIVE|INACTIVE>
Verdict: All systems nominal. Ship. / <N> service(s) degraded. Run /specter-<service> to fix.
```

## Interactive Shell

Launch the interactive REPL shell:
```powershell
specter
```
- Switch: `use bridge`, `use automa`, `use browser`, `use runner`, `use faker`, `use cloud`.
- Exit: `exit` or `back`.

## Boundaries

One-shot report, changes nothing. Do not restart services automatically without user prompt. To fix an individual subsystem, invoke its dedicated skill (`/specter-bridge`, `/specter-browser`, `/specter-automa`).
