---
name: specter-security
description: >
  Enterprise standard and autonomous runbook for cloud server hardening,
  Zero-Trust network offloading (Zero open inbound ports), cryptographic SSH enforcement,
  and proactive disk failure prevention. Trigger: /specter-security, "specter security",
  "vps security", "harden server", "zero open ports", "disk resilience".
argument-hint: "[audit|harden|check-ports]"
license: MIT
---

# Specter Security Standard (`specter-security`)

Paranoid cloud defense. Zero open inbound ports. Key-only auth. Disk lockup prevention. Eliminates automated port scanners and unexpected node outages.

## The 4 Invariants

1. **Zero Open Inbound Ports**: Public network interfaces accept NO direct incoming traffic. Inbound routes strictly via Cloudflare Tunnels (Zero Trust). Services bind to `127.0.0.1`.
2. **Key-Only Authentication**: Passwords disabled. Strictly enforce Ed25519 or ECDSA keys.
3. **Proactive Disk Resilience**: Automated log rotation and journal trimming. Alert before disk utilization crosses 80%.
4. **Outbound-Only Mesh**: Local access to services traverses encrypted reverse tunnels via `specter bridge`.

## Commands

Audit target node:
```bash
# 1. Check open listening ports on all public interfaces
ss -tulpn | grep -v '127.0.0.1'

# 2. Check SSH key-only configuration
grep -E '^(PasswordAuthentication|PermitRootLogin|PubkeyAuthentication)' /etc/ssh/sshd_config

# 3. Check disk headroom
df -h /
```

Harden node:
```bash
# Enforce UFW default deny inbound
ufw default deny incoming
ufw default allow outgoing
ufw enable

# Trim journal logs to max 100MB
journalctl --vacuum-size=100M
```

## Output Contract

Report audit strictly:
```text
[SECURITY AUDIT] Target: <server_ip_or_hostname>
• Inbound Ports: 0 open (IMMUNE) / <N> open (VIOLATION: <ports>)
• Auth: Ed25519 strictly enforced (<status>)
• Disk: <used>% (<free>GB free) - Safe
Verdict: Node fully hardened. / Action Required: run hardening commands.
```

## Boundaries

Do NOT alter `/etc/ssh/sshd_config` or change SSH keys without verifying current session connectivity. Audit mode inspects and reports only, applies nothing.
