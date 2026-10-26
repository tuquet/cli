use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Workflow,
    Browser,
    Gateway,
    Auth,
}

impl ActiveTab {
    pub fn next(self) -> Self {
        match self {
            Self::Workflow => Self::Browser,
            Self::Browser => Self::Gateway,
            Self::Gateway => Self::Auth,
            Self::Auth => Self::Workflow,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Workflow => Self::Auth,
            Self::Browser => Self::Workflow,
            Self::Gateway => Self::Browser,
            Self::Auth => Self::Gateway,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Workflow => "[1] WORKFLOW",
            Self::Browser => "[2] BROWSER",
            Self::Gateway => "[3] GATEWAY",
            Self::Auth => "[4] AUTH",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayState {
    pub socks5_online: bool,
    pub socks5_port: u16,
    pub http_online: bool,
    pub http_port: u16,
    pub ssh_online: bool,
    pub ssh_port: u16,
    pub default_server: String,
    pub active_servers: usize,
    pub total_servers: usize,
}

impl Default for GatewayState {
    fn default() -> Self {
        Self {
            socks5_online: false,
            socks5_port: 1080,
            http_online: false,
            http_port: 8118,
            ssh_online: false,
            ssh_port: 2222,
            default_server: "my-vps".to_string(),
            active_servers: 0,
            total_servers: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserState {
    pub installed: bool,
    pub version: String,
    pub executable_path: String,
    pub running_pids_count: usize,
    pub profile_count: usize,
}

impl Default for BrowserState {
    fn default() -> Self {
        Self {
            installed: false,
            version: "N/A".to_string(),
            executable_path: "N/A".to_string(),
            running_pids_count: 0,
            profile_count: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerState {
    pub daemon_online: bool,
    pub port: u16,
    pub workflow_count: usize,
    pub db_size_str: String,
    pub active_jobs: usize,
}

impl Default for RunnerState {
    fn default() -> Self {
        Self {
            daemon_online: false,
            port: 8765,
            workflow_count: 0,
            db_size_str: "N/A".to_string(),
            active_jobs: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudState {
    pub enrolled: bool,
    pub host_name: String,
    pub tenant_name: String,
    pub cloud_url: String,
}

impl Default for CloudState {
    fn default() -> Self {
        Self {
            enrolled: false,
            host_name: "WORKSTATION".to_string(),
            tenant_name: "Personal Workspace".to_string(),
            cloud_url: "Local Only".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TuiState {
    pub active_tab: ActiveTab,
    pub gateway: GatewayState,
    pub browser: BrowserState,
    pub runner: RunnerState,
    pub cloud: CloudState,
    pub status_message: String,
    pub last_tick: Instant,
    pub tick_count: u64,
}

impl Default for TuiState {
    fn default() -> Self {
        Self {
            active_tab: ActiveTab::Workflow,
            gateway: GatewayState::default(),
            browser: BrowserState::default(),
            runner: RunnerState::default(),
            cloud: CloudState::default(),
            status_message: "Ready. Hotkeys: (b) Toggle Gateway, (r) Refresh, (q) Quit."
                .to_string(),
            last_tick: Instant::now(),
            tick_count: 0,
        }
    }
}

impl TuiState {
    pub async fn refresh(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);
        self.last_tick = Instant::now();

        // 1. Probe Gateway
        let gateway_cfg = crate::infrastructure::gateway::GatewayConfig::load()
            .unwrap_or_else(|_| crate::infrastructure::gateway::GatewayConfig::default_config());
        let git_port = gateway_cfg
            .workloads
            .as_ref()
            .and_then(|w| w.git.as_ref())
            .map(|g| g.port)
            .unwrap_or(1080);
        let http_port = gateway_cfg
            .workloads
            .as_ref()
            .and_then(|w| w.supabase.as_ref())
            .map(|s| s.http_port)
            .unwrap_or(8118);
        let ssh_port = gateway_cfg
            .servers
            .get("my-vps")
            .and_then(|s| s.local_ssh_port)
            .unwrap_or(2222);

        let (git_active, http_active, ssh_active) = tokio::join!(
            crate::infrastructure::gateway::probe_port_async(git_port),
            crate::infrastructure::gateway::probe_port_async(http_port),
            crate::infrastructure::gateway::probe_port_async(ssh_port),
        );

        self.gateway = GatewayState {
            socks5_online: git_active,
            socks5_port: git_port,
            http_online: http_active,
            http_port,
            ssh_online: ssh_active,
            ssh_port,
            default_server: gateway_cfg
                .workstation
                .as_ref()
                .and_then(|w| w.default_server.clone())
                .unwrap_or_else(|| "my-vps".to_string()),
            active_servers: if git_active || ssh_active || http_active {
                1
            } else {
                0
            },
            total_servers: gateway_cfg.servers.len(),
        };

        // 2. Probe Browser Fleet
        let base_dir = crate::core::browser::resolve_data_dir();
        let browser_status = crate::core::browser::resolver::get_runtime_status();
        let sessions = crate::core::browser::pid_tracker::list_sessions(&base_dir, true);
        let profile_count = crate::core::browser::BrowserProfile::list_all(&base_dir)
            .map(|list| list.len())
            .unwrap_or(0);

        self.browser = BrowserState {
            installed: browser_status.installed,
            version: browser_status.pinned_version,
            executable_path: browser_status.executable_path,
            running_pids_count: sessions.len(),
            profile_count,
        };

        // 3. Probe Runner & Automation
        let config = crate::config::AppConfig::load();
        let host = std::env::var(crate::constants::ENV_DAEMON_HOST)
            .unwrap_or_else(|_| crate::constants::DEFAULT_HOST.to_string());
        let port = std::env::var(crate::constants::ENV_DAEMON_PORT)
            .unwrap_or_else(|_| config.server_port.to_string());
        let port_u16 = port.parse::<u16>().unwrap_or(8765);
        let health_url = format!("http://{}:{}/api/v1/health", host, port);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(400))
            .build();
        let daemon_running = match client {
            Ok(c) => match c.get(&health_url).send().await {
                Ok(res) => res.status().is_success(),
                _ => false,
            },
            _ => false,
        };

        let db_path = std::path::PathBuf::from(&config.data_dir)
            .join(crate::constants::FILE_AUTOMATION_SQLITE);
        let db_size_str = if db_path.exists() {
            std::fs::metadata(&db_path)
                .map(|meta| format!("{:.2} KB", meta.len() as f64 / 1024.0))
                .unwrap_or_else(|_| "Present".to_string())
        } else {
            "Not initialized".to_string()
        };
        let workflow_count =
            match crate::infrastructure::cloud::CloudWorkflowRepository::connect().await {
                Ok(repo) => repo.count().await.unwrap_or(0),
                Err(_) => 0,
            };

        self.runner = RunnerState {
            daemon_online: daemon_running,
            port: port_u16,
            workflow_count,
            db_size_str,
            active_jobs: 0,
        };

        // 4. Probe Cloud Identity
        let cloud_creds =
            crate::infrastructure::cloud_reporter::CloudReporter::whoami(&config.data_dir).await;
        let hostname = std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_else(|_| "WORKSTATION".to_string());

        if let Some(creds) = cloud_creds {
            self.cloud = CloudState {
                enrolled: true,
                host_name: creds.name,
                tenant_name: creds
                    .tenant_id
                    .unwrap_or_else(|| "Enterprise Corp".to_string()),
                cloud_url: creds.cloud_url.unwrap_or_else(|| "Cloud Mesh".to_string()),
            };
        } else {
            self.cloud = CloudState {
                enrolled: false,
                host_name: hostname,
                tenant_name: "Personal Workspace".to_string(),
                cloud_url: "Local Only (Unpaired)".to_string(),
            };
        }
    }
}
