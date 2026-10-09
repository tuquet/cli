use crate::cli::BrowserCommands;
use crate::ui::{badge_online, badge_warn, Card, Column, Table, create_tabular_card, TabularRow};

pub async fn handle_runtime(command: BrowserCommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        BrowserCommands::Install { force, version } => {
            match crate::core::browser::resolver::download_stealth_runtime(force, version.as_deref()).await {
                Ok(path) => {
                    let active_ver = crate::core::browser::resolver::get_active_version();
                    let mut card = Card::new("ANTIDETECT CHROMIUM RUNTIME");
                    card.with_badge(badge_online("INSTALLED"));
                    card.with_min_width(68);
                    card.add_kv("Engine", "Chromium C++ Antidetect Engine (Blink/V8 Patch)");
                    card.add_kv("Active Version", format!("v{}", active_ver));
                    card.add_kv("Executable", path);
                    card.with_footer("Antidetect runtime is ready for deterministic stealth automation");
                    println!();
                    card.print();
                    println!();
                    Ok(())
                }
                Err(e) => Err(format!("Failed to install Antidetect Chromium: {}", e).into()),
            }
        }
        BrowserCommands::Search { remote } => {
            let manifest = crate::core::browser::manifest::fetch_manifest(remote).await?;
            let installed = crate::core::browser::resolver::list_installed_runtimes();
            let active_ver = crate::core::browser::resolver::get_active_version();

            println!();
            let mut header_card = Card::new("UPSTREAM ANTIDETECT CHROMIUM RELEASES");
            header_card.with_badge(badge_online("MANIFEST"));
            header_card.with_min_width(74);
            header_card.add_kv("Upstream", "adryfish/fingerprint-chromium");
            header_card.add_kv("Golden LTS", manifest.channels.get("lts").map(|s| s.as_str()).unwrap_or("148.0.7778.215"));
            header_card.add_kv("Active Version", format!("v{}", active_ver));
            header_card.add_kv("Last Sync", &manifest.updated_at);
            header_card.with_footer("Run 'specter browser install <version>' to install or 'use <version>' to switch");
            header_card.print();
            println!();

            let columns = vec![
                Column { title: "Version".to_string(), min_width: 16, align_right: false },
                Column { title: "Channel".to_string(), min_width: 10, align_right: false },
                Column { title: "Status".to_string(), min_width: 14, align_right: false },
                Column { title: "Installed".to_string(), min_width: 12, align_right: false },
                Column { title: "Notes".to_string(), min_width: 32, align_right: false },
            ];

            let mut table = Table::new(columns);
            for rel in &manifest.releases {
                let status_str = match rel.status.as_str() {
                    "recommended" => "★ GOLDEN LTS",
                    "buggy" => "⚠ BUGGY",
                    "unverified" => "? UNVERIFIED",
                    "deprecated" => "○ ARCHIVE",
                    _ => &rel.status,
                };

                let installed_status = if installed.iter().any(|r| r.version == rel.version && r.is_active) {
                    "★ ACTIVE".to_string()
                } else if installed.iter().any(|r| r.version == rel.version) {
                    "● INSTALLED".to_string()
                } else {
                    "-".to_string()
                };

                table.add_row(vec![
                    rel.version.clone(),
                    rel.channel.clone(),
                    status_str.to_string(),
                    installed_status,
                    rel.notes.clone(),
                ]);
            }
            println!("{}", table.render());
            Ok(())
        }
        BrowserCommands::List { format } => {
            let runtimes = crate::core::browser::resolver::list_installed_runtimes();
            let format = format.resolve();

            if format.is_json() {
                let json_runtimes: Vec<serde_json::Value> = runtimes
                    .iter()
                    .map(|r| {
                        serde_json::json!({
                            "version": r.version,
                            "channel": r.status_badge,
                            "size_mb": r.size_mb,
                            "active": r.is_active,
                            "executable_path": r.path
                        })
                    })
                    .collect();
                println!("{}", serde_json::to_string_pretty(&json_runtimes)?);
                return Ok(());
            }

            println!();
            let mut card = Card::new("INSTALLED ANTIDETECT BROWSER VERSIONS");
            card.with_badge(badge_online("INSPECTED"));
            card.with_min_width(74);
            card.with_footer("Use 'specter browser use <version>' to switch active version (e.g. '148', '144', 'lts')");

            if runtimes.is_empty() {
                card.add_line("No Antidetect Chromium runtimes found in ~/.specter/browser/runtimes/.");
                card.add_line("Run 'specter browser install' to provision the recommended Golden LTS v148.");
                card.print();
                println!();
                return Ok(());
            }

            card.print();
            println!();

            let columns = vec![
                Column { title: "Version".to_string(), min_width: 16, align_right: false },
                Column { title: "Channel / Tag".to_string(), min_width: 16, align_right: false },
                Column { title: "Disk Footprint".to_string(), min_width: 14, align_right: false },
                Column { title: "Status".to_string(), min_width: 14, align_right: false },
                Column { title: "Executable Path".to_string(), min_width: 20, align_right: false },
            ];

            let mut table = Table::new(columns);
            for r in runtimes {
                let status_str = if r.is_active {
                    "● ACTIVE".to_string()
                } else {
                    "○ INSTALLED".to_string()
                };
                let size_str = r.size_mb.map(|s| format!("{:.1} MB", s)).unwrap_or("-".to_string());

                table.add_row(vec![
                    r.version,
                    r.status_badge,
                    size_str,
                    status_str,
                    r.path,
                ]);
            }
            println!("{}", table.render());
            Ok(())
        }
        BrowserCommands::Use { version } => {
            match crate::core::browser::resolver::set_active_version(&version) {
                Ok(info) => {
                    let mut card = Card::new("ACTIVE ANTIDETECT BROWSER");
                    card.with_badge(badge_online("UPDATED"));
                    card.with_min_width(68);
                    card.add_kv("Active Version", format!("v{} ({})", info.version, info.status_badge));
                    card.add_kv("Executable Path", info.path);
                    card.add_kv("Config File", crate::core::browser::resolver::get_browser_config_path().to_string_lossy().to_string());
                    card.with_footer("All future automation tasks will bind to this browser version");
                    println!();
                    card.print();
                    println!();
                    Ok(())
                }
                Err(e) => {
                    crate::ui::Notify::error(&e);
                    Err(e.to_string().into())
                }
            }
        }
        BrowserCommands::Status { format } => {
            let status = crate::core::browser::resolver::get_runtime_status();
            let active_ver = crate::core::browser::resolver::get_active_version();
            let profile_dir = crate::config::canonical_specter_dir().join("browser").join("profiles");
            let profile_count = std::fs::read_dir(&profile_dir)
                .map(|entries| entries.flatten().filter(|e| e.path().is_dir()).count())
                .unwrap_or(0);
            let format = format.resolve();

            if format.is_json() {
                let payload = serde_json::json!({
                    "installed": status.installed,
                    "active_version": active_ver,
                    "pinned_version": status.pinned_version,
                    "platform": status.platform,
                    "executable_path": status.executable_path,
                    "size_mb": status.size_mb,
                    "profiles_count": profile_count,
                    "sandbox_root": "~/.specter/browser/profiles"
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
                return Ok(());
            }

            let badge = if status.installed {
                badge_online("INSTALLED")
            } else {
                badge_warn("NOT INSTALLED")
            };

            let footprint_str = status
                .size_mb
                .map(|mb| format!("{:.1} MB Physical Disk", mb))
                .unwrap_or_else(|| "Physical Disk".to_string());

            let exec_clean = if status.executable_path.contains(".specter") {
                format!(
                    "~/.specter{}",
                    status
                        .executable_path
                        .split(".specter")
                        .nth(1)
                        .unwrap_or("")
                        .replace('\\', "/")
                )
            } else {
                status.executable_path.clone()
            };


            let rows = vec![
                TabularRow::new(
                    "Engine Core",
                    format!("v{}", active_ver),
                    format!("Blink/V8 Stealth ({})", status.platform),
                    if status.installed { badge_online("READY") } else { badge_warn("MISSING") },
                ),
                TabularRow::new(
                    "Runtime Exec",
                    exec_clean,
                    footprint_str,
                    if status.installed { badge_online("READY") } else { badge_warn("MISSING") },
                ),
                TabularRow::new(
                    "Sandbox Root",
                    "~/.specter/browser/profiles",
                    format!("{} Profile Sandbox(es)", profile_count),
                    badge_online("READY"),
                ),
            ];

            let card = create_tabular_card(
                "ANTIDETECT BROWSER ENGINE",
                Some(badge),
                ["COMPONENT", "VERSION / PATH", "ROLE / DETAILS", "STATUS"],
                &rows,
                Some(if !status.installed {
                    "Run 'specter browser install' to download Golden LTS v148"
                } else {
                    "Engine ready. Run 'specter browser list' or 'use <version>' to manage."
                }),
                72,
            );
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
                    card.with_footer("Run 'specter browser install' when you need to reinstall");
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
        BrowserCommands::Config { edit, show, args } => manage_config(&args, edit, show),
        _ => Err("Invalid runtime command".into()),
    }
}

pub fn manage_config(args: &[String], edit: bool, show: bool) -> Result<(), Box<dyn std::error::Error>> {
    crate::config::ConfigController::handle_dispatch("browser", args, edit, show)
}
