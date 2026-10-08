use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;
use serde::{Deserialize, Serialize};
use sysinfo::{Pid, System};
use tracing::info;

use super::config::{expand_home, ServerConfig};
use super::probe::{probe_port, wait_for_port};
use super::tools::find_executable;

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerPids {
    pub server_id: String,
    pub cf_pid: Option<u32>,
    pub ssh_pid: Option<u32>,
}

#[derive(Debug)]
pub struct StartServerResult {
    pub server_id: String,
    pub cf_port: Option<u16>,
    pub socks_port: Option<u16>,
    pub cf_started: bool,
    pub ssh_started: bool,
}

pub struct BridgeSupervisor;

impl BridgeSupervisor {
    pub fn pids_dir() -> PathBuf {
        let dir = super::config::BridgeConfig::canonical_dir().join("bridge").join("pids");
        let _ = fs::create_dir_all(&dir);
        dir
    }

    fn pid_file(server_id: &str) -> PathBuf {
        Self::pids_dir().join(format!("{}.json", server_id))
    }

    /// Starts a specific server connection (Cloudflare + SSH SOCKS5)
    pub async fn start_server(
        server_id: &str,
        srv: &ServerConfig,
        enable_ssh: bool,
    ) -> Result<StartServerResult, Box<dyn std::error::Error>> {
        let mut cf_pid = None;
        let mut ssh_pid = None;
        let mut cf_started = false;
        let mut ssh_started = false;

        let cf_port = srv.local_ssh_port.unwrap_or(2222);
        let socks_port = srv.socks_port.unwrap_or(1080);

        // 1. Start Cloudflare Tunnel if server_type == "cloudflare"
        if srv.server_type == "cloudflare" {
            let hostname = srv.cf_hostname.as_deref().ok_or_else(|| {
                format!("Server '{}': 'cf_hostname' is required for cloudflare tunnel", server_id)
            })?;

            if !probe_port(cf_port) {
                let cf_bin = find_executable("cloudflared").ok_or(
                    "Binary 'cloudflared' not found in PATH. Install via 'scoop install cloudflared' or package manager."
                )?;

                info!("Launching Cloudflare tunnel: {} -> 127.0.0.1:{}", hostname, cf_port);
                let mut cmd = Command::new(cf_bin);
                cmd.args(["access", "tcp", "--hostname", hostname, "--url", &format!("127.0.0.1:{}", cf_port)])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());

                #[cfg(windows)]
                {
                    use std::os::windows::process::CommandExt;
                    // CREATE_NO_WINDOW (0x08000000) | DETACHED_PROCESS (0x00000008)
                    cmd.creation_flags(0x08000008);
                }

                let child = cmd.spawn()?;
                cf_pid = Some(child.id());
                cf_started = true;

                // Wait for port to become active (up to 10s)
                if !wait_for_port(cf_port, Duration::from_secs(10)).await {
                    return Err(format!("Cloudflare tunnel on port {} failed to initialize within 10s", cf_port).into());
                }
            } else {
                info!("Cloudflare tunnel on port {} is already active", cf_port);
            }
        }

        // 2. Start SSH SOCKS5 Tunnel
        if !probe_port(socks_port) {
            let ssh_bin = find_executable("ssh").ok_or(
                "Binary 'ssh' not found in PATH. Please install OpenSSH client."
            )?;

            let remote_user = srv.remote_user.as_deref().unwrap_or("root");
            let target_host = if srv.server_type == "cloudflare" {
                "127.0.0.1"
            } else {
                srv.host.as_deref().unwrap_or("127.0.0.1")
            };
            let connect_port = if srv.server_type == "cloudflare" {
                cf_port
            } else {
                srv.local_ssh_port.unwrap_or(22)
            };

            info!("Launching SSH SOCKS5 proxy on port 127.0.0.1:{} -> {}@{}:{}", socks_port, remote_user, target_host, connect_port);

            let mut cmd = Command::new(ssh_bin);
            cmd.arg("-p")
                .arg(connect_port.to_string())
                .arg("-N")
                .arg("-D")
                .arg(socks_port.to_string());

            if enable_ssh && srv.server_type != "cloudflare" {
                cmd.arg("-L").arg(format!("{}:127.0.0.1:22", srv.local_ssh_port.unwrap_or(2222)));
            }

            cmd.arg("-o")
                .arg("BatchMode=yes")
                .arg("-o")
                .arg("StrictHostKeyChecking=no")
                .arg("-o")
                .arg(format!("ServerAliveInterval={}", srv.server_alive_interval.unwrap_or(15)))
                .arg("-o")
                .arg(format!("ServerAliveCountMax={}", srv.server_alive_count_max.unwrap_or(3)))
                .arg("-o")
                .arg("ExitOnForwardFailure=yes");

            if let Some(ref ciphers) = srv.ciphers
                && !ciphers.is_empty()
            {
                cmd.arg("-o").arg(format!("Ciphers={}", ciphers.join(",")));
            }

            if let Some(ref id_file) = srv.identity_file {
                let expanded = expand_home(id_file);
                if expanded.exists() {
                    cmd.arg("-i").arg(expanded);
                }
            }

            cmd.arg(format!("{}@{}", remote_user, target_host))
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());

            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                // CREATE_NO_WINDOW (0x08000000) | DETACHED_PROCESS (0x00000008)
                cmd.creation_flags(0x08000008);
            }

            let child = cmd.spawn()?;
            ssh_pid = Some(child.id());
            ssh_started = true;

            // Wait for SOCKS5 port to become active (up to 10s)
            if !wait_for_port(socks_port, Duration::from_secs(10)).await {
                return Err(format!("SSH SOCKS5 proxy on port {} failed to initialize within 10s", socks_port).into());
            }
        } else {
            info!("SOCKS5 proxy on port {} is already active", socks_port);
        }

        // Save recorded PIDs
        let pids_data = ServerPids {
            server_id: server_id.to_string(),
            cf_pid,
            ssh_pid,
        };
        if let Ok(json) = serde_json::to_string_pretty(&pids_data) {
            let _ = fs::write(Self::pid_file(server_id), json);
        }

        Ok(StartServerResult {
            server_id: server_id.to_string(),
            cf_port: Some(cf_port),
            socks_port: Some(socks_port),
            cf_started,
            ssh_started,
        })
    }

    /// Stops a specific server's background daemons
    pub fn stop_server(server_id: &str, srv: &ServerConfig) -> Result<(), Box<dyn std::error::Error>> {
        let mut sys = System::new_all();
        sys.refresh_all();

        let pid_path = Self::pid_file(server_id);
        if pid_path.exists() {
            if let Ok(content) = fs::read_to_string(&pid_path)
                && let Ok(pids) = serde_json::from_str::<ServerPids>(&content)
            {
                if let Some(pid) = pids.cf_pid {
                    kill_pid(&sys, pid);
                }
                if let Some(pid) = pids.ssh_pid {
                    kill_pid(&sys, pid);
                }
            }
            let _ = fs::remove_file(&pid_path);
        }

        // Also proactively kill by matching process commandline arguments for safety
        let cf_port_str = srv.local_ssh_port.unwrap_or(2222).to_string();
        let socks_port_str = srv.socks_port.unwrap_or(1080).to_string();

        for (pid, process) in sys.processes() {
            let name = process.name().to_string_lossy().to_lowercase();
            let cmdline = process.cmd().iter().map(|s| s.to_string_lossy()).collect::<Vec<_>>().join(" ");

            if name.contains("cloudflared") && cmdline.contains(&cf_port_str) {
                let _ = process.kill();
                info!("Terminated cloudflared process (PID {})", pid);
            }
            if name.contains("ssh") && cmdline.contains(&socks_port_str) {
                let _ = process.kill();
                info!("Terminated ssh process (PID {})", pid);
            }
        }

        Ok(())
    }

    /// Stops all running bridge processes and cleans up state
    pub fn stop_all() -> Result<(), Box<dyn std::error::Error>> {
        let mut sys = System::new_all();
        sys.refresh_all();

        // 1. Terminate tracked processes from PID files first
        let pids_dir = Self::pids_dir();
        if let Ok(entries) = fs::read_dir(&pids_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Some(pids) = fs::read_to_string(&path)
                        .ok()
                        .and_then(|content| serde_json::from_str::<ServerPids>(&content).ok())
                    {
                        if let Some(pid) = pids.cf_pid {
                            kill_pid(&sys, pid);
                        }
                        if let Some(pid) = pids.ssh_pid {
                            kill_pid(&sys, pid);
                        }
                    }
                    let _ = fs::remove_file(&path);
                }
            }
        }

        // 2. Discover dynamically configured ports from BridgeConfig
        let (configured_cf_ports, configured_socks_ports) = if let Ok(config) = super::config::BridgeConfig::load() {
            let mut cf_ports = Vec::new();
            let mut socks_ports = Vec::new();
            for srv in config.servers.values() {
                cf_ports.push(srv.local_ssh_port.unwrap_or(2222).to_string());
                socks_ports.push(srv.socks_port.unwrap_or(1080).to_string());
            }
            (cf_ports, socks_ports)
        } else {
            (vec!["2222".to_string()], vec!["1080".to_string()])
        };

        // 3. Proactively stop any residual bridge processes matching configured ports
        for (pid, process) in sys.processes() {
            let name = process.name().to_string_lossy().to_lowercase();
            let cmdline = process.cmd().iter().map(|s| s.to_string_lossy()).collect::<Vec<_>>().join(" ");

            if name.contains("cloudflared") && cmdline.contains("access tcp")
                && (configured_cf_ports.iter().any(|p| cmdline.contains(p)) || configured_cf_ports.is_empty()) {
                let _ = process.kill();
                info!("Stopped cloudflared tunnel PID {}", pid);
            }
            if name.contains("ssh") && cmdline.contains("-D")
                && (configured_socks_ports.iter().any(|p| cmdline.contains(p)) || configured_socks_ports.is_empty()) {
                let _ = process.kill();
                info!("Stopped ssh SOCKS5 tunnel PID {}", pid);
            }
        }

        Ok(())
    }
}

pub(crate) fn is_valid_bridge_process_name(raw_name: &str) -> bool {
    let name = raw_name.to_lowercase();
    name.contains("cloudflared") || name.contains("ssh") || name.contains("specter")
}

fn kill_pid(sys: &System, pid_u32: u32) {
    let pid = Pid::from(pid_u32 as usize);
    if let Some(proc) = sys.process(pid) {
        let name = proc.name().to_string_lossy();
        if is_valid_bridge_process_name(&name) {
            let _ = proc.kill();
            info!("Killed PID {} ({})", pid_u32, name);
        } else {
            tracing::warn!(
                "Refusing to kill PID {} with name '{}': not cloudflared, ssh, or specter (stale PID)",
                pid_u32,
                name
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_bridge_process_name() {
        assert!(is_valid_bridge_process_name("cloudflared"));
        assert!(is_valid_bridge_process_name("cloudflared.exe"));
        assert!(is_valid_bridge_process_name("ssh"));
        assert!(is_valid_bridge_process_name("ssh.exe"));
        assert!(is_valid_bridge_process_name("specter"));
        assert!(is_valid_bridge_process_name("specter.exe"));

        // Innocent processes must NOT match
        assert!(!is_valid_bridge_process_name("explorer.exe"));
        assert!(!is_valid_bridge_process_name("chrome.exe"));
        assert!(!is_valid_bridge_process_name("firefox.exe"));
        assert!(!is_valid_bridge_process_name("node.exe"));
        assert!(!is_valid_bridge_process_name("svchost.exe"));
    }
}

