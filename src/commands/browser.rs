use crate::cli::BrowserCommands;

pub async fn handle(command: BrowserCommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        BrowserCommands::Install { force, revision } => {
            match crate::core::browser::resolver::download_chromium_runtime(force, revision.as_deref()).await {
                Ok(path) => {
                    println!("\x1b[32m[SUCCESS] Dedicated Open-Source Chromium runtime ready at: {}\x1b[0m", path);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("\x1b[31m[ERROR] Failed to install Chromium runtime: {}\x1b[0m", e);
                    std::process::exit(1);
                }
            }
        }
        BrowserCommands::Status => {
            let status = crate::core::browser::resolver::get_runtime_status();
            println!("============================================================");
            println!(" Tuquet Ecosystem - Open-Source Chromium Runtime Status");
            println!("============================================================");
            println!(" Engine:          Chromium (Pure Open Source - BSD 3-Clause)");
            println!(" Platform:        {}", status.platform);
            println!(" Status:          {}", if status.installed { "\x1b[32mINSTALLED\x1b[0m" } else { "\x1b[33mNOT INSTALLED\x1b[0m" });
            println!(" Version/Rev:     {}", status.pinned_version);
            println!(" Executable Path: {}", status.executable_path);
            println!(" Directory:       {}", status.directory);
            if let Some(mb) = status.size_mb {
                println!(" Disk Usage:      {:.1} MB", mb);
            }
            println!("============================================================");
            if !status.installed {
                println!("👉 Run 'tuquet browser install' to download and setup.");
            }
            Ok(())
        }
        BrowserCommands::Clean => {
            match crate::core::browser::resolver::clean_runtime() {
                Ok(_) => {
                    println!("\x1b[32m[SUCCESS] Cleaned browser runtime directory.\x1b[0m");
                    Ok(())
                }
                Err(e) => {
                    eprintln!("\x1b[31m[ERROR] Failed to clean runtime: {}\x1b[0m", e);
                    std::process::exit(1);
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
                    eprintln!("\x1b[31m[ERROR] {}\x1b[0m", e);
                    std::process::exit(1);
                }
            }
        }
    }
}
