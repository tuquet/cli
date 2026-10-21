<div align="center">
  <img src="https://tuquet.com/icons/cli.svg" width="76" height="76" alt="CLI Logo" />
  <h1>Tuquet CLI (`tq`)</h1>
  <p><strong>Unified Command-Line Interface for the Tuquet Autonomous Agent Ecosystem</strong></p>

  <p>
    <a href="https://docs.tuquet.com/en/cli/commands/"><img src="https://img.shields.io/badge/Docs-CLI%20Reference-blue.svg" alt="Documentation Hub" /></a>
    <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-Clap%2FRatatui-orange.svg" alt="Rust" /></a>
    <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License" /></a>
  </p>

  <p>
    <strong><a href="https://docs.tuquet.com/en/cli/commands/">📖 Explore Complete CLI Reference at Documentation SSOT &rarr;</a></strong>
  </p>
</div>

---

## 📌 Overview

**Tuquet CLI** (`tq`) is the single unified entrypoint for managing workstation connectivity, distributed compute nodes, browser automation workflows, and cloud authentication. Built with pure Rust for sub-millisecond startup times and rock-solid reliability.

Detailed reference guides and examples are maintained at **[docs.tuquet.com/en/cli/commands/](https://docs.tuquet.com/en/cli/commands/)**.

---

## ⚡ 4 Core Commands

Adhering strictly to **KISS & YAGNI**, the CLI exposes 4 focused, orthogonal command families:

```bash
tq <command> [subcommand] [options]
```

| Family | Subcommands | Purpose |
|---|---|---|
| **`tq auth`** | `login`, `logout`, `whoami`, `tenant` | Cloud identity and multi-tenant context management |
| **`tq worker`** | `start`, `stop`, `status`, `logs` | Local workstation compute daemon & device enrollment |
| **`tq automation`** | `run`, `list`, `inspect` | Browser automation workflows & safe profile execution |
| **`tq gateway`** | `start`, `stop`, `status`, `probe` | Multi-VPS network mesh, SOCKS5/HTTP tunnels & leak shield |

---

## 📦 Installation

```powershell
# Windows (Scoop)
scoop bucket add tuquet https://github.com/tuquet/scoop-bucket
scoop install tuquet
```

```bash
# macOS & Linux (curl)
curl -fsSL https://raw.githubusercontent.com/tuquet/cli/main/install.sh | sh
```

---

## 🛠️ Development

```bash
# Build release binary
cargo build --release

# Run tests
cargo test
```

For full documentation and advanced flags, visit the **[Official CLI Documentation](https://docs.tuquet.com/en/cli/commands/)**.
