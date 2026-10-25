use crate::config::canonical_ssot_dir;
use crate::ui::prompt::clack_step_ok;
use std::path::PathBuf;

/// Result of Core provisioning
pub struct CoreResult {
    pub ssot_root: PathBuf,
    pub db_path: PathBuf,
}

/// Provision SSOT directories (~/.tq/) and initialize SQLite DB
pub fn run(verbose: bool) -> Result<CoreResult, Box<dyn std::error::Error>> {
    let root = canonical_ssot_dir();

    let dirs = [
        root.join("bin"),
        root.join(crate::constants::PILLAR_SYSTEM),
        root.join(crate::constants::PILLAR_AUTOMATION),
        root.join(crate::constants::PILLAR_BROWSER)
            .join(crate::constants::DIR_PROFILES),
        root.join(crate::constants::PILLAR_BROWSER)
            .join(crate::constants::DIR_RUNTIMES),
        root.join(crate::constants::PILLAR_BROWSER)
            .join(crate::constants::DIR_EXTENSIONS),
        root.join(crate::constants::PILLAR_GATEWAY).join("bin"),
        root.join(crate::constants::PILLAR_GATEWAY)
            .join(crate::constants::DIR_PIDS),
        root.join(crate::constants::PILLAR_FAKER),
    ];

    for d in &dirs {
        std::fs::create_dir_all(d)?;
    }

    let db_path = root
        .join(crate::constants::PILLAR_AUTOMATION)
        .join(crate::constants::FILE_AUTOMATION_SQLITE);

    let _ = crate::infrastructure::db::AutomationDb::new(&db_path)?;

    if verbose {
        clack_step_ok(format!(
            "SSOT root and database initialized at {}",
            root.display()
        ));
    }

    Ok(CoreResult {
        ssot_root: root,
        db_path,
    })
}
