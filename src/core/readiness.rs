use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

const READINESS_TTL_SECS: u64 = 300; // 5 minutes cache

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReadinessState {
    Pristine,
    Degraded,
    Fresh,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessIssue {
    pub component: String,
    pub title: String,
    pub remedy: String,
    pub is_critical: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkstationReadiness {
    pub state: ReadinessState,
    pub issues: Vec<ReadinessIssue>,
    pub ssot_root: PathBuf,
    pub timestamp: u64,
}

impl WorkstationReadiness {
    pub fn is_pristine(&self) -> bool {
        self.state == ReadinessState::Pristine
    }

    pub fn is_fresh(&self) -> bool {
        self.state == ReadinessState::Fresh
    }

    pub fn is_degraded(&self) -> bool {
        self.state == ReadinessState::Degraded
    }
}

fn cache_file_path() -> PathBuf {
    crate::config::canonical_dir()
        .join(crate::constants::PILLAR_SYSTEM)
        .join(".readiness_cache.json")
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Invalidate cache when state-changing operations run
pub fn invalidate_cache() {
    let path = cache_file_path();
    let _ = fs::remove_file(path);
}

/// Fast passive probe without heavy operations (<5ms)
pub fn quick_probe() -> WorkstationReadiness {
    let now = current_timestamp();
    let cache_path = cache_file_path();

    // 1. Try reading valid cache first
    if let Ok(content) = fs::read_to_string(&cache_path) {
        if let Ok(cached) = serde_json::from_str::<WorkstationReadiness>(&content) {
            if now.saturating_sub(cached.timestamp) < READINESS_TTL_SECS {
                return cached;
            }
        }
    }

    // 2. Perform lightweight passive check
    let ssot_root = crate::config::canonical_dir();
    let system_pillar = ssot_root.join(crate::constants::PILLAR_SYSTEM);

    // Case 1: Fresh Machine (SSOT root or system pillar does not exist)
    if !ssot_root.exists() || !system_pillar.exists() {
        let readiness = WorkstationReadiness {
            state: ReadinessState::Fresh,
            issues: vec![ReadinessIssue {
                component: "SSOT Core".into(),
                title: "Workstation has not been onboarded yet".into(),
                remedy: "Run 'tq setup' to initialize workstation".into(),
                is_critical: true,
            }],
            ssot_root,
            timestamp: now,
        };
        save_cache(&readiness);
        return readiness;
    }

    // Case 2: Check Critical Runtimes & Dependencies
    let mut issues = Vec::new();

    // Check Browser Engine
    let browser_status = crate::core::browser::resolver::get_runtime_status();
    if !browser_status.installed {
        issues.push(ReadinessIssue {
            component: "Browser Engine".into(),
            title: "Antidetect Chromium Golden LTS is not installed".into(),
            remedy: "Run 'tq browser install 148'".into(),
            is_critical: false,
        });
    }

    // Check OpenSSH
    let ssh_found = crate::infrastructure::bridge::tools::find_executable("ssh").is_some();
    if !ssh_found {
        issues.push(ReadinessIssue {
            component: "OpenSSH".into(),
            title: "SSH client binary not found in PATH".into(),
            remedy: "Install OpenSSH or run 'tq doctor --fix'".into(),
            is_critical: true,
        });
    }

    let state = if issues.is_empty() {
        ReadinessState::Pristine
    } else {
        ReadinessState::Degraded
    };

    let readiness = WorkstationReadiness {
        state,
        issues,
        ssot_root,
        timestamp: now,
    };

    save_cache(&readiness);
    readiness
}

fn save_cache(readiness: &WorkstationReadiness) {
    let cache_path = cache_file_path();
    if let Some(parent) = cache_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(serialized) = serde_json::to_string(readiness) {
        let _ = fs::write(cache_path, serialized);
    }
}
