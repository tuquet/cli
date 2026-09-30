use crate::cli::BrowserCommands;
use crate::ui::{badge_online, badge_warn, Card};

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
    }
}
