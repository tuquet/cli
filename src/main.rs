use std::io::IsTerminal;
use automa_core::cli::{Cli, Commands};
use automa_core::commands;
use clap::{CommandFactory, Parser};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Trigger non-blocking 24h background update check & clean up stale update backups
    automa_core::infrastructure::updater::cleanup_stale_update_files();
    automa_core::infrastructure::updater::spawn_background_update_check();

    let result = match cli.command {
        Some(Commands::Upgrade) => automa_core::infrastructure::updater::run_upgrade().await,
        Some(Commands::Status { json }) => commands::status::show_dashboard(json).await,
        Some(Commands::Doctor { fix }) => commands::doctor::run(fix).await,
        Some(Commands::Bootstrap { force }) => commands::doctor::bootstrap(force).await,
        Some(Commands::Config { service, edit, show }) => {
            handle_global_config(service.as_deref(), edit, show).await
        }
        Some(Commands::Whoami) => commands::cloud::whoami().await,
        Some(Commands::Login { url, token, name }) => commands::cloud::login(url, token, name).await,
        Some(Commands::Automa { command }) => dispatch_scoped("automa", command, commands::automa::handle).await,
        Some(Commands::Runner { command }) => dispatch_scoped("runner", command, commands::runner::handle).await,
        Some(Commands::Cloud { command }) => dispatch_scoped("cloud", command, commands::cloud::handle).await,
        Some(Commands::Browser { command }) => dispatch_scoped("browser", command, commands::browser::handle).await,
        Some(Commands::Profile { command }) => commands::browser::handle_profile(command).await,
        Some(Commands::Launch { profile, port, headless, url, detach, proxy, mode, no_cdp, skip_proxy_check }) => {
            commands::browser::launch_browser(profile, port, headless, url, detach, proxy, mode, no_cdp, skip_proxy_check).await
        }
        Some(Commands::Proxy { command }) => commands::browser::handle_proxy(command).await,
        Some(Commands::Faker { command }) => commands::faker::handle(command).await,
        Some(Commands::Bridge { command }) => commands::bridge::handle(command).await,
        Some(Commands::Shell { service }) => {
            commands::shell::run(service.as_deref()).await
        }
        Some(Commands::Mcp) => commands::mcp::run().await,
        Some(Commands::Schema { command, json }) => {
            commands::schema::handle(command, json).await
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

async fn dispatch_scoped<T, F, Fut>(
    scope: &'static str,
    subcmd: Option<T>,
    handler: F,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnOnce(T) -> Fut,
    Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error>>>,
{
    match subcmd {
        Some(cmd) => handler(cmd).await,
        None => {
            if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
                commands::shell::run(Some(scope)).await
            } else {
                let mut cmd = Cli::command();
                let _ = cmd.find_subcommand_mut(scope).map(|c| c.print_help());
                std::process::exit(2);
            }
        }
    }
}

async fn handle_global_config(
    service: Option<&str>,
    edit: bool,
    show: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let service_name = match service {
        None => {
            if edit {
                eprintln!("Please specify a service to edit: specter config <service> --edit");
                eprintln!("Available services: bridge, faker, browser, automa, runner, cloud, system");
            } else {
                automa_core::config::ConfigRegistry::render_overview_card();
            }
            return Ok(());
        }
        Some(s) => s,
    };

    match automa_core::config::ConfigRegistry::canonical_service(service_name) {
        Some("bridge") => commands::bridge::manage_config(edit, show),
        Some("faker") => commands::faker::manage_config(edit, show, None),
        Some("browser") => commands::browser::manage_config(edit, show),
        Some("automa") => commands::automa::manage_config(edit, show),
        Some("runner") => commands::runner::manage_config(edit, show),
        Some("system") => commands::cloud::manage_config(edit, show),
        _ => Err(format!(
            "Unknown service '{}'. Available services: bridge, faker, browser, automa, runner, cloud, system",
            service_name
        ).into()),
    }
}

