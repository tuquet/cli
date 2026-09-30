use std::path::PathBuf;
use crate::cli::{BrowserCommands, ExtCommands};
use crate::ui::{badge_error, badge_online, badge_warn, Card};
use crate::core::browser::{Extension, ExtensionRegistry};

pub async fn handle(command: BrowserCommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        BrowserCommands::Install { force, revision } => {
            match crate::core::browser::resolver::download_chromium_runtime(force, revision.as_deref()).await {
                Ok(path) => {
                    let mut card = Card::new("CHROMIUM RUNTIME");
                    card.with_badge(badge_online("INSTALLED"));
                    card.with_min_width(64);
                    card.add_kv("Engine", "Chromium (Pure Open Source - BSD-3-Clause)");
                    card.add_kv("Executable", path);
                    card.with_footer("Dedicated browser runtime is ready for automated execution");
                    println!();
                    card.print();
                    println!();
                    Ok(())
                }
                Err(e) => {
                    Err(format!("Failed to install Chromium runtime: {}", e).into())
                }
            }
        }
        BrowserCommands::Status => {
            let status = crate::core::browser::resolver::get_runtime_status();
            let badge = if status.installed {
                badge_online("INSTALLED")
            } else {
                badge_warn("NOT INSTALLED")
            };
            let mut card = Card::new("CHROMIUM RUNTIME");
            card.with_badge(badge);
            card.with_min_width(64);
            card.add_kv("Engine", "Chromium (Pure Open Source - BSD-3-Clause)");
            card.add_kv("Platform", &status.platform);
            card.add_kv("Revision", &status.pinned_version);
            card.add_kv("Executable", &status.executable_path);
            card.add_kv("Directory", &status.directory);
            if let Some(mb) = status.size_mb {
                card.add_kv("Disk Usage", format!("{:.1} MB", mb));
            }
            if !status.installed {
                card.with_footer("Run 'tuquet browser install' to download and setup");
            }
            println!();
            card.print();
            println!();
            Ok(())
        }
        BrowserCommands::Clean => {
            match crate::core::browser::resolver::clean_runtime() {
                Ok(_) => {
                    let mut card = Card::new("CHROMIUM RUNTIME");
                    card.with_badge(badge_online("CLEANED"));
                    card.with_min_width(64);
                    card.add_line("Removed dedicated browser runtime to reclaim disk space.");
                    card.with_footer("Run 'tuquet browser install' when you need to reinstall");
                    println!();
                    card.print();
                    println!();
                    Ok(())
                }
                Err(e) => {
                    Err(format!("Failed to clean runtime: {}", e).into())
                }
            }
        }
        BrowserCommands::Path => {
            match crate::core::browser::resolver::resolve_executable_path("default").await {
                Ok(path) => {
                    println!("{}", path);
                    Ok(())
                }
                Err(e) => {
                    Err(e.into())
                }
            }
        }
        BrowserCommands::Ext { command: ext_subcmd, browser, extension_path } => {
            handle_ext(ext_subcmd, &browser, extension_path).await
        }
    }
}

pub async fn handle_ext(
    subcmd: Option<ExtCommands>,
    browser: &str,
    legacy_path: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    if legacy_path.is_some() {
        return setup_extension(browser, legacy_path).await;
    }

    let mut registry = ExtensionRegistry::load();

    match subcmd {
        None | Some(ExtCommands::List) => {
            let list = registry.list();
            let mut card = Card::new("BROWSER EXTENSIONS");
            card.with_badge(badge_online(&format!("{} REGISTERED", list.len())));
            card.with_min_width(68);

            for ext in &list {
                let status_icon = if !ext.path.exists() {
                    "⚠ MISSING ON DISK"
                } else if ext.enabled {
                    "● ACTIVE"
                } else {
                    "○ DISABLED"
                };
                let tag = if ext.is_builtin { " (built-in)" } else { "" };
                card.add_kv(
                    &format!("ID: {}{}", ext.id, tag),
                    format!("{} [v{}] - {}", ext.name, ext.version, status_icon),
                );
                card.add_kv("  Path", ext.path.display().to_string());
            }

            card.add_line("");
            card.add_line("Management Commands:");
            card.add_line("  tuquet browser ext add <path>       Register custom extension");
            card.add_line("  tuquet browser ext enable <id>      Enable extension for sessions");
            card.add_line("  tuquet browser ext disable <id>     Disable extension");
            card.add_line("  tuquet browser ext remove <id>      Unregister custom extension");
            card.add_line("  tuquet browser ext info <id>        View extension details");
            card.add_line("  tuquet browser ext launch           Launch browser with extensions");
            card.with_footer("Registry SSOT: ~/.tuquet/extensions/registry.json");

            println!();
            card.print();
            println!();
            Ok(())
        }
        Some(ExtCommands::Add { path, id }) => {
            match Extension::from_unpacked_dir(&path, id) {
                Ok(ext) => {
                    let ext_id = ext.id.clone();
                    let name = ext.name.clone();
                    let version = ext.version.clone();
                    let resolved_path = ext.path.display().to_string();
                    registry.register(ext)?;

                    let mut card = Card::new("BROWSER EXTENSION");
                    card.with_badge(badge_online("REGISTERED"));
                    card.with_min_width(64);
                    card.add_kv("ID", ext_id);
                    card.add_kv("Name", name);
                    card.add_kv("Version", version);
                    card.add_kv("Path", resolved_path);
                    card.with_footer("Extension is now active and ready for browser sessions");
                    println!();
                    card.print();
                    println!();
                    Ok(())
                }
                Err(e) => {
                    let mut card = Card::new("BROWSER EXTENSION");
                    card.with_badge(badge_error("REGISTRATION FAILED"));
                    card.with_min_width(64);
                    card.add_kv("Target Path", path.display().to_string());
                    card.add_kv("Error", format!("{}", e));
                    card.with_footer("Ensure path contains a valid Chrome manifest.json file");
                    println!();
                    card.print();
                    println!();
                    Err(e.into())
                }
            }
        }
        Some(ExtCommands::Remove { id }) => {
            match registry.unregister(&id) {
                Ok(true) => {
                    let mut card = Card::new("BROWSER EXTENSION");
                    card.with_badge(badge_online("REMOVED"));
                    card.with_min_width(64);
                    card.add_kv("Removed ID", id);
                    card.add_line("Extension unregistered from local Tuquet registry.");
                    println!();
                    card.print();
                    println!();
                    Ok(())
                }
                Ok(false) => {
                    let mut card = Card::new("BROWSER EXTENSION");
                    card.with_badge(badge_warn("NOT FOUND"));
                    card.with_min_width(64);
                    card.add_kv("Target ID", id);
                    card.add_line("No extension with this ID was found in the registry.");
                    println!();
                    card.print();
                    println!();
                    Ok(())
                }
                Err(e) => {
                    let mut card = Card::new("BROWSER EXTENSION");
                    card.with_badge(badge_error("ERROR"));
                    card.with_min_width(64);
                    card.add_kv("Target ID", id);
                    card.add_kv("Error", format!("{}", e));
                    println!();
                    card.print();
                    println!();
                    Err(e.into())
                }
            }
        }
        Some(ExtCommands::Enable { id }) => {
            if registry.set_enabled(&id, true)? {
                let mut card = Card::new("BROWSER EXTENSION");
                card.with_badge(badge_online("ENABLED"));
                card.with_min_width(64);
                card.add_kv("ID", id);
                card.add_line("Extension enabled and will be loaded in browser sessions.");
                println!();
                card.print();
                println!();
            } else {
                let mut card = Card::new("BROWSER EXTENSION");
                card.with_badge(badge_warn("NOT FOUND"));
                card.with_min_width(64);
                card.add_kv("ID", id);
                card.add_line("Extension ID not found in registry.");
                println!();
                card.print();
                println!();
            }
            Ok(())
        }
        Some(ExtCommands::Disable { id }) => {
            if registry.set_enabled(&id, false)? {
                let mut card = Card::new("BROWSER EXTENSION");
                card.with_badge(badge_warn("DISABLED"));
                card.with_min_width(64);
                card.add_kv("ID", id);
                card.add_line("Extension disabled and will NOT be loaded automatically.");
                println!();
                card.print();
                println!();
            } else {
                let mut card = Card::new("BROWSER EXTENSION");
                card.with_badge(badge_warn("NOT FOUND"));
                card.with_min_width(64);
                card.add_kv("ID", id);
                card.add_line("Extension ID not found in registry.");
                println!();
                card.print();
                println!();
            }
            Ok(())
        }
        Some(ExtCommands::Info { id }) => {
            if let Some(ext) = registry.get(&id) {
                let mut card = Card::new("BROWSER EXTENSION");
                card.with_badge(if ext.enabled { badge_online("ACTIVE") } else { badge_warn("DISABLED") });
                card.with_min_width(64);
                card.add_kv("ID", &ext.id);
                card.add_kv("Name", &ext.name);
                card.add_kv("Version", &ext.version);
                card.add_kv("Manifest Version", format!("MV{}", ext.manifest_version));
                card.add_kv("Type", if ext.is_builtin { "Built-in System Extension" } else { "Custom Extension" });
                if let Some(ref desc) = ext.description {
                    card.add_kv("Description", desc);
                }
                card.add_kv("Filesystem Path", ext.path.display().to_string());
                card.add_kv("Directory Exists", if ext.path.exists() { "Yes" } else { "No (Missing on disk)" });
                println!();
                card.print();
                println!();
            } else {
                let mut card = Card::new("BROWSER EXTENSION");
                card.with_badge(badge_warn("NOT FOUND"));
                card.with_min_width(64);
                card.add_kv("ID", id);
                card.add_line("Extension not found in local registry. Run 'tuquet browser ext list'.");
                println!();
                card.print();
                println!();
            }
            Ok(())
        }
        Some(ExtCommands::Path { id }) => {
            if let Some(ext) = registry.get(&id) {
                println!("{}", ext.path.display());
            } else {
                eprintln!("Extension '{}' not found in registry.", id);
                std::process::exit(1);
            }
            Ok(())
        }
        Some(ExtCommands::Launch { ext, browser }) => {
            let paths_to_load = if let Some(ext_filter) = ext {
                let ids: Vec<&str> = ext_filter.split(',').map(|s| s.trim()).collect();
                let mut res = Vec::new();
                for id in ids {
                    if let Some(e) = registry.get(id) {
                        res.push(e.path.clone());
                    } else {
                        eprintln!("Warning: Extension ID '{}' not found in registry.", id);
                    }
                }
                res
            } else {
                registry.get_enabled_paths()
            };

            let load_arg = ExtensionRegistry::format_load_extension_arg(&paths_to_load);
            let mut card = Card::new("BROWSER LAUNCH CONFIG");
            card.with_badge(badge_online("CONFIGURED"));
            card.with_min_width(68);
            card.add_kv("Target Browser", browser);
            card.add_kv("Loaded Extensions", format!("{} extension(s)", paths_to_load.len()));
            card.add_line(format!("Launch command: chrome.exe --load-extension=\"{}\"", load_arg));
            card.with_footer("Ready to launch and attach to worker daemon");
            println!();
            card.print();
            println!();
            Ok(())
        }
    }
}

pub async fn setup_extension(
    browser: &str,
    extension_path: Option<std::path::PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let ext_dir = extension_path.unwrap_or_else(|| {
        std::path::PathBuf::from(crate::core::browser::worker_coordinator::resolve_cli_runner_extension_path())
    });

    let exists = ext_dir.exists();
    let badge = if exists {
        badge_online("READY")
    } else {
        badge_warn("NOT BUILT")
    };

    let mut card = Card::new("BROWSER EXTENSION");
    card.with_badge(badge);
    card.with_min_width(64);
    card.add_kv("Target Browser", browser);
    card.add_kv("Extension Path", ext_dir.display().to_string());

    if !exists {
        card.add_line("Status: Extension unpacked directory does not exist yet.");
        card.with_footer("Build extension with: pnpm --filter @automa/runner build");
    } else {
        card.add_line(format!("Launch command: chrome.exe --load-extension=\"{}\"", ext_dir.display()));
        card.with_footer("Ready to launch and attach to worker daemon");
    }
    println!();
    card.print();
    println!();
    Ok(())
}
