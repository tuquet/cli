use std::path::PathBuf;

pub async fn dispatch_runner(cmd: &str, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        "status" => {
            let target_url = args.first().copied().unwrap_or("http://127.0.0.1:8765");
            crate::commands::runner::check_status(target_url, false).await?;
        }
        "start" => {
            let detach = args.contains(&"-d") || args.contains(&"--detach");
            if detach {
                println!("Starting runner daemon in background...");
            } else {
                println!("Starting runner daemon in foreground (Ctrl+C to stop)...");
            }
            crate::commands::runner::run_server(None, None, detach, None, None, false).await?;
        }
        "stop" => {
            let force = args.contains(&"-f") || args.contains(&"--force");
            crate::commands::runner::stop_daemon(force).await?;
        }
        "restart" => {
            let detach = args.contains(&"-d") || args.contains(&"--detach");
            crate::commands::runner::restart_daemon(detach).await?;
        }
        "logs" => {
            let follow = args.contains(&"-f") || args.contains(&"--follow");
            crate::commands::runner::show_logs(follow, 50).await?;
        }
        "probe" => {
            crate::commands::runner::print_probe_manifest()?;
        }
        "export-openapi" => {
            let output = args.first().map(PathBuf::from).unwrap_or_else(|| PathBuf::from("openapi.json"));
            crate::commands::runner::export_openapi(&output)?;
        }
        "setup-ext" => {
            let browser = args.first().copied().unwrap_or("chrome");
            crate::commands::browser::setup_extension(browser, None).await?;
        }
        "config" => {
            let edit = args.contains(&"--edit") || args.contains(&"-e");
            let show = args.contains(&"--show") || args.contains(&"-s");
            crate::commands::runner::manage_config(edit, show)?;
        }
        other => {
            println!("Unknown runner command '{}'. Type 'help' to see valid commands.", other);
        }
    }
    Ok(())
}
