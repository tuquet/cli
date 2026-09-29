use std::io::IsTerminal;
use automa_core::cli::{Cli, Commands};
use automa_core::commands;
use clap::{CommandFactory, Parser};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let result = match cli.command {
        Some(Commands::Automa { command }) => match command {
            Some(subcmd) => commands::automa::handle(subcmd).await,
            None => {
                if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
                    commands::shell::run(Some("automa")).await
                } else {
                    let mut cmd = Cli::command();
                    let _ = cmd.find_subcommand_mut("automa").map(|c| c.print_help());
                    std::process::exit(2);
                }
            }
        },
        Some(Commands::Runner { command }) => match command {
            Some(subcmd) => commands::runner::handle(subcmd).await,
            None => {
                if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
                    commands::shell::run(Some("runner")).await
                } else {
                    let mut cmd = Cli::command();
                    let _ = cmd.find_subcommand_mut("runner").map(|c| c.print_help());
                    std::process::exit(2);
                }
            }
        },
        Some(Commands::Cloud { command }) => match command {
            Some(subcmd) => commands::cloud::handle(subcmd).await,
            None => {
                if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
                    commands::shell::run(Some("cloud")).await
                } else {
                    let mut cmd = Cli::command();
                    let _ = cmd.find_subcommand_mut("cloud").map(|c| c.print_help());
                    std::process::exit(2);
                }
            }
        },
        Some(Commands::Browser { command }) => match command {
            Some(subcmd) => commands::browser::handle(subcmd).await,
            None => {
                if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
                    commands::shell::run(Some("browser")).await
                } else {
                    let mut cmd = Cli::command();
                    let _ = cmd.find_subcommand_mut("browser").map(|c| c.print_help());
                    std::process::exit(2);
                }
            }
        },
        Some(Commands::Shell { service }) => {
            commands::shell::run(service.as_deref()).await
        }
        None => {
            if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
                commands::shell::run(None).await
            } else {
                let mut cmd = Cli::command();
                let _ = cmd.print_help();
                std::process::exit(2);
            }
        }
    };

    if let Err(e) = result {
        eprintln!("\x1b[31m[ERROR] {}\x1b[0m", e);
        std::process::exit(1);
    }

    Ok(())
}
