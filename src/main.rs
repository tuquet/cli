use automa_core::cli::{Cli, Commands};
use automa_core::commands;
use clap::{CommandFactory, Parser};
use std::io::IsTerminal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Trigger non-blocking 24h background update check & clean up stale update backups
    automa_core::infrastructure::updater::cleanup_stale_update_files();
    automa_core::infrastructure::updater::spawn_background_update_check();

    let result = match cli.command {
        Some(Commands::Auth { command }) => match command {
            Some(cmd) => commands::cloud::handle(cmd)
                .await
                .map_err(|e| e as Box<dyn std::error::Error>),
            None => {
                let mut cmd = Cli::command();
                if let Some(sub) = cmd.find_subcommand_mut("auth") {
                    let _ = sub.print_help();
                }
                Ok(())
            }
        },
        Some(Commands::Worker { command }) => match command {
            Some(cmd) => commands::worker::handle(cmd).await,
            None => {
                let mut cmd = Cli::command();
                if let Some(sub) = cmd.find_subcommand_mut("worker") {
                    let _ = sub.print_help();
                }
                Ok(())
            }
        },
        Some(Commands::Automation { command }) => match command {
            Some(cmd) => commands::automation::handle(cmd).await,
            None => {
                let mut cmd = Cli::command();
                if let Some(sub) = cmd.find_subcommand_mut("automation") {
                    let _ = sub.print_help();
                }
                Ok(())
            }
        },
        Some(Commands::Gateway { command }) => match command {
            Some(cmd) => commands::gateway::handle(Some(cmd)).await,
            None => {
                let mut cmd = Cli::command();
                if let Some(sub) = cmd.find_subcommand_mut("gateway") {
                    let _ = sub.print_help();
                }
                Ok(())
            }
        },
        Some(Commands::Env {
            project,
            check,
            switch,
            list,
            path,
            format,
        }) => {
            let fmt = match format.to_lowercase().as_str() {
                "json" => automa_core::ui::OutputFormat::Json,
                "card" => automa_core::ui::OutputFormat::Card,
                _ => automa_core::ui::OutputFormat::Table,
            };
            commands::env_cmd::handle(project, check, switch, list, path, fmt).await
        }
        Some(Commands::Browser { command }) => match command {
            Some(cmd) => commands::browser::handle(cmd).await,
            None => commands::browser::handle_interactive().await,
        },
        Some(Commands::Mcp) => commands::mcp::run().await,
        None => {
            let readiness = automa_core::core::readiness::quick_probe();
            if !readiness.is_pristine() {
                automa_core::ui::gate::handle_readiness_gate(&readiness).await
            } else if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
                let mut cmd = Cli::command();
                let _ = cmd.print_help();
                Ok(())
            } else {
                let mut cmd = Cli::command();
                let _ = cmd.print_help();
                std::process::exit(2);
            }
        }
    };

    if let Err(e) = result {
        if let Some(io_err) = e.downcast_ref::<std::io::Error>()
            && io_err.kind() == std::io::ErrorKind::Interrupted
        {
            std::process::exit(130);
        }
        automa_core::ui::Notify::error(e);
        std::process::exit(1);
    }

    Ok(())
}
