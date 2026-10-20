# Spec: Runner Process Name Validation & Cleanup (KISS & YAGNI)

## Objective
Simplify and consolidate runner daemon process identification and termination guards in `cli` by eliminating obsolete and redundant names (`spc`, `specter`, `runner`, `tuquet`) in accordance with **KISS** (Keep It Simple, Stupid) and **YAGNI** (You Aren't Gonna Need It). Align the process name guard strictly with the single canonical binary name **`tq`** (defined as `[[bin]] name = "tq"` in `cli/Cargo.toml`) and unify the orphaned process scanner in `stop_daemon` with `is_valid_runner_name`.

### Context & Problem Statement
1. **Dead Artifacts & Non-Binaries (YAGNI):** `is_valid_runner_name` previously checked for `"spc"` and `"specter"` (historical project names), `"runner"` (a CLI subcommand, never an executable binary), and `"tuquet"`. The project's compiled binary target is solely `tq` (`tq.exe` on Windows).
2. **Inconsistent Scanner Logic (KISS):** In `cli/src/commands/runner/supervisor.rs`, `stop_daemon` performed a fallback scan using `name.contains("tuquet")`. Because the binary executes as `tq.exe`, `"tq.exe".contains("tuquet")` evaluated to `false`, causing orphaned background daemons to be completely missed.
3. **Guard Purpose:** The only goal of `is_valid_runner_name` is to prevent accidentally killing innocent processes if an OS recycles a stale PID stored in `runner.pid`.

---

## Tech Stack
- **Language:** Rust (edition 2024)
- **Target Crate:** `tuquet` (`cli/`)
- **Key Dependencies:** `sysinfo`, `tokio`, `tracing`

---

## Commands
Full executable commands for development and verification:

```bash
# Run unit test for runner validation
cargo test --manifest-path cli/Cargo.toml --lib commands::runner::tests::test_is_valid_runner_name

# Run all runner-related unit tests
cargo test --manifest-path cli/Cargo.toml --lib commands::runner

# Check compilation & lints
cargo check --manifest-path cli/Cargo.toml
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
```

---

## Project Structure
Relevant source files and test locations:

```text
cli/
├── Cargo.toml                              # Binary definition: [[bin]] name = "tq"
└── src/
    └── commands/
        └── runner/
            ├── mod.rs                      # Re-exports supervisor functions & unit tests
            └── supervisor.rs               # Definition of is_valid_runner_name & stop_daemon
```

---

## Code Style & Implementation Design

### Implementation (KISS / YAGNI)
Strip whitespace and optional `.exe` extension (case-insensitive), matching strictly against canonical binary name `tq`:

```rust
/// Validates whether a process executable name matches the TuQuet runner binary (`tq`).
/// Protects against terminating innocent processes during stale PID cleanup.
pub fn is_valid_runner_name(raw_name: &str) -> bool {
    let name = raw_name.trim().to_lowercase();
    let base = name.strip_suffix(".exe").unwrap_or(&name);
    base == "tq"
}
```

### Unification in `stop_daemon`
Replace ad-hoc `name.contains("tuquet")` in the orphan scanner loop with `is_valid_runner_name(&name)`, and update warning log to specify `tq`:

```rust
for (p, proc) in sys.processes() {
    let p_u32 = p.as_u32();
    if p_u32 == current_pid {
        continue;
    }
    let name = proc.name().to_string_lossy();
    if is_valid_runner_name(&name) {
        let cmd = proc
            .cmd()
            .iter()
            .map(|s| s.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ");
        if cmd.contains("runner") && cmd.contains("start") && !found_pids.contains(&p_u32) {
            found_pids.push(p_u32);
        }
    }
}
```

And stale PID warning log:
```rust
tracing::warn!(
    "Stale PID {} detected: process name '{}' does not match tq. Skipping kill.",
    pid,
    raw_name
);
eprintln!(
    "Warning: Stale PID {} detected: process name '{}' does not match tq. Skipping kill.",
    pid, raw_name
);
```

---

## Testing Strategy
- **Framework:** Rust built-in test runner (`cargo test`)
- **Unit Tests (`cli/src/commands/runner/mod.rs`):**
  - **Valid cases:** `"tq"`, `"tq.exe"`, `"TQ.EXE"`, `"  tq.exe  "`
  - **Rejected legacy/non-binary/aliases:** `"tuquet"`, `"tuquet.exe"`, `"spc"`, `"spc.exe"`, `"specter"`, `"specter.exe"`, `"runner"`, `"runner.exe"`
  - **Rejected third-party applications:** `"chrome.exe"`, `"notepad.exe"`, `"svchost.exe"`, `"cmd.exe"`, `"powershell.exe"`

---

## Boundaries
- **Always:**
  - Preserve case-insensitivity on Windows (`.exe` suffix removal and name comparison).
  - Protect user machines from stale PID collisions by validating process identity before executing `taskkill` / `kill`.
  - Maintain consistent error/warning log messages citing `tq`.
- **Ask First:**
  - Adding any new binary aliases or changing `[[bin]]` configuration in `Cargo.toml`.
- **Never:**
  - Allow killing arbitrary processes if PID matches but process name fails validation.
  - Re-introduce deprecated brand names (`specter`, `spc`) or non-existent binary names (`runner`).

---

## Success Criteria
1. `is_valid_runner_name("tq")` and `is_valid_runner_name("tq.exe")` evaluate to `true` (case-insensitive).
2. Legacy names (`spc`, `specter`), subcommand alias (`runner`), and brand aliases (`tuquet`) evaluate to `false`.
3. Innocent system processes evaluate to `false`.
4. `stop_daemon` process scan correctly detects detached runner instances started under `tq` / `tq.exe`.
5. All unit tests in `cli/src/commands/runner` pass cleanly.
