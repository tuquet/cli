use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use crate::ui::{clack_outro, colors};
use super::history::record_history;
use super::scanner::DiscoveredPackage;

/// Execute a script/target from a discovered project using the detected (or overridden) toolchain
pub fn run_package_script(
    pkg: &DiscoveredPackage,
    script: &str,
    pm_override: Option<&str>,
    extra_args: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    record_history(pkg, script);

    let pm = pm_override.unwrap_or(&pkg.package_manager);
    let display_name = pkg.name.as_deref().unwrap_or(&pkg.id);

    // Resolve raw command description from pkg.scripts if available
    let raw_cmd = pkg
        .scripts
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(script))
        .map(|(_, c)| c.as_str())
        .unwrap_or(script);

    // Check if Java process-scoped JDK switching via Scoop applies
    let scoop_jdk = detect_required_jdk_from_framework(pkg.framework.as_deref())
        .and_then(|ver| resolve_scoop_jdk_home(&ver));

    let jdk_note = match scoop_jdk {
        Some((ref ver, ref home)) => format!(
            " {MUTED}[JDK {ver} → {home}]{RESET}",
            MUTED = colors::MUTED,
            RESET = colors::RESET,
            ver = ver,
            home = home.display()
        ),
        None => String::new(),
    };

    clack_outro(&format!(
        "Launching {BOLD}{CYAN}{pm} ({script}){RESET} in {BOLD_WHITE}{display_name}{RESET} {MUTED}({dir}){RESET}{jdk_note}",
        BOLD = colors::BOLD,
        CYAN = colors::CYAN,
        BOLD_WHITE = colors::BOLD_WHITE,
        MUTED = colors::MUTED,
        RESET = colors::RESET,
        dir = pkg.dir.display(),
    ));

    let mut child_cmd = build_toolchain_command(pm, script, raw_cmd, extra_args);

    if let Some((_, ref java_home)) = scoop_jdk {
        let java_bin = java_home.join("bin");
        let current_path = std::env::var_os("PATH").unwrap_or_default();
        let mut paths = vec![java_bin];
        paths.extend(std::env::split_paths(&current_path));
        if let Ok(joined) = std::env::join_paths(paths) {
            child_cmd.env("PATH", joined);
        }
        child_cmd.env("JAVA_HOME", java_home);
    }

    let status = child_cmd
        .current_dir(&pkg.dir)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| {
            format!(
                "Failed to spawn '{} ({})' in {}: {}",
                pm,
                script,
                pkg.dir.display(),
                e
            )
        })?;

    if !status.success() {
        let code = status.code().unwrap_or(1);
        // Ignore standard Ctrl+C exit code on Windows (-1073741510 / 0xc000013a or 130)
        if code != 130 && code != -1073741510 {
            return Err(
                format!("Command '{} ({})' exited with status code {}", pm, script, code).into(),
            );
        }
    }

    Ok(())
}

fn build_toolchain_command(
    pm: &str,
    script: &str,
    raw_cmd: &str,
    extra_args: &[String],
) -> Command {
    match pm {
        "cargo" => {
            let mut c = spawn_shell_cmd("cargo");
            let sub_args: Vec<&str> = raw_cmd
                .strip_prefix("cargo ")
                .unwrap_or(script)
                .split_whitespace()
                .collect();
            c.args(&sub_args);
            if !extra_args.is_empty() {
                c.arg("--").args(extra_args);
            }
            c
        }
        "mvn" | "mvnw" => {
            let exe = if cfg!(windows) && pm == "mvnw" {
                ".\\mvnw.cmd"
            } else if pm == "mvnw" {
                "./mvnw"
            } else {
                "mvn"
            };
            let mut c = spawn_shell_cmd(exe);
            let sub_args: Vec<&str> = raw_cmd.split_whitespace().collect();
            c.args(&sub_args);
            c.args(extra_args);
            c
        }
        "gradle" | "gradlew" => {
            let exe = if cfg!(windows) && pm == "gradlew" {
                ".\\gradlew.bat"
            } else if pm == "gradlew" {
                "./gradlew"
            } else {
                "gradle"
            };
            let mut c = spawn_shell_cmd(exe);
            let sub_args: Vec<&str> = raw_cmd.split_whitespace().collect();
            c.args(&sub_args);
            c.args(extra_args);
            c
        }
        "docker" => {
            let mut c = spawn_shell_cmd("docker");
            let sub_args: Vec<&str> = raw_cmd
                .strip_prefix("docker ")
                .unwrap_or(raw_cmd)
                .split_whitespace()
                .collect();
            c.args(&sub_args);
            c.args(extra_args);
            c
        }
        _ => {
            let mut c = spawn_shell_cmd(pm);
            c.arg("run").arg(script);
            if !extra_args.is_empty() {
                c.arg("--").args(extra_args);
            }
            c
        }
    }
}

fn spawn_shell_cmd(program: &str) -> Command {
    #[cfg(target_os = "windows")]
    {
        let mut c = Command::new("cmd");
        c.arg("/C").arg(program);
        c
    }
    #[cfg(not(target_os = "windows"))]
    {
        Command::new(program)
    }
}

pub fn detect_required_jdk_from_framework(framework: Option<&str>) -> Option<String> {
    let fw = framework?;
    let idx = fw.find("JDK ")?;
    let rest = &fw[idx + 4..];
    let ver: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if ver.is_empty() { None } else { Some(ver) }
}

/// Locate a matching JDK installation inside `~/scoop/apps/` without modifying global environment variables
pub fn resolve_scoop_jdk_home(version: &str) -> Option<(String, PathBuf)> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok()?;
    let scoop_apps = Path::new(&home).join("scoop").join("apps");
    if !scoop_apps.is_dir() {
        return None;
    }

    let entries = std::fs::read_dir(&scoop_apps).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if (name.contains("jdk") || name.contains("temurin") || name.contains("zulu") || name.contains("graalvm"))
            && name.contains(version)
        {
            let current_dir = entry.path().join("current");
            if current_dir.join("bin").is_dir() {
                return Some((version.to_string(), current_dir));
            }
        }
    }
    None
}
