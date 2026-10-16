---
name: specter-cicd
description: >
  CI/CD pipeline orchestration, local pre-flight gatekeeping, semantic version tagging, and GitHub Actions budget control.
  Trigger: /specter-cicd, "specter cicd", "cicd", "tuquet release", "release plan", "preflight check", "cut release", "ship release", "ci budget".
  Do NOT use for editing workflow JSON (use /specter-automa) or cloud database migrations (use /specter-cloud).
argument-hint: "[preflight|plan|tag|status|dispatch] [repo] [version]"
license: MIT
---

# Specter CI/CD & Release Pipeline (`specter-cicd`)

Deterministic release pipeline and zero-waste CI gatekeeper. Enforces local verification before tag creation to eliminate broken builds and conserve GitHub Actions minutes.

## The Ladder

Execute release steps in order:
1. **Pre-flight Audit**: Run ecosystem hygiene & security scan (`node scripts/check-hygiene.mjs`), verify clean git working tree, zero uncommitted changes, and active network bridge if pushing.
2. **Local Gatekeeper**: Run native tests and lints locally (`cargo test` or `pnpm test`). If anything fails or leaks are detected, abort immediately at zero cloud cost.
3. **Version Alignment**: Bump version in manifest (`Cargo.toml` or `package.json`) to match target semantic tag.
4. **Atomic Tagging**: Create annotated git tag (`git tag -a v<version> -m "release: v<version>"`).
5. **Secure Dispatch**: Route release tag through workstation proxy alias (`git spush origin v<version>`).
6. **Watch & Verify**: Monitor release build (`gh run watch` or Telegram alert via `@FlowupAI_bot`).

## Commands

### 1. Local Pre-flight Verification (Fail Fast at 0s CI Cost)

```powershell
# Mandatory Hygiene & Security Scan (Blocks local path leaks & real IPs):
node scripts/check-hygiene.mjs

# For Rust repositories (cli, runner, faker, browser):
$ErrorActionPreference = 'Stop'
cargo check --workspace
cargo test --workspace

# For Node.js / TypeScript repositories (automa, bot, cloud, lib, web):
pnpm test
pnpm run lint
```

### 2. Git Working Tree & Status Check

```powershell
# Ensure working directory is strictly clean:
git status --porcelain
```

### 3. Create Annotated Release Tag

```powershell
# Format: v<Major>.<Minor>.<Patch>
git tag -a v1.0.0 -m "release: v1.0.0"
```

### 4. Push Release Tag via Workstation Proxy

```powershell
# Always route push through active bridge proxy:
git spush origin v1.0.0
```

### 5. Monitor Release Pipeline on GitHub

```powershell
# Track GitHub Actions release build live in terminal:
gh run list --workflow=release.yml --limit 1
gh run watch
```

## Output Contract

Report status strictly:
```text
[CICD] Repo: <repo> | Target: v<version> (<READY|BLOCKED>)
• Hygiene:     Zero leaks & clean SSR (<PASSED|BLOCKED: <violation>>)
• Pre-flight:  Tests <PASSED|FAILED>, Lint <PASSED|FAILED>, Git <CLEAN|DIRTY>
• Version:     <old_version> -> <new_version> (Semantic Alignment)
• CI Guard:    Smart filtering active (zero duplicate runner waste)
• Egress:      git spush via SOCKS5 bridge (:1080)
• Artifact:    <target_binary_or_package>
Verdict: Pre-flight passed. Ready to ship. / Blocked: fix failures locally before tagging.
```

## Boundaries

Scope: CI/CD workflow lifecycle, release orchestration, local pre-flight gating, semantic tag management, and CI budget optimization.
- Do NOT push release tags if local tests or lints fail.
- Do NOT commit unverified code or dirty working trees into release tags.
- Do NOT bypass `git spush` on restricted workstation networks.
- One-shot execution.
