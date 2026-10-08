# Specter CLI — Master Cheat Sheet & Scope Manual

> **Unified Developer CLI & Interactive Scoped Shell for the Tuquet Ecosystem**  
> Single Source of Truth (SSOT): `~/.specter/`

---

## ⚡ Canonical Command Reference (SSOT)

> [!IMPORTANT]
> **Single Source of Truth**: All living operational commands, command flags, workload recipes, and automated runbooks are maintained centrally in the **[Specter Documentation Portal](https://tuquet.github.io/docs/commands/)** and **[Specter Skills (`tuquet/skills`)](https://github.com/tuquet/skills)**.
> - In AI Agent terminals (Antigravity, Claude Code, Cursor), invoke: **`/specter-help`**
> - In web browser: View the [**Specter Master CLI Reference**](https://tuquet.github.io/docs/commands/) & [**`specter-help` Skill Card**](https://github.com/tuquet/skills/blob/main/skills/specter-help/SKILL.md)

---

## 🧭 Scoped REPL Quick Navigation

Launch interactive scoped shell:
```bash
specter                      # Launches REPL at Global scope
specter <scope>              # Direct jump to target scope (e.g., specter bridge)
```

### REPL Shortcuts & Navigation Keys

| Key / Command | Target / Behavior | Example |
| :--- | :--- | :--- |
| `use <scope>` | Switch active service context | `use bridge`, `use automa`, `use faker` |
| `<scope>` | Direct switch shortcut (at Global scope) | `bridge`, `automa`, `runner`, `cloud`, `browser`, `faker` |
| `back` / `cd ..` | Return to previous / Global scope | `back` (from `specter(bridge)>` to `specter(global)>`) |
| `exit` / `quit` / `q` | Exit current sub-scope (or exit CLI if in Global) | `exit`, `q`, or `Ctrl+D` |
| `clear` / `cls` | Clear terminal canvas | `clear` |
| `help` / `?` | Context-aware help for active scope | `help` |
| `Tab` | Smart autocompletion (commands, flags, vault workflows) | `run <Tab>` scans `~/.specter/automa/workflows/` |
| **Prefix Tolerance** | Inside `specter(bridge)>`, both `status` and `bridge status` work identically |

---

## 🗺️ Master Scopes & Dedicated Skills Directory

Each interactive scope maps 1-to-1 with a specialized, atomic **Specter Skill**:

| Scope | Prompt Indicator | Subsystem Role | Dedicated AI Agent Skill |
| :--- | :--- | :--- | :--- |
| **`global`** | `[global]` (Cyan) | Platform health, updates, MCP server | [`/specter`](https://github.com/tuquet/skills/blob/main/skills/specter/SKILL.md) |
| **`bridge`** | `[bridge]` (Magenta) | Multi-VPS mesh, Cloudflare tunnels, SOCKS5 & HTTP 8118 | [`/specter-bridge`](https://github.com/tuquet/skills/blob/main/skills/specter-bridge/SKILL.md) |
| **`browser`** | `[browser]` (Blue) | Isolated Chromium LTS runtime, profile sandboxes | [`/specter-browser`](https://github.com/tuquet/skills/blob/main/skills/specter-browser/SKILL.md) |
| **`automa`** | `[automa]` (Amber) | Visual workflow execution, CDP engine, local SQLite | [`/specter-automa`](https://github.com/tuquet/skills/blob/main/skills/specter-automa/SKILL.md) |
| **`runner`** | `[runner]` (Green) | Background supervisor daemon (8765), Win32 Job Object | [`/specter-runner`](https://github.com/tuquet/skills/blob/main/skills/specter-runner/SKILL.md) |
| **`cloud`** | `[cloud]` (Purple) | Supabase device enrollment, tenant pairing & DB push | [`/specter-cloud`](https://github.com/tuquet/skills/blob/main/skills/specter-cloud/SKILL.md) |
| **`faker`** | `[faker]` (Cyan) | Synthetic persona generator (CCCD, addresses, emails) | [`/specter-faker`](https://github.com/tuquet/skills/blob/main/skills/specter-faker/SKILL.md) |

---

## 📁 Canonical Storage Architecture (`~/.specter/`)

All ecosystem data resolves strictly to 5 canonical microservice domains under `~/.specter/`:
- `~/.specter/system/`: Machine ID (`.machine_id`), device credentials (`.identity.json`), CLI history (`history.txt`).
- `~/.specter/bridge/`: Multi-VPS mesh configuration (`bridge.json`), background daemon PIDs (`pids/`).
- `~/.specter/browser/`: Standalone Chromium LTS binaries (`runtimes/`), sandboxed user profiles (`profiles/`).
- `~/.specter/automa/`: Workflow definitions (`workflows/`), local execution DB (`automa.sqlite`).
- `~/.specter/faker/`: Generator schema definitions, template seeds (`faker.json`).
