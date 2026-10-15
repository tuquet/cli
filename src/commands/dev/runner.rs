use std::process::{Command, Stdio};
use crate::ui::{clack_outro, colors};
use super::scanner::DiscoveredPackage;

/// Execute a script from a discovered `package.json` using the detected (or overridden) package manager
pub fn run_package_script(
    pkg: &DiscoveredPackage,
    script: &str,
    pm_override: Option<&str>,
    extra_args: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let pm = pm_override.unwrap_or(&pkg.package_manager);
    let display_name = pkg.name.as_deref().unwrap_or(&pkg.id);

    clack_outro(&format!(
        "Launching {BOLD}{CYAN}{pm} run {script}{RESET} in {BOLD_WHITE}{display_name}{RESET} {MUTED}({dir}){RESET}",
        BOLD = colors::BOLD,
        CYAN = colors::CYAN,
        BOLD_WHITE = colors::BOLD_WHITE,
        MUTED = colors::MUTED,
        RESET = colors::RESET,
        dir = pkg.dir.display(),
    ));

    #[cfg(target_os = "windows")]
    let mut child_cmd = {
        let mut c = Command::new("cmd");
        c.arg("/C").arg(pm).arg("run").arg(script);
        if !extra_args.is_empty() {
            c.arg("--").args(extra_args);
        }
        c
    };

    #[cfg(not(target_os = "windows"))]
    let mut child_cmd = {
        let mut c = Command::new(pm);
        c.arg("run").arg(script);
        if !extra_args.is_empty() {
            c.arg("--").args(extra_args);
        }
        c
    };

    let status = child_cmd
        .current_dir(&pkg.dir)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| format!("Failed to spawn '{} run {}' in {}: {}", pm, script, pkg.dir.display(), e))?;

    if !status.success() {
        let code = status.code().unwrap_or(1);
        // Ignore standard Ctrl+C exit code on Windows (-1073741510 / 0xc000013a or 130)
        if code != 130 && code != -1073741510 {
            return Err(format!("Script '{} run {}' exited with status code {}", pm, script, code).into());
        }
    }

    Ok(())
}
