<div align="center">
  <img src="https://tuquet.github.io/icons/cli.svg" width="80" height="80" alt="CLI Logo" />
  <h1>Specter Master CLI (`specter`)</h1>
  <p><strong>Interactive Scoped Shell, Service Multiplexer &amp; High-Performance Automation Terminal in Rust</strong></p>

  <p>
    <a href="https://github.com/tuquet/scoop-bucket"><img src="https://img.shields.io/badge/Scoop-specter-brightgreen.svg" alt="Scoop" /></a>
    <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-Clap%2FRatatui-orange.svg" alt="Rust" /></a>
    <img src="https://img.shields.io/badge/Shell-Interactive%20Scoped-blue.svg" alt="Interactive Shell" />
    <a href="CHEATSHEET.md"><img src="https://img.shields.io/badge/Docs-Navigation%20Card-purple.svg" alt="Navigation Card" /></a>
    <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License" /></a>
  </p>
  <p><strong><a href="CHEATSHEET.md">📖 View Scope Navigation Card &rarr;</a> • <a href="https://github.com/tuquet/skills/blob/main/skills/tuquet-help/SKILL.md">⚡ Master Cheatsheet (`/tuquet-help`) &rarr;</a></strong></p>
</div>

---

## 📋 Executive Summary & Business ROI

Modern automation environments are plagued by fragmented developer tooling: separate Python scripts for scraping, uncoordinated Node daemons, manual SSH port forwards, and siloed CLI binaries.

**Specter Master CLI** (`specter`) unifies the entire automation, process supervision, and cloud control plane into a single, high-performance terminal compiled in native Rust:

| Strategic Pillar | The Traditional Problem | The Specter CLI Solution | Measurable Business ROI |
| :--- | :--- | :--- | :--- |
| **Tool Fragmentation** | Developers juggle 5+ separate CLIs and scripts across different terminals. | **Interactive Scoped REPL** (`specter`) multiplexes all services into a unified context-aware shell. | **Zero Context Switching**; single binary orchestrates network, browser, and daemons. |
| **Resource Overhead** | Node/Electron CLI wrappers consume 100MB+ RAM merely idling in background. | **Native Rust Binary** with sub-10MB footprint and instantaneous sub-millisecond startup. | **90% Resource Savings**; runs smoothly on lightweight developer workstations. |
| **Configuration Sprawl** | Config files scattered across home folders, `.env` files, and registry keys. | **Single Source of Truth (SSOT)** strictly resolving to `~/.specter/` across 5 pillars. | **Zero Config Drift**; predictable, reproducible environments across multi-PC fleets. |
| **AI Agent Ergonomics** | LLM copilots struggle with complex, brittle shell command syntax. | **Atomic Agent Skills Standard** (`tuquet-*`) with deterministic output contracts and slash commands. | **100% Reliable Automation**; zero agent hallucination when executing operational tasks. |

---

## 🖥️ Interactive Scoped REPL

Launch the interactive shell by running `specter` in any terminal. Features **Smart Tab-Completion**, hierarchical scope switching, and command history persistence:

```console
specter
```

```text
============================================================
  🛸 Specter Unified Interactive Shell (v1.0.0)
============================================================
Type 'help' for commands, 'use <service>' to switch scope, 'exit' to quit.
Tip: Press [Tab] for hierarchical suggestions and workflow completions.

specter> use bridge
specter(bridge)> start my-vps --http
[Bridge] SOCKS5 tunnel established on 127.0.0.1:1080
[Bridge] HTTP-to-SOCKS5 adapter active on 127.0.0.1:8118
specter(bridge)> back

specter> status
[System] Machine ID: 8f4c-e812-3b91  |  Root: ~/.specter/
[Bridge] SOCKS5: Active (1080)       |  HTTP: Active (8118)
[Automa] Local Workflows: 14 loaded  |  Runtime: Chromium LTS (Ready)
specter> exit
```

---

## ⚡ Command Reference & AI Agent Skills (SSOT)

To eliminate documentation drift and guarantee always-accurate syntax, all operational commands, recipes, and cheatsheets are maintained centrally in **[Tuquet Skills (`tuquet/skills`)](https://github.com/tuquet/skills)**:

| Domain / Pillar | Dedicated Skill | Slash Command | Scope & Capabilities |
| :--- | :--- | :--- | :--- |
| **Platform Health** | [`specter`](https://github.com/tuquet/skills/blob/main/skills/specter/SKILL.md) | `/specter` | Holistic ecosystem health check (`specter status`) & SSOT validation. |
| **Master Cheatsheet** | [`specter-help`](https://github.com/tuquet/skills/blob/main/skills/specter-help/SKILL.md) | `/specter-help` | One-shot cheatsheet, REPL shortcuts, and universal tool reference. |
| **Network Bridge** | [`specter-bridge`](https://github.com/tuquet/skills/blob/main/skills/specter-bridge/SKILL.md) | `/specter-bridge` | Multi-VPS tunnels, SOCKS5 (1080), HTTP adapter (8118), and `git spush`. |
| **Browser Runtime** | [`specter-browser`](https://github.com/tuquet/skills/blob/main/skills/specter-browser/SKILL.md) | `/specter-browser` | Isolated Chromium LTS provisioning, sandbox cleanup, and binary paths. |
| **Workflow Engine** | [`specter-automa`](https://github.com/tuquet/skills/blob/main/skills/specter-automa/SKILL.md) | `/specter-automa` | Headless visual workflow execution, DAG validation, and SQLite runs. |
| **Process Supervisor** | [`specter-runner`](https://github.com/tuquet/skills/blob/main/skills/specter-runner/SKILL.md) | `/specter-runner` | Background worker daemon (port 8765), Win32 Job Object tree supervision. |
| **Cloud Control Plane** | [`specter-cloud`](https://github.com/tuquet/skills/blob/main/skills/specter-cloud/SKILL.md) | `/specter-cloud` | Device fleet enrollment (`specter login/whoami`) and Supabase DB push. |
| **Synthetic Identity** | [`specter-faker`](https://github.com/tuquet/skills/blob/main/skills/specter-faker/SKILL.md) | `/specter-faker` | Synthetic personas, validated CCCDs, and custom email domain pools. |
| **Cloud Hardening** | [`specter-security`](https://github.com/tuquet/skills/blob/main/skills/specter-security/SKILL.md) | `/specter-security` | Enterprise cloud server hardening, Zero Inbound Ports, disk hygiene. |

---

## 🚀 Quick Start & Installation

### 1. Universal Web Installer (Zero Dependencies)
For clean workstations without developer tools:

**Windows (PowerShell):**
```powershell
powershell -c "irm https://raw.githubusercontent.com/tuquet/cli/main/install.ps1 | iex"
```

**macOS & Linux (curl / sh):**
```bash
curl -fsSL https://raw.githubusercontent.com/tuquet/cli/main/install.sh | sh
```

### 2. Via Scoop Package Manager (Recommended for Windows Devs)
```console
# Register the Tuquet Scoop Bucket
scoop bucket add tuquet https://github.com/tuquet/scoop-bucket

# Install the Unified Master CLI
scoop install specter
```

### 3. Build from Source (Cargo)
```console
git clone https://github.com/tuquet/cli.git
cd cli
cargo build --release
# Executable generated at: target/release/specter (or specter.exe on Windows)
```

---

## 📄 License

Distributed under the [MIT License](LICENSE).
