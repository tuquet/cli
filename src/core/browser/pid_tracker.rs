use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use sysinfo::System;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSession {
    pub profile_id: String,
    pub profile_name: String,
    pub pid: u32,
    pub mode: String, // "extension" (zero-port stealth) or "driver" (cdp)
    pub port: u16,    // 0 for zero-port
    #[serde(default)]
    pub ws_url: Option<String>,
    #[serde(default)]
    pub headless: bool,
    #[serde(default)]
    pub proxy: Option<String>,
    pub started_at_secs: u64,
}

impl BrowserSession {
    pub fn new(
        profile_id: impl Into<String>,
        profile_name: impl Into<String>,
        pid: u32,
        mode: impl Into<String>,
        port: u16,
        ws_url: Option<String>,
        headless: bool,
        proxy: Option<String>,
    ) -> Self {
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            profile_id: profile_id.into(),
            profile_name: profile_name.into(),
            pid,
            mode: mode.into(),
            port,
            ws_url,
            headless,
            proxy,
            started_at_secs: now_secs,
        }
    }

    pub fn uptime_formatted(&self) -> String {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let elapsed = now.saturating_sub(self.started_at_secs);

        if elapsed < 60 {
            format!("{}s", elapsed)
        } else if elapsed < 3600 {
            format!("{}m {}s", elapsed / 60, elapsed % 60)
        } else if elapsed < 86400 {
            format!("{}h {}m", elapsed / 3600, (elapsed % 3600) / 60)
        } else {
            format!("{}d {}h", elapsed / 86400, (elapsed % 86400) / 3600)
        }
    }
}

pub fn pids_dir(base_dir: &Path) -> PathBuf {
    base_dir.join("pids")
}

pub fn save_session(session: &BrowserSession, base_dir: &Path) -> std::io::Result<PathBuf> {
    let dir = pids_dir(base_dir);
    std::fs::create_dir_all(&dir)?;
    let file_path = dir.join(format!("{}.json", session.profile_id));
    let json = serde_json::to_string_pretty(session)?;
    std::fs::write(&file_path, json)?;
    Ok(file_path)
}

pub fn remove_session(profile_id: &str, base_dir: &Path) {
    let file_path = pids_dir(base_dir).join(format!("{}.json", profile_id));
    let _ = std::fs::remove_file(file_path);
}

pub fn is_valid_browser_name(raw_name: &str) -> bool {
    let name = raw_name.to_lowercase();
    name.contains("chrome")
        || name.contains("chromium")
        || name.contains("specter")
        || name.contains("brave")
        || name.contains("msedge")
        || name.contains("edge")
}

pub fn is_pid_alive_and_browser(sys: &System, pid: u32) -> bool {
    let sys_pid = sysinfo::Pid::from_u32(pid);
    if let Some(proc) = sys.process(sys_pid) {
        let name = proc.name().to_string_lossy();
        is_valid_browser_name(&name)
    } else {
        false
    }
}

/// List all active browser sessions.
/// If `auto_prune` is true, stale or terminated PID files are deleted automatically.
pub fn list_sessions(base_dir: &Path, auto_prune: bool) -> Vec<BrowserSession> {
    let dir = pids_dir(base_dir);
    if !dir.exists() {
        return Vec::new();
    }

    let mut sys = System::new_all();
    sys.refresh_all();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    let mut active = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(session) = serde_json::from_str::<BrowserSession>(&content) {
                        if is_pid_alive_and_browser(&sys, session.pid) {
                            active.push(session);
                        } else if auto_prune {
                            let _ = std::fs::remove_file(&path);
                        }
                    } else if auto_prune {
                        let _ = std::fs::remove_file(&path);
                    }
                }
            }
        }
    }

    active.sort_by_key(|a| a.started_at_secs);
    active
}

/// Find active session by profile name, profile id, or prefix
pub fn find_active_session(query: &str, base_dir: &Path) -> Option<BrowserSession> {
    let sessions = list_sessions(base_dir, true);
    let target = query.trim();

    sessions
        .iter()
        .find(|s| {
            s.profile_id.eq_ignore_ascii_case(target)
                || s.profile_name.eq_ignore_ascii_case(target)
                || s.profile_id.starts_with(target)
                || s.profile_name.to_lowercase().contains(&target.to_lowercase())
        })
        .cloned()
}

/// Cleanly terminate a browser process tree and remove its session record
pub async fn terminate_session(
    session: &BrowserSession,
    base_dir: &Path,
    _force: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let pid = session.pid;

    #[cfg(target_os = "windows")]
    {
        let pid_str = pid.to_string();
        let mut cmd = tokio::process::Command::new("taskkill");
        // /F = Force termination, /T = Kill process and all child processes in the process tree
        cmd.args(["/F", "/T", "/PID", &pid_str])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        let _ = cmd.status().await;
    }

    #[cfg(not(target_os = "windows"))]
    {
        let pid_str = pid.to_string();
        let mut cmd = tokio::process::Command::new("kill");
        cmd.args(["-9", &pid_str])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        let _ = cmd.status().await;
    }

    // Clean up session descriptor
    remove_session(&session.profile_id, base_dir);

    // Sanitize lock files in user profile sandbox
    let sandbox_dir = base_dir.join("profiles").join(&session.profile_id);
    if sandbox_dir.exists() {
        crate::core::browser::sanitize_browser_profile(&sandbox_dir, false).await;
    }

    Ok(())
}

/// Allocate next available CDP port starting from requested port or 9222
pub fn allocate_cdp_port(requested_port: u16) -> Result<u16, String> {
    if requested_port > 0 {
        match std::net::TcpListener::bind(format!("127.0.0.1:{}", requested_port)) {
            Ok(listener) => {
                drop(listener);
                Ok(requested_port)
            }
            Err(_) => Err(format!(
                "Port {} is already occupied by another process. Choose a different port or use '--cdp' for auto-allocation.",
                requested_port
            )),
        }
    } else {
        // Auto-allocate first free port starting from 9222
        for port in 9222..=9299 {
            if let Ok(listener) = std::net::TcpListener::bind(format!("127.0.0.1:{}", port)) {
                drop(listener);
                return Ok(port);
            }
        }
        Err("No free CDP port available in range 9222-9299.".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_browser_name() {
        assert!(is_valid_browser_name("chrome.exe"));
        assert!(is_valid_browser_name("chromium"));
        assert!(is_valid_browser_name("fingerprint-chromium.exe"));
        assert!(is_valid_browser_name("specter.exe"));
        assert!(!is_valid_browser_name("notepad.exe"));
        assert!(!is_valid_browser_name("svchost.exe"));
    }

    #[test]
    fn test_uptime_formatting() {
        let mut session = BrowserSession::new(
            "test-id",
            "Test Profile",
            1234,
            "extension",
            0,
            None,
            false,
            None,
        );
        session.started_at_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            - 150;

        assert_eq!(session.uptime_formatted(), "2m 30s");
    }

    #[test]
    fn test_allocate_cdp_port() {
        let port = allocate_cdp_port(0);
        assert!(port.is_ok());
        let p = port.unwrap();
        assert!((9222..=9299).contains(&p));
    }

    #[test]
    fn test_is_pid_alive_current_process() {
        let mut sys = System::new_all();
        sys.refresh_all();
        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        let current_pid = std::process::id();
        let sys_pid = sysinfo::Pid::from_u32(current_pid);
        let proc = sys.process(sys_pid);
        assert!(proc.is_some(), "Process with PID {} should be found by sysinfo", current_pid);
    }
}
