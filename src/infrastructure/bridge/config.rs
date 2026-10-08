use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    pub workstation: Option<WorkstationConfig>,
    pub servers: HashMap<String, ServerConfig>,
    pub workloads: Option<WorkloadsConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkstationConfig {
    pub name: String,
    pub default_server: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub name: String,
    #[serde(rename = "type")]
    pub server_type: String, // "cloudflare" or "direct_ssh"
    pub cf_hostname: Option<String>,
    pub host: Option<String>,
    pub local_ssh_port: Option<u16>,
    pub remote_user: Option<String>,
    pub identity_file: Option<String>,
    pub socks_port: Option<u16>,
    pub ciphers: Option<Vec<String>>,
    pub server_alive_interval: Option<u32>,
    pub server_alive_count_max: Option<u32>,
    pub description: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub priority: Option<i32>,
}

impl ServerConfig {
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t.eq_ignore_ascii_case(tag))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadsConfig {
    pub git: Option<WorkloadGit>,
    pub supabase: Option<WorkloadSupabase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadGit {
    pub server: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadSupabase {
    pub server: String,
    pub socks_port: u16,
    pub http_port: u16,
}

#[derive(Debug, Clone)]
pub struct ConfigDiagnostic {
    pub level: DiagnosticLevel,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticLevel {
    Info,
    Warning,
    Error,
}

pub use crate::config::EnvExpander;

pub struct ServerSelector;

impl ServerSelector {
    /// Lọc danh sách server được kích hoạt (enabled != false)
    pub fn get_enabled(config: &BridgeConfig) -> Vec<(&String, &ServerConfig)> {
        config.servers
            .iter()
            .filter(|(_, srv)| srv.is_enabled())
            .collect()
    }

    /// Lọc danh sách server bị vô hiệu hóa (enabled == false)
    pub fn get_disabled(config: &BridgeConfig) -> Vec<(&String, &ServerConfig)> {
        config.servers
            .iter()
            .filter(|(_, srv)| !srv.is_enabled())
            .collect()
    }

    /// Lọc danh sách server theo tag
    pub fn filter_by_tag<'a>(config: &'a BridgeConfig, tag: &str) -> Vec<(&'a String, &'a ServerConfig)> {
        config.servers
            .iter()
            .filter(|(_, srv)| srv.is_enabled() && srv.has_tag(tag))
            .collect()
    }

    /// Lọc danh sách server theo pattern hoặc tên cụ thể
    pub fn filter_by_pattern<'a>(config: &'a BridgeConfig, pattern: &str) -> Vec<(&'a String, &'a ServerConfig)> {
        if pattern.eq_ignore_ascii_case("all") || pattern == "*" {
            return Self::get_enabled(config);
        }
        if let Some(prefix) = pattern.strip_suffix('*') {
            return config.servers
                .iter()
                .filter(|(id, srv)| srv.is_enabled() && id.starts_with(prefix))
                .collect();
        }
        config.servers
            .iter()
            .filter(|(id, _)| id.eq_ignore_ascii_case(pattern))
            .collect()
    }
}

impl BridgeConfig {
    /// Canonical root path for Specter SSOT storage (~/.specter/)
    pub fn canonical_dir() -> PathBuf {
        crate::config::canonical_ssot_dir()
    }

    /// Single Canonical config path: ~/.specter/bridge/bridge.json (SSOT)
    pub fn config_path() -> PathBuf {
        crate::config::canonical_ssot_dir().join("bridge").join("bridge.json")
    }

    /// Load the configuration with dynamic environment variable expansion
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let path = Self::config_path();
        if path.exists() {
            let content = fs::read_to_string(&path)?;
            let expanded = EnvExpander::expand(&content);
            let config: Self = serde_json::from_str(&expanded)?;
            return Ok(config);
        }

        // Return default built-in configuration if no file exists yet
        Ok(Self::default_config())
    }

    /// Save configuration back to canonical file ~/.specter/bridge/bridge.json
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let json_str = serde_json::to_string_pretty(self)?;
        fs::write(path, json_str)?;
        Ok(())
    }

    /// Enable or disable a server in the config and persist changes
    pub fn set_server_enabled(&mut self, server_id: &str, enabled: bool) -> Result<bool, Box<dyn std::error::Error>> {
        if let Some(srv) = self.servers.get_mut(server_id) {
            srv.enabled = Some(enabled);
            self.save()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn default_config() -> Self {
        let mut servers = HashMap::new();
        servers.insert(
            "my-vps".to_string(),
            ServerConfig {
                name: "My Primary VPS (Server A)".to_string(),
                server_type: "cloudflare".to_string(),
                cf_hostname: Some("vps.example.com".to_string()),
                host: None,
                local_ssh_port: Some(2222),
                remote_user: Some("root".to_string()),
                identity_file: Some("~/.ssh/id_ed25519".to_string()),
                socks_port: Some(1080),
                ciphers: Some(vec![
                    "chacha20-poly1305@openssh.com".to_string(),
                    "aes128-gcm@openssh.com".to_string(),
                ]),
                server_alive_interval: Some(15),
                server_alive_count_max: Some(3),
                description: Some("Primary VPS for Git SOCKS5 proxy and management".to_string()),
                enabled: Some(true),
                tags: vec!["primary".to_string(), "git".to_string()],
                priority: Some(1),
            },
        );

        Self {
            workstation: Some(WorkstationConfig {
                name: std::env::var("COMPUTERNAME").unwrap_or_else(|_| "WORKSTATION".to_string()),
                default_server: Some("my-vps".to_string()),
            }),
            servers,
            workloads: Some(WorkloadsConfig {
                git: Some(WorkloadGit {
                    server: "my-vps".to_string(),
                    port: 1080,
                }),
                supabase: Some(WorkloadSupabase {
                    server: "my-vps".to_string(),
                    socks_port: 1080,
                    http_port: 8118,
                }),
            }),
        }
    }

    /// Validate workloads binding to servers
    pub fn validate_workloads(&self) -> Vec<ConfigDiagnostic> {
        let mut diags = Vec::new();
        if let Some(workloads) = &self.workloads {
            if let Some(git) = &workloads.git {
                match self.servers.get(&git.server) {
                    None => {
                        diags.push(ConfigDiagnostic {
                            level: DiagnosticLevel::Error,
                            message: format!("Workload 'git': target server '{}' does not exist in configuration", git.server),
                        });
                    }
                    Some(srv) if !srv.is_enabled() => {
                        diags.push(ConfigDiagnostic {
                            level: DiagnosticLevel::Warning,
                            message: format!("Workload 'git': target server '{}' is disabled in configuration", git.server),
                        });
                    }
                    _ => {}
                }
            }

            if let Some(sb) = &workloads.supabase {
                match self.servers.get(&sb.server) {
                    None => {
                        diags.push(ConfigDiagnostic {
                            level: DiagnosticLevel::Error,
                            message: format!("Workload 'supabase': target server '{}' does not exist in configuration", sb.server),
                        });
                    }
                    Some(srv) if !srv.is_enabled() => {
                        diags.push(ConfigDiagnostic {
                            level: DiagnosticLevel::Warning,
                            message: format!("Workload 'supabase': target server '{}' is disabled in configuration", sb.server),
                        });
                    }
                    _ => {}
                }
            }
        }
        diags
    }

    /// Validate configuration integrity (port conflicts, key existence, endpoints)
    pub fn validate(&self) -> Vec<ConfigDiagnostic> {
        let mut diags = Vec::new();
        let mut used_local_ports = HashMap::new();
        let mut used_socks_ports = HashMap::new();

        for (id, srv) in &self.servers {
            // Check local SSH port conflict
            if let Some(port) = srv.local_ssh_port
                && let Some(prev) = used_local_ports.insert(port, id)
            {
                diags.push(ConfigDiagnostic {
                    level: DiagnosticLevel::Error,
                    message: format!("Port conflict: local_ssh_port {} used by both '{}' and '{}'", port, prev, id),
                });
            }

            // Check SOCKS port conflict
            if let Some(port) = srv.socks_port
                && let Some(prev) = used_socks_ports.insert(port, id)
            {
                diags.push(ConfigDiagnostic {
                    level: DiagnosticLevel::Error,
                    message: format!("Port conflict: socks_port {} used by both '{}' and '{}'", port, prev, id),
                });
            }

            // Check identity key existence
            if let Some(ref key_path_str) = srv.identity_file {
                let expanded = expand_home(key_path_str);
                if !expanded.exists() {
                    diags.push(ConfigDiagnostic {
                        level: DiagnosticLevel::Warning,
                        message: format!("Server '{}': IdentityFile not found at {:?}", id, expanded),
                    });
                }
            }

            // Check cloudflare hostname requirement
            if srv.server_type == "cloudflare" && srv.cf_hostname.is_none() {
                diags.push(ConfigDiagnostic {
                    level: DiagnosticLevel::Error,
                    message: format!("Server '{}': type='cloudflare' requires 'cf_hostname'", id),
                });
            }
        }

        diags.extend(self.validate_workloads());
        diags
    }
}

pub fn expand_home(path_str: &str) -> PathBuf {
    if let Some(stripped) = path_str.strip_prefix("~/") {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(stripped)
    } else {
        PathBuf::from(path_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_expander_basic_and_fallback() {
        unsafe {
            std::env::set_var("SPECTER_TEST_HOST", "vps.example.com");
        }

        let input = "Server at ${SPECTER_TEST_HOST} with port ${SPECTER_UNSET_PORT:-1080} and fallback ${SPECTER_FALLBACK:-cdn.local}";
        let output = EnvExpander::expand(input);
        assert_eq!(output, "Server at vps.example.com with port 1080 and fallback cdn.local");

        let input_unset = "Missing ${SPECTER_COMPLETELY_UNSET} variable";
        let output_unset = EnvExpander::expand(input_unset);
        assert_eq!(output_unset, "Missing  variable");
    }

    #[test]
    fn test_server_selector_enabled_and_disabled() {
        let mut config = BridgeConfig::default_config();
        config.servers.insert(
            "disabled-vps".to_string(),
            ServerConfig {
                name: "Disabled VPS".to_string(),
                server_type: "direct_ssh".to_string(),
                cf_hostname: None,
                host: Some("1.2.3.4".to_string()),
                local_ssh_port: Some(2223),
                remote_user: Some("root".to_string()),
                identity_file: None,
                socks_port: Some(1081),
                ciphers: None,
                server_alive_interval: None,
                server_alive_count_max: None,
                description: None,
                enabled: Some(false),
                tags: vec!["backup".to_string()],
                priority: None,
            },
        );

        let enabled = ServerSelector::get_enabled(&config);
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].0, "my-vps");

        let disabled = ServerSelector::get_disabled(&config);
        assert_eq!(disabled.len(), 1);
        assert_eq!(disabled[0].0, "disabled-vps");

        let primary_tag = ServerSelector::filter_by_tag(&config, "primary");
        assert_eq!(primary_tag.len(), 1);
        assert_eq!(primary_tag[0].0, "my-vps");

        let backup_tag = ServerSelector::filter_by_tag(&config, "backup");
        assert_eq!(backup_tag.len(), 0, "Disabled server with backup tag must not be included");
    }

    #[test]
    fn test_server_selector_patterns() {
        let mut config = BridgeConfig::default_config();
        config.servers.insert(
            "mmo-worker-1".to_string(),
            ServerConfig {
                name: "MMO 1".to_string(),
                server_type: "cloudflare".to_string(),
                cf_hostname: Some("worker1.example.com".to_string()),
                host: None,
                local_ssh_port: Some(2224),
                remote_user: Some("root".to_string()),
                identity_file: None,
                socks_port: Some(1082),
                ciphers: None,
                server_alive_interval: None,
                server_alive_count_max: None,
                description: None,
                enabled: Some(true),
                tags: vec!["mmo".to_string()],
                priority: None,
            },
        );

        let mmo_pattern = ServerSelector::filter_by_pattern(&config, "mmo-*");
        assert_eq!(mmo_pattern.len(), 1);
        assert_eq!(mmo_pattern[0].0, "mmo-worker-1");

        let all = ServerSelector::filter_by_pattern(&config, "all");
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_validate_workloads() {
        let mut config = BridgeConfig::default_config();
        config.workloads = Some(WorkloadsConfig {
            git: Some(WorkloadGit {
                server: "non-existent".to_string(),
                port: 1080,
            }),
            supabase: None,
        });

        let diags = config.validate_workloads();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].level, DiagnosticLevel::Error);
        assert!(diags[0].message.contains("does not exist"));
    }
}
