use automa_core::cli::{Cli, Commands};
use automa_core::commands;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Automa { command } => commands::automa::handle(command).await,
        Commands::Runner { command } => commands::runner::handle(command).await,
        Commands::Cloud { command } => commands::cloud::handle(command).await,
        Commands::Browser { command } => commands::browser::handle(command).await,
    }
}
