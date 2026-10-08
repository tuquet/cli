use std::path::PathBuf;
use crate::config::automa::AutomaConfig;
use crate::config::browser::BrowserConfig;
use crate::config::runner::RunnerConfig;
use crate::config::system::SystemConfig;
use crate::infrastructure::bridge::BridgeConfig;
use tuquet_faker::config::FakerConfig;
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

    pub fn open_in_editor(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_all_contains_six_pillars() {
        let list = ConfigRegistry::list_all();
        assert_eq!(list.len(), 6);
        let pillars: Vec<&str> = list.iter().map(|e| e.pillar).collect();
        assert!(pillars.contains(&"bridge"));
        assert!(pillars.contains(&"faker"));
        assert!(pillars.contains(&"browser"));
        assert!(pillars.contains(&"automa"));
        assert!(pillars.contains(&"runner"));
        assert!(pillars.contains(&"system"));

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
        assert!(ConfigRegistry::resolve_service_path("non_existent").is_none());
    }
}


