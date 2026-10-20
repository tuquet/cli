# Tuquet CLI — Master Cheat Sheet & Scope Manual

> **Unified Developer CLI & Interactive Scoped Shell for the Tuquet Ecosystem**  
> Single Source of Truth (SSOT): `~/.tuquet/`

---

## ⚡ Canonical Command Reference (SSOT)

> [!IMPORTANT]
> **Single Source of Truth**: All living operational commands, command flags, workload recipes, and automated runbooks are maintained centrally in the **[Tuquet Documentation Portal](https://docs.tuquet.com/en/tuquet/commands/)** and **[Tuquet Skills (`tuquet/skills`)](https://github.com/tuquet/skills)**.
> - In AI Agent terminals (Antigravity, Claude Code, Cursor), invoke: **`/tuquet-help`**
> - In web browser: View the [**Tuquet Master CLI Reference**](https://docs.tuquet.com/en/tuquet/commands/) & [**`tuquet-help` Skill Card**](https://github.com/tuquet/skills/blob/main/skills/tuquet-help/SKILL.md)

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

Each interactive scope maps 1-to-1 with a specialized, atomic **Tuquet Skill**:

| Scope | Prompt Indicator | Subsystem Role | Dedicated AI Agent Skill |
| :--- | :--- | :--- | :--- |
| **`global`** | `[global]` (Cyan) | Platform health, updates, MCP server | [`/tuquet`](https://github.com/tuquet/skills/blob/main/skills/tuquet/SKILL.md) |
| **`bridge`** | `[bridge]` (Magenta) | Multi-VPS mesh, Cloudflare tunnels, SOCKS5 & HTTP 8118 | [`/tuquet-bridge`](https://github.com/tuquet/skills/blob/main/skills/tuquet-bridge/SKILL.md) |
| **`browser`** | `[browser]` (Blue) | Isolated Chromium LTS runtime, profile sandboxes | [`/tuquet-browser`](https://github.com/tuquet/skills/blob/main/skills/tuquet-browser/SKILL.md) |
| **`automa`** | `[automa]` (Amber) | Visual workflow execution, CDP engine, local SQLite | [`/tuquet-automa`](https://github.com/tuquet/skills/blob/main/skills/tuquet-automa/SKILL.md) |
| **`runner`** | `[runner]` (Green) | Background supervisor daemon (port 8765), auto-cleanup | [`/tuquet-runner`](https://github.com/tuquet/skills/blob/main/skills/tuquet-runner/SKILL.md) |
| **`cloud`** | `[cloud]` (Purple) | Supabase device enrollment, tenant pairing & DB push | [`/tuquet-cloud`](https://github.com/tuquet/skills/blob/main/skills/tuquet-cloud/SKILL.md) |
| **`faker`** | `[faker]` (Cyan) | Synthetic persona generator (CCCD, addresses, emails) | [`/tuquet-faker`](https://github.com/tuquet/skills/blob/main/skills/tuquet-faker/SKILL.md) |

---

## 📁 Canonical Storage Architecture (`~/.tuquet/`)

All ecosystem data resolves strictly to 5 canonical microservice domains under `~/.tuquet/`:
- `~/.tuquet/system/`: Machine ID (`.machine_id`), device credentials (`.identity.json`), CLI history (`history.txt`).
- `~/.tuquet/bridge/`: Multi-VPS mesh configuration (`bridge.json`), background daemon PIDs (`pids/`).
- `~/.tuquet/browser/`: Standalone Chromium LTS binaries (`runtimes/`), sandboxed user profiles (`profiles/`).
- `~/.tuquet/automa/`: Workflow definitions (`workflows/`), local execution DB (`automa.sqlite`).
- `~/.tuquet/faker/`: Generator schema definitions, template seeds (`faker.json`).
