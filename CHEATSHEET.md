# CLI — Master Cheat Sheet & Scope Manual

> **Unified Developer CLI & Interactive Scoped Shell for the Ecosystem**  
> Single Source of Truth (SSOT): `~/.tuquet/`

---

## ⚡ Canonical Command Reference (SSOT)

> [!IMPORTANT]
> **Single Source of Truth**: All living operational commands, command flags, workload recipes, and automated runbooks are maintained centrally in the **[Documentation Portal](https://docs.tuquet.com/en/tuquet/commands/)** and **[Skills (`tuquet/skills`)](https://github.com/tuquet/skills)**.
> - In AI Agent terminals (Antigravity, Claude Code, Cursor), invoke: **`/help`**
> - In web browser: View the [**Master CLI Reference**](https://docs.tuquet.com/en/tuquet/commands/) & [**`help` Skill Card**](https://github.com/tuquet/skills/blob/main/skills/help/SKILL.md)

---

## 🧭 Scoped REPL Quick Navigation

Launch interactive scoped shell:
```bash
tuquet                      # Launches REPL at Global scope
tuquet <scope>              # Direct jump to target scope (e.g., tuquet bridge)
```

### REPL Shortcuts & Navigation Keys

| Key / Command | Target / Behavior | Example |
| :--- | :--- | :--- |
| `use <scope>` | Switch active service context | `use bridge`, `use automa`, `use faker` |
| `<scope>` | Direct switch shortcut (at Global scope) | `bridge`, `automa`, `runner`, `cloud`, `browser`, `faker` |
| `back` / `cd ..` | Return to previous / Global scope | `back` (from `tuquet(bridge)>` to `tuquet(global)>`) |
| `exit` / `quit` / `q` | Exit current sub-scope (or exit CLI if in Global) | `exit`, `q`, or `Ctrl+D` |
| `clear` / `cls` | Clear terminal canvas | `clear` |
| `help` / `?` | Context-aware help for active scope | `help` |
| `Tab` | Smart autocompletion (commands, flags, vault workflows) | `run <Tab>` scans `~/.tuquet/automa/workflows/` |
| **Prefix Tolerance** | Inside `tuquet(bridge)>`, both `status` and `bridge status` work identically |

---

## 🗺️ Master Scopes & Dedicated Skills Directory

Each interactive scope maps 1-to-1 with a specialized, atomic **Skill**:

| Scope | Prompt Indicator | Subsystem Role | Dedicated AI Agent Skill |
| :--- | :--- | :--- | :--- |
| **`global`** | `[global]` (Cyan) | Platform health, updates, MCP server | [`/tuquet`](https://github.com/tuquet/skills/blob/main/skills/tuquet/SKILL.md) |
| **`bridge`** | `[bridge]` (Magenta) | Multi-VPS mesh, Cloudflare tunnels, SOCKS5 & HTTP 8118 | [`/bridge`](https://github.com/tuquet/skills/blob/main/skills/bridge/SKILL.md) |
| **`browser`** | `[browser]` (Blue) | Isolated Chromium LTS runtime, profile sandboxes | [`/browser`](https://github.com/tuquet/skills/blob/main/skills/browser/SKILL.md) |
| **`automa`** | `[automa]` (Amber) | Visual workflow execution, CDP engine, local SQLite | [`/automa`](https://github.com/tuquet/skills/blob/main/skills/automa/SKILL.md) |
| **`runner`** | `[runner]` (Green) | Background supervisor daemon (port 8765), auto-cleanup | [`/runner`](https://github.com/tuquet/skills/blob/main/skills/runner/SKILL.md) |
| **`cloud`** | `[cloud]` (Purple) | Supabase device enrollment, tenant pairing & DB push | [`/cloud`](https://github.com/tuquet/skills/blob/main/skills/cloud/SKILL.md) |
| **`faker`** | `[faker]` (Cyan) | Synthetic persona generator (CCCD, addresses, emails) | [`/faker`](https://github.com/tuquet/skills/blob/main/skills/faker/SKILL.md) |

---

## 📁 Canonical Storage Architecture (`~/.tuquet/`)

All ecosystem data resolves strictly to 5 canonical microservice domains under `~/.tuquet/`:
- `~/.tuquet/system/`: Machine ID (`.machine_id`), device credentials (`.identity.json`), CLI history (`history.txt`).
- `~/.tuquet/bridge/`: Multi-VPS mesh configuration (`bridge.json`), background daemon PIDs (`pids/`).
- `~/.tuquet/browser/`: Standalone Chromium LTS binaries (`runtimes/`), sandboxed user profiles (`profiles/`).
- `~/.tuquet/automa/`: Workflow definitions (`workflows/`), local execution DB (`automa.sqlite`).
- `~/.tuquet/faker/`: Generator schema definitions, template seeds (`faker.json`).
