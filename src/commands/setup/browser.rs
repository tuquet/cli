use crate::ui::prompt::{clack_step_ok, clack_step_warn};

pub async fn run(force: bool) -> Result<(), Box<dyn std::error::Error>> {
    let status = crate::core::browser::resolver::get_runtime_status();
    if status.installed && !force {
        clack_step_ok(format!(
            "Antidetect Chromium already provisioned: v{}",
            crate::core::browser::resolver::get_active_version()
        ));
        return Ok(());
    }

    println!("  Downloading & provisioning Antidetect Chromium (Golden LTS v148)...");
    match crate::core::browser::resolver::download_stealth_runtime(force, Some("148")).await {
        Ok(path) => {
            clack_step_ok(format!("Installed Antidetect Chromium at {}", path));
        }
        Err(e) => {
            clack_step_warn(format!(
                "Browser download deferred: {}. You can install later via 'tq browser install'",
                e
            ));
        }
    }

    Ok(())
}
