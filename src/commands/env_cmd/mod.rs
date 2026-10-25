use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::commands::dev::scanner::{
    DiscoveredPackage, filter_packages, scan_machine_packages, scan_workspace_packages,
};
use crate::ui::{
    ClackOption, ClackSpinner, Column, Notify, OutputFormat, Table, TabularRow, badge_online,
    badge_warn, clack_cancel, clack_intro, clack_outro, clack_select, colors, create_tabular_card,
};

// =============================================================================
// HERMES-INSPIRED SECRET SCRUBBING & CHILD PROCESS SECURITY
// =============================================================================

/// Substrings that trigger aggressive secret scrubbing (Hermes Agent standard)
pub const SECRET_SUBSTRINGS: &[&str] = &[
    "KEY",
    "TOKEN",
    "SECRET",
    "PASSWORD",
    "CREDENTIAL",
    "PASSWD",
    "AUTH",
    "DSN",
    "WEBHOOK",
    "CREDS",
    "BEARER",
    "APIKEY",
    "COOKIE",
];

/// Safe system environment variable prefixes allowed in subshells and sandboxes
pub const SAFE_ENV_PREFIXES: &[&str] = &[
    "PATH",
    "HOME",
    "USER",
    "LANG",
    "LC_",
    "TERM",
    "TMPDIR",
    "TMP",
    "TEMP",
    "SHELL",
    "LOGNAME",
    "XDG_",
    "PYTHONPATH",
    "VIRTUAL_ENV",
    "CONDA",
];

/// Essential Windows operating system environment variables (prevents WinSock / CRT crash)
pub const WINDOWS_ESSENTIAL_ENV_VARS: &[&str] = &[
    "SYSTEMROOT",
    "SYSTEMDRIVE",
    "WINDIR",
    "COMSPEC",
    "PATHEXT",
    "OS",
    "PROCESSOR_ARCHITECTURE",
    "NUMBER_OF_PROCESSORS",
    "PUBLIC",
    "ALLUSERSPROFILE",
    "PROGRAMDATA",
    "PROGRAMFILES",
    "PROGRAMFILES(X86)",
    "APPDATA",
    "LOCALAPPDATA",
    "USERPROFILE",
    "HOMEDRIVE",
    "HOMEPATH",
    "COMPUTERNAME",
];

/// Non-secret Tuquet runtime environment variables that can safely ride along
pub const TQ_ALLOWED_CONTEXT_VARS: &[&str] =
    &["TQ_HOME", "TQ_TENANT_ID", "TQ_SESSION_ID", "TQ_PROFILE"];

/// Check if an environment variable name is considered secret
pub fn is_secret_env_name(name: &str) -> bool {
    let upper = name.to_uppercase();
    SECRET_SUBSTRINGS.iter().any(|s| upper.contains(s))
}

/// Scrub an environment map for child process execution (sandboxing & leak-prevention)
pub fn scrub_child_env<I, K, V>(source_env: I) -> std::collections::HashMap<String, String>
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<str>,
    V: AsRef<str>,
{
    let mut clean_env = std::collections::HashMap::new();

    for (k, v) in source_env {
        let key = k.as_ref();
        let val = v.as_ref();
        let upper_key = key.to_uppercase();

        // 1. Secret-substring block: drop immediately
        if is_secret_env_name(key) {
            continue;
        }

        // 2. Windows essentials: pass through
        if cfg!(windows) && WINDOWS_ESSENTIAL_ENV_VARS.contains(&upper_key.as_str()) {
            clean_env.insert(key.to_string(), val.to_string());
            continue;
        }

        // 3. Allowed Tuquet context vars: pass through
        if TQ_ALLOWED_CONTEXT_VARS.contains(&upper_key.as_str()) {
            clean_env.insert(key.to_string(), val.to_string());
            continue;
        }

        // 4. Safe prefix match: pass through
        if SAFE_ENV_PREFIXES.iter().any(|p| upper_key.starts_with(p)) {
            clean_env.insert(key.to_string(), val.to_string());
        }
    }

    clean_env
}

const SECRET_KEYWORDS: &[&str] = &[
    "secret",
    "password",
    "passwd",
    "token",
    "key",
    "private",
    "credential",
    "auth",
    "jwt",
    "dsn",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvProjectReport {
    pub project_id: String,
    pub project_dir: PathBuf,
    pub active_file: String,
    pub template_file: Option<String>,
    pub total_keys: usize,
    pub missing_keys: Vec<(String, String)>,
    pub empty_keys: Vec<String>,
    pub extra_keys: Vec<String>,
    pub profiles: Vec<String>,
    pub masked_entries: Vec<(String, String)>,
}

pub async fn handle(
    project: Option<String>,
    check: bool,
    switch_profile: Option<String>,
    list: bool,
    path: Option<PathBuf>,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut intro_shown = false;

    // Resolve target project directory
    let (target_id, target_dir) = if project.is_none() && path.is_none() && has_any_env_file(&cwd) {
        let name = cwd
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(".")
            .to_string();
        (name, cwd)
    } else {
        let interactive_scan = !list && !check && switch_profile.is_none();
        if interactive_scan {
            clack_intro("tq env", "Environment Inspector & Profile Switcher");
            intro_shown = true;
        }
        let sp = if interactive_scan {
            Some(ClackSpinner::start(
                "Scanning workspace for .env configurations...",
            ))
        } else {
            None
        };

        let packages = match path {
            Some(ref custom_root) => scan_workspace_packages(custom_root),
            None => scan_machine_packages(&cwd),
        };
        let env_packages: Vec<&DiscoveredPackage> = packages
            .iter()
            .filter(|p| has_any_env_file(&p.dir))
            .collect();

        let pool: Vec<&DiscoveredPackage> = if env_packages.is_empty() {
            packages.iter().collect()
        } else {
            env_packages
        };

        if let Some(spinner) = sp {
            spinner.finish(format!(
                "Discovered {} project(s) with .env* files",
                pool.len()
            ));
        }

        if pool.is_empty() {
            Notify::warn("No projects with .env files found.");
            return Ok(());
        }

        // Non-interactive multi-project overview when `tq env -l` or `tq env --check` is run without a project arg
        if project.is_none() && (list || check) {
            let reports: Vec<EnvProjectReport> = pool
                .iter()
                .map(|p| inspect_project_env(&p.id, &p.dir))
                .collect();
            render_env_multi_list(&reports, format);
            return Ok(());
        }

        match project {
            Some(ref q) => {
                let matched = filter_packages(&packages, q);
                if matched.is_empty() {
                    return Err(format!("No project matching '{}' found.", q).into());
                } else if matched.len() == 1 {
                    (matched[0].id.clone(), matched[0].dir.clone())
                } else {
                    let opts: Vec<ClackOption> = matched
                        .iter()
                        .map(|p| {
                            ClackOption::new(
                                &p.id,
                                p.dir.display().to_string(),
                                format!("{} ({})", p.id, p.dir.display()),
                            )
                        })
                        .collect();
                    match clack_select("Select target project for .env management", &opts, 0, 10)? {
                        Some(idx) => (matched[idx].id.clone(), matched[idx].dir.clone()),
                        None => {
                            clack_cancel("Operation cancelled.");
                            return Ok(());
                        }
                    }
                }
            }
            None => {
                let opts: Vec<ClackOption> = pool
                    .iter()
                    .map(|p| {
                        let env_files = list_env_filenames(&p.dir);
                        ClackOption::new(
                            &p.id,
                            format!("{} — {}", env_files.join(", "), p.dir.display()),
                            format!("{} ({})", p.id, env_files.join(", ")),
                        )
                    })
                    .collect();
                match clack_select("Select a project to inspect .env files", &opts, 0, 10)? {
                    Some(idx) => (pool[idx].id.clone(), pool[idx].dir.clone()),
                    None => {
                        clack_cancel("Operation cancelled.");
                        return Ok(());
                    }
                }
            }
        }
    };

    let report = inspect_project_env(&target_id, &target_dir);

    if let Some(ref prof) = switch_profile {
        apply_profile_switch(&target_dir, &target_id, prof)?;
        return Ok(());
    }

    if list || check {
        render_env_report(&report, format);
        return Ok(());
    }

    if !intro_shown {
        clack_intro(
            "tq env",
            &format!(
                "{} • {} keys, {} missing vs template, {} empty",
                report.project_id,
                report.total_keys,
                report.missing_keys.len(),
                report.empty_keys.len()
            ),
        );
    }

    let mut actions: Vec<(String, ClackOption)> = Vec::new();

    if !report.missing_keys.is_empty() {
        let missing_names: Vec<&str> = report
            .missing_keys
            .iter()
            .take(4)
            .map(|(k, _)| k.as_str())
            .collect();
        actions.push((
            "sync".to_string(),
            ClackOption::new(
                format!(
                    "Sync {} missing key(s) from {} → .env",
                    report.missing_keys.len(),
                    report.template_file.as_deref().unwrap_or(".env.example")
                ),
                format!("Missing: {}", missing_names.join(", ")),
                "Sync missing keys to .env",
            ),
        ));
    }

    actions.push((
        "inspect".to_string(),
        ClackOption::new(
            "Inspect variables & diff report",
            format!(
                "{} active keys in {} (secrets masked)",
                report.total_keys, report.active_file
            ),
            "Inspect variables & diff report",
        ),
    ));

    for prof in &report.profiles {
        actions.push((
            format!("switch:{}", prof),
            ClackOption::new(
                format!("Switch profile → .env.{}", prof),
                format!(
                    "Copies .env.{} to .env (backs up current .env to .env.bak)",
                    prof
                ),
                format!("Switch profile → .env.{}", prof),
            ),
        ));
    }

    let clack_opts: Vec<ClackOption> = actions.iter().map(|(_, o)| o.clone()).collect();
    match clack_select(
        &format!("Select .env action for {}", report.project_id),
        &clack_opts,
        0,
        8,
    )? {
        Some(idx) => {
            let (act_id, _) = &actions[idx];
            if act_id == "sync" {
                let added = sync_missing_keys_to_dotenv(&target_dir, &report.missing_keys)?;
                clack_outro(&format!(
                    "Appended {BOLD}{CYAN}{added} missing key(s){RESET} to {BOLD_WHITE}{dir}/.env{RESET}",
                    BOLD = colors::BOLD,
                    CYAN = colors::CYAN,
                    BOLD_WHITE = colors::BOLD_WHITE,
                    RESET = colors::RESET,
                    dir = target_dir.display(),
                ));
            } else if let Some(prof) = act_id.strip_prefix("switch:") {
                apply_profile_switch(&target_dir, &target_id, prof)?;
            } else {
                clack_outro(&format!(
                    "Environment report for {BOLD}{CYAN}{id}{RESET}",
                    BOLD = colors::BOLD,
                    CYAN = colors::CYAN,
                    RESET = colors::RESET,
                    id = report.project_id,
                ));
                render_env_report(&report, format);
            }
        }
        None => {
            clack_cancel("Operation cancelled.");
        }
    }

    Ok(())
}

fn has_any_env_file(dir: &Path) -> bool {
    !list_env_filenames(dir).is_empty()
}

fn list_env_filenames(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names = Vec::new();
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        if (name == ".env" || name.starts_with(".env.")) && !name.ends_with(".bak") {
            names.push(name.into_owned());
        }
    }
    names.sort();
    names
}

pub fn inspect_project_env(project_id: &str, dir: &Path) -> EnvProjectReport {
    let active_file = if dir.join(".env").is_file() {
        ".env".to_string()
    } else if dir.join(".env.local").is_file() {
        ".env.local".to_string()
    } else {
        ".env".to_string()
    };

    let active_content = std::fs::read_to_string(dir.join(&active_file)).unwrap_or_default();
    let active_pairs = parse_dotenv_content(&active_content);
    let active_map: BTreeMap<String, String> = active_pairs.iter().cloned().collect();

    let template_candidates = [".env.example", ".env.template", ".env.sample"];
    let template_file = template_candidates
        .iter()
        .find(|f| dir.join(*f).is_file())
        .map(|s| (*s).to_string());

    let template_pairs = match template_file {
        Some(ref tf) => {
            let content = std::fs::read_to_string(dir.join(tf)).unwrap_or_default();
            parse_dotenv_content(&content)
        }
        None => Vec::new(),
    };
    let template_map: BTreeMap<String, String> = template_pairs.iter().cloned().collect();

    let mut missing_keys = Vec::new();
    for (k, v) in &template_pairs {
        if !active_map.contains_key(k) {
            missing_keys.push((k.clone(), v.clone()));
        }
    }

    let mut empty_keys = Vec::new();
    let mut extra_keys = Vec::new();
    let mut masked_entries = Vec::new();

    for (k, v) in &active_pairs {
        if v.trim().is_empty() {
            empty_keys.push(k.clone());
        }
        if !template_map.is_empty() && !template_map.contains_key(k) {
            extra_keys.push(k.clone());
        }
        masked_entries.push((k.clone(), mask_env_value(k, v)));
    }

    let mut profiles = Vec::new();
    for name in list_env_filenames(dir) {
        if let Some(prof) = name.strip_prefix(".env.")
            && !matches!(prof, "example" | "template" | "sample" | "bak")
        {
            profiles.push(prof.to_string());
        }
    }

    EnvProjectReport {
        project_id: project_id.to_string(),
        project_dir: dir.to_path_buf(),
        active_file,
        template_file,
        total_keys: active_pairs.len(),
        missing_keys,
        empty_keys,
        extra_keys,
        profiles,
        masked_entries,
    }
}

pub fn parse_dotenv_content(content: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line).trim();
        let Some(eq_idx) = line.find('=') else {
            continue;
        };
        let key = line[..eq_idx].trim();
        if key.is_empty() {
            continue;
        }
        let mut val = line[eq_idx + 1..].trim();
        if (val.starts_with('"') && val.ends_with('"') && val.len() >= 2)
            || (val.starts_with('\'') && val.ends_with('\'') && val.len() >= 2)
        {
            val = &val[1..val.len() - 1];
        } else if let Some(comment_idx) = val.find(" #") {
            val = val[..comment_idx].trim();
        }
        pairs.push((key.to_string(), val.to_string()));
    }
    pairs
}

pub fn mask_env_value(key: &str, value: &str) -> String {
    if value.is_empty() {
        return "(empty)".to_string();
    }
    let key_lower = key.to_lowercase();
    let is_secret = SECRET_KEYWORDS.iter().any(|kw| key_lower.contains(kw));
    if is_secret {
        if value.len() <= 4 {
            "••••••••".to_string()
        } else {
            format!("{}••••{}", &value[..2], &value[value.len() - 2..])
        }
    } else {
        value.to_string()
    }
}

pub fn sync_missing_keys_to_dotenv(
    dir: &Path,
    missing_keys: &[(String, String)],
) -> Result<usize, Box<dyn std::error::Error>> {
    if missing_keys.is_empty() {
        return Ok(0);
    }
    let dotenv_path = dir.join(".env");
    let mut content = std::fs::read_to_string(&dotenv_path).unwrap_or_default();
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str("\n# Synced from .env.example by tq env\n");
    for (k, v) in missing_keys {
        content.push_str(&format!("{}={}\n", k, v));
    }
    std::fs::write(&dotenv_path, content)?;
    Ok(missing_keys.len())
}

fn apply_profile_switch(
    dir: &Path,
    project_id: &str,
    profile: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let clean_prof = profile.trim().trim_start_matches(".env.");
    let source_path = dir.join(format!(".env.{}", clean_prof));
    if !source_path.is_file() {
        return Err(format!(
            "Profile file '{}' does not exist in {}",
            source_path.display(),
            dir.display()
        )
        .into());
    }

    let target_path = dir.join(".env");
    if target_path.is_file() {
        let backup_path = dir.join(".env.bak");
        let _ = std::fs::copy(&target_path, &backup_path);
    }

    std::fs::copy(&source_path, &target_path)?;
    clack_outro(&format!(
        "Switched {BOLD}{CYAN}{id}{RESET} active {BOLD_WHITE}.env{RESET} ← {CYAN}.env.{prof}{RESET} {MUTED}(backup saved to .env.bak){RESET}",
        BOLD = colors::BOLD,
        CYAN = colors::CYAN,
        BOLD_WHITE = colors::BOLD_WHITE,
        MUTED = colors::MUTED,
        RESET = colors::RESET,
        id = project_id,
        prof = clean_prof,
    ));
    Ok(())
}

fn render_env_report(report: &EnvProjectReport, format: OutputFormat) {
    match format {
        OutputFormat::Json => {
            if let Ok(json) = serde_json::to_string_pretty(report) {
                println!("{}", json);
            }
        }
        OutputFormat::Table => {
            let mut table = Table::new(vec![
                Column {
                    title: "VARIABLE KEY".to_string(),
                    min_width: 26,
                    align_right: false,
                },
                Column {
                    title: "VALUE (MASKED)".to_string(),
                    min_width: 28,
                    align_right: false,
                },
                Column {
                    title: "STATUS".to_string(),
                    min_width: 14,
                    align_right: false,
                },
            ])
            .with_footer(format!(
                "Project: {} ({}) • {} keys, {} missing, {} empty • Profiles: {}",
                report.project_id,
                report.active_file,
                report.total_keys,
                report.missing_keys.len(),
                report.empty_keys.len(),
                if report.profiles.is_empty() {
                    "none".to_string()
                } else {
                    report.profiles.join(", ")
                }
            ));

            for (k, v) in &report.masked_entries {
                let status = if report.empty_keys.contains(k) {
                    "EMPTY"
                } else {
                    "OK"
                };
                table.add_row(vec![k.clone(), v.clone(), status.to_string()]);
            }
            for (mk, mv) in &report.missing_keys {
                table.add_row(vec![
                    mk.clone(),
                    format!("(default: {})", mask_env_value(mk, mv)),
                    "MISSING".to_string(),
                ]);
            }
            println!();
            table.print();
            println!();
        }
        OutputFormat::Card => {
            let mut rows: Vec<TabularRow> = report
                .masked_entries
                .iter()
                .map(|(k, v)| {
                    let badge = if report.empty_keys.contains(k) {
                        badge_warn("EMPTY")
                    } else {
                        badge_online("SET")
                    };
                    TabularRow::new(k, v, &report.active_file, badge)
                })
                .collect();

            for (mk, mv) in &report.missing_keys {
                rows.push(TabularRow::new(
                    mk,
                    mask_env_value(mk, mv),
                    report.template_file.as_deref().unwrap_or(".env.example"),
                    badge_warn("MISSING"),
                ));
            }

            let footer = format!(
                "Active: {} • Missing: {} • Empty: {} • Profiles: {}",
                report.active_file,
                report.missing_keys.len(),
                report.empty_keys.len(),
                if report.profiles.is_empty() {
                    "none".to_string()
                } else {
                    report.profiles.join(", ")
                }
            );
            let card = create_tabular_card(
                &format!("ENVIRONMENT VARIABLES ({})", report.project_id),
                Some(if report.missing_keys.is_empty() {
                    badge_online("SYNCED")
                } else {
                    badge_warn(&format!("{} MISSING", report.missing_keys.len()))
                }),
                ["KEY", "VALUE (MASKED)", "SOURCE", "STATUS"],
                &rows,
                Some(&footer),
                76,
            );
            println!();
            card.print();
            println!();
        }
    }
}

fn render_env_multi_list(reports: &[EnvProjectReport], format: OutputFormat) {
    match format {
        OutputFormat::Json => {
            if let Ok(json) = serde_json::to_string_pretty(reports) {
                println!("{}", json);
            }
        }
        OutputFormat::Table | OutputFormat::Card => {
            let mut table = Table::new(vec![
                Column {
                    title: "PROJECT".to_string(),
                    min_width: 22,
                    align_right: false,
                },
                Column {
                    title: "ACTIVE FILE".to_string(),
                    min_width: 14,
                    align_right: false,
                },
                Column {
                    title: "KEYS".to_string(),
                    min_width: 8,
                    align_right: true,
                },
                Column {
                    title: "MISSING".to_string(),
                    min_width: 9,
                    align_right: true,
                },
                Column {
                    title: "PROFILES".to_string(),
                    min_width: 18,
                    align_right: false,
                },
            ])
            .with_footer(format!(
                "Total: {} project(s) with .env* files",
                reports.len()
            ));

            for r in reports {
                table.add_row(vec![
                    r.project_id.clone(),
                    r.active_file.clone(),
                    r.total_keys.to_string(),
                    r.missing_keys.len().to_string(),
                    if r.profiles.is_empty() {
                        "-".to_string()
                    } else {
                        r.profiles.join(", ")
                    },
                ]);
            }
            println!();
            table.print();
            println!();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hermes_secret_scrubbing() {
        let env_vars = vec![
            ("PATH", "C:\\Windows\\system32"),
            ("SUPABASE_SERVICE_ROLE_KEY", "super_secret_role_key"),
            ("DATABASE_PASSWORD", "db_pwd_123"),
            ("TELEGRAM_BOT_TOKEN", "bot_12345"),
            ("TEMP", "C:\\Temp"),
            ("TQ_TENANT_ID", "00000000-0000-0000-0000-000000000001"),
            ("USER_DEFINED_SECRET", "should_be_dropped"),
            ("NODE_AUTH_TOKEN", "should_be_dropped"),
        ];

        let clean = scrub_child_env(env_vars);

        // Secrets must be dropped
        assert!(!clean.contains_key("SUPABASE_SERVICE_ROLE_KEY"));
        assert!(!clean.contains_key("DATABASE_PASSWORD"));
        assert!(!clean.contains_key("TELEGRAM_BOT_TOKEN"));
        assert!(!clean.contains_key("USER_DEFINED_SECRET"));
        assert!(!clean.contains_key("NODE_AUTH_TOKEN"));

        // Safe variables must remain
        assert!(clean.contains_key("PATH"));
        assert!(clean.contains_key("TEMP"));
        assert!(clean.contains_key("TQ_TENANT_ID"));
    }

    #[test]
    fn test_parse_dotenv_and_sync_missing() {
        let temp_dir = std::env::temp_dir().join(format!("spc_env_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        std::fs::write(
            temp_dir.join(".env.example"),
            "PORT=3000\nAPI_SECRET_KEY=supersecret123\nDATABASE_URL=postgres://localhost/db\n",
        )
        .unwrap();
        std::fs::write(temp_dir.join(".env"), "PORT=5173\nAPI_SECRET_KEY=\n").unwrap();
        std::fs::write(temp_dir.join(".env.staging"), "PORT=8080\n").unwrap();

        let report = inspect_project_env("test-app", &temp_dir);
        assert_eq!(report.total_keys, 2);
        assert_eq!(report.missing_keys.len(), 1);
        assert_eq!(report.missing_keys[0].0, "DATABASE_URL");
        assert_eq!(report.empty_keys, vec!["API_SECRET_KEY".to_string()]);
        assert_eq!(report.profiles, vec!["staging".to_string()]);

        let synced = sync_missing_keys_to_dotenv(&temp_dir, &report.missing_keys).unwrap();
        assert_eq!(synced, 1);

        let report2 = inspect_project_env("test-app", &temp_dir);
        assert!(report2.missing_keys.is_empty());
        assert_eq!(report2.total_keys, 3);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
