use std::path::PathBuf;
use crate::config::automa::AutomaConfig;
use crate::config::browser::BrowserConfig;
use crate::config::inbox::InboxConfig;
use crate::config::runner::RunnerConfig;
use crate::config::system::SystemConfig;
use crate::infrastructure::bridge::BridgeConfig;
use specter_faker::config::FakerConfig;
use crate::ui::{badge_online, Card};

pub struct PillarConfigEntry {
    pub pillar: &'static str,
    pub title: &'static str,
    pub path: PathBuf,
    pub exists: bool,
}

pub struct ConfigRegistry;

impl ConfigRegistry {
    pub const ALL_SERVICES: &'static [&'static str] = crate::constants::ALL_PILLARS;

    pub fn list_all() -> Vec<PillarConfigEntry> {
        let bridge_path = BridgeConfig::config_path();
        let faker_path = FakerConfig::config_path();
        let browser_path = BrowserConfig::config_path();
        let automa_path = AutomaConfig::config_path();
        let runner_path = RunnerConfig::config_path();
        let system_path = SystemConfig::config_path();
        let inbox_path = InboxConfig::config_path();

        vec![
            PillarConfigEntry {
                pillar: crate::constants::PILLAR_BRIDGE,
                title: "Network Bridge & Multi-VPS Mesh",
                exists: bridge_path.exists(),
                path: bridge_path,
            },
            PillarConfigEntry {
                pillar: crate::constants::PILLAR_FAKER,
                title: "Synthetic Identity & Persona Generator",
                exists: faker_path.exists(),
                path: faker_path,
            },
            PillarConfigEntry {
                pillar: crate::constants::PILLAR_BROWSER,
                title: "Dedicated Isolated Chromium Runtime",
                exists: browser_path.exists(),
                path: browser_path,
            },
            PillarConfigEntry {
                pillar: crate::constants::PILLAR_AUTOMA,
                title: "Browser Automation Workflow Engine",
                exists: automa_path.exists(),
                path: automa_path,
            },
            PillarConfigEntry {
                pillar: crate::constants::PILLAR_RUNNER,
                title: "Distributed Worker Daemon & Host Node",
                exists: runner_path.exists(),
                path: runner_path,
            },
            PillarConfigEntry {
                pillar: crate::constants::PILLAR_SYSTEM,
                title: "Machine Identity & Cloud Fleet Enrollment",
                exists: system_path.exists(),
                path: system_path,
            },
            PillarConfigEntry {
                pillar: crate::constants::PILLAR_INBOX,
                title: "Catch-All Email & OTP Interceptor Subsystem",
                exists: inbox_path.exists(),
                path: inbox_path,
            },
        ]
    }

    pub fn canonical_service(service: &str) -> Option<&'static str> {
        match service.to_ascii_lowercase().as_str() {
            "bridge" | "tunnel" | "vps" => Some(crate::constants::PILLAR_BRIDGE),
            "faker" | "user" | "persona" => Some(crate::constants::PILLAR_FAKER),
            "browser" | "chromium" | "chrome" => Some(crate::constants::PILLAR_BROWSER),
            "automa" | "workflow" => Some(crate::constants::PILLAR_AUTOMA),
            "runner" | "worker" | "daemon" => Some(crate::constants::PILLAR_RUNNER),
            "cloud" | "system" => Some(crate::constants::PILLAR_SYSTEM),
            "inbox" | "mail" | "otp" => Some(crate::constants::PILLAR_INBOX),
            _ => None,
        }
    }

    pub fn resolve_service_path(service: &str) -> Option<PathBuf> {
        match Self::canonical_service(service) {
            Some(crate::constants::PILLAR_BRIDGE) => Some(BridgeConfig::config_path()),
            Some(crate::constants::PILLAR_FAKER) => Some(FakerConfig::config_path()),
            Some(crate::constants::PILLAR_BROWSER) => Some(BrowserConfig::config_path()),
            Some(crate::constants::PILLAR_AUTOMA) => Some(AutomaConfig::config_path()),
            Some(crate::constants::PILLAR_RUNNER) => Some(RunnerConfig::config_path()),
            Some(crate::constants::PILLAR_SYSTEM) => Some(SystemConfig::config_path()),
            Some(crate::constants::PILLAR_INBOX) => Some(InboxConfig::config_path()),
            _ => None,
        }
    }

    pub fn render_overview_card() {
        println!();
        let mut card = Card::new("SPECTER ECOSYSTEM CONFIGURATION REGISTRY");
        card.with_badge(badge_online("SSOT READY"));
        card.with_min_width(74);

        for entry in Self::list_all() {
            let label = format!("{: <9}", entry.pillar.to_uppercase());
            let status = if entry.exists { "" } else { " (auto-init)" };
            card.add_kv(&label, format!("{}{}", entry.path.display(), status));
        }

        card.with_footer("Tip: 'specter <service> config' for raw path or '--edit' to open in editor");
        card.print();
        println!();
    }

    pub fn schema_url(service: &str) -> String {
        let canonical = Self::canonical_service(service).unwrap_or(service);
        format!(
            "https://raw.githubusercontent.com/tuquet/schema/main/config/{}.schema.json",
            canonical
        )
    }

    pub fn get_default_json(service: &str) -> serde_json::Value {
        match Self::canonical_service(service) {
            Some(crate::constants::PILLAR_BRIDGE) => {
                serde_json::to_value(crate::infrastructure::bridge::BridgeConfig::default()).unwrap_or_default()
            }
            Some(crate::constants::PILLAR_FAKER) => {
                serde_json::to_value(specter_faker::config::FakerConfig::default()).unwrap_or_default()
            }
            Some(crate::constants::PILLAR_BROWSER) => {
                serde_json::to_value(crate::config::BrowserConfig::default()).unwrap_or_default()
            }
            Some(crate::constants::PILLAR_AUTOMA) => {
                serde_json::to_value(crate::config::AutomaConfig::default()).unwrap_or_default()
            }
            Some(crate::constants::PILLAR_RUNNER) => {
                serde_json::to_value(crate::config::RunnerConfig::default()).unwrap_or_default()
            }
            Some(crate::constants::PILLAR_SYSTEM) => {
                serde_json::to_value(crate::config::SystemConfig::default()).unwrap_or_default()
            }
            Some(crate::constants::PILLAR_INBOX) => {
                serde_json::to_value(crate::config::InboxConfig::default()).unwrap_or_default()
            }
            _ => serde_json::json!({}),
        }
    }

    pub fn ensure_schema_field(path: &std::path::Path, service: &str) -> Result<(), Box<dyn std::error::Error>> {
        let canonical = match Self::canonical_service(service) {
            Some(c) => c,
            None => return Ok(()),
        };

        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let url = Self::schema_url(canonical);

        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(&content) {
                    let need_update = match map.get("$schema") {
                        Some(serde_json::Value::String(s)) => s != &url,
                        _ => true,
                    };

                    if need_update {
                        let mut ordered = serde_json::Map::new();
                        ordered.insert("$schema".to_string(), serde_json::Value::String(url));
                        for (k, v) in map {
                            if k != "$schema" {
                                ordered.insert(k, v);
                            }
                        }
                        let pretty = serde_json::to_string_pretty(&serde_json::Value::Object(ordered))?;
                        std::fs::write(path, pretty)?;
                    }
                }
            }
        } else {
            let default_val = Self::get_default_json(canonical);
            let mut ordered = serde_json::Map::new();
            ordered.insert("$schema".to_string(), serde_json::Value::String(url));
            if let serde_json::Value::Object(map) = default_val {
                for (k, v) in map {
                    if k != "$schema" {
                        ordered.insert(k, v);
                    }
                }
            }
            let pretty = serde_json::to_string_pretty(&serde_json::Value::Object(ordered))?;
            std::fs::write(path, pretty)?;
        }

        Ok(())
    }

    pub fn open_in_editor(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let service = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str());
        if let Some(srv) = service {
            let _ = Self::ensure_schema_field(path, srv);
        }

        println!("Opening {} in default editor...", path.display());
        if cfg!(windows) {
            std::process::Command::new("notepad.exe").arg(path).spawn()?;
        } else if let Ok(editor) = std::env::var("EDITOR") {
            std::process::Command::new(editor).arg(path).status()?;
        } else {
            println!("Open with your preferred editor: {}", path.display());
        }
        Ok(())
    }

    pub fn open_in_editor_with_service(path: &std::path::Path, service: &str) -> Result<(), Box<dyn std::error::Error>> {
        let _ = Self::ensure_schema_field(path, service);
        Self::open_in_editor(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_all_contains_seven_pillars() {
        let list = ConfigRegistry::list_all();
        assert_eq!(list.len(), 7);
        let pillars: Vec<&str> = list.iter().map(|e| e.pillar).collect();
        assert!(pillars.contains(&"bridge"));
        assert!(pillars.contains(&"faker"));
        assert!(pillars.contains(&"browser"));
        assert!(pillars.contains(&"automa"));
        assert!(pillars.contains(&"runner"));
        assert!(pillars.contains(&"system"));
        assert!(pillars.contains(&"inbox"));

        for entry in list {
            assert!(entry.path.to_string_lossy().contains(".specter"));
        }
    }

    #[test]
    fn test_resolve_service_path_aliases() {
        assert!(ConfigRegistry::resolve_service_path("bridge").is_some());
        assert!(ConfigRegistry::resolve_service_path("tunnel").is_some());
        assert!(ConfigRegistry::resolve_service_path("faker").is_some());
        assert!(ConfigRegistry::resolve_service_path("user").is_some());
        assert!(ConfigRegistry::resolve_service_path("browser").is_some());
        assert!(ConfigRegistry::resolve_service_path("automa").is_some());
        assert!(ConfigRegistry::resolve_service_path("runner").is_some());
        assert!(ConfigRegistry::resolve_service_path("cloud").is_some());
        assert!(ConfigRegistry::resolve_service_path("system").is_some());
        assert!(ConfigRegistry::resolve_service_path("inbox").is_some());
        assert!(ConfigRegistry::resolve_service_path("mail").is_some());
        assert!(ConfigRegistry::resolve_service_path("otp").is_some());
        assert!(ConfigRegistry::resolve_service_path("non_existent").is_none());
    }

    #[test]
    fn test_schema_url_resolution() {
        assert_eq!(
            ConfigRegistry::schema_url("faker"),
            "https://raw.githubusercontent.com/tuquet/schema/main/config/faker.schema.json"
        );
        assert_eq!(
            ConfigRegistry::schema_url("browser"),
            "https://raw.githubusercontent.com/tuquet/schema/main/config/browser.schema.json"
        );
    }

    #[test]
    fn test_get_default_json_all_services() {
        for service in ["bridge", "faker", "browser", "automa", "runner", "system", "inbox"] {
            let def = ConfigRegistry::get_default_json(service);
            assert!(def.is_object(), "Default for {} must be a JSON object", service);
        }
    }

    #[test]
    fn test_ensure_schema_field_injection() {
        let temp_dir = std::env::temp_dir().join(format!("specter_test_{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let test_file = temp_dir.join("faker.json");

        // 1. New file creation with $schema
        ConfigRegistry::ensure_schema_field(&test_file, "faker").expect("Ensure schema succeeds");
        assert!(test_file.exists());
        let content = std::fs::read_to_string(&test_file).expect("Read test file");
        let val: serde_json::Value = serde_json::from_str(&content).expect("Valid JSON");
        assert!(val.get("$schema").is_some());
        assert_eq!(
            val["$schema"].as_str().unwrap(),
            "https://raw.githubusercontent.com/tuquet/schema/main/config/faker.schema.json"
        );

        // 2. Existing file without $schema gets $schema injected
        std::fs::write(&test_file, "{\"default_nat\": \"US\"}").expect("Write raw file");
        ConfigRegistry::ensure_schema_field(&test_file, "faker").expect("Ensure schema on existing");
        let content2 = std::fs::read_to_string(&test_file).expect("Read test file");
        let val2: serde_json::Value = serde_json::from_str(&content2).expect("Valid JSON");
        assert!(val2.get("$schema").is_some());
        assert_eq!(val2["default_nat"].as_str().unwrap(), "US");

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}


