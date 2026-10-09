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
        Some(Commands::Status { format }) => commands::status::show_dashboard(format.resolve()).await,
        Some(Commands::Doctor { fix, format }) => commands::doctor::run(fix, format.resolve()).await,
        Some(Commands::Bootstrap { force }) => commands::doctor::bootstrap(force).await,
        Some(Commands::Config { service, args, edit, show }) => {
            handle_global_config(service.as_deref(), &args, edit, show).await
        }
        Some(Commands::Whoami { format }) => commands::cloud::whoami(format.resolve()).await.map_err(|e| e as Box<dyn std::error::Error>),
        Some(Commands::Login {
            url,
            email,
            password,
            otp,
            code,
            token,
            tenant,
            name,
            api_key,
            format,
        }) => {
            commands::cloud::login(url, email, password, otp, code, token, tenant, name, api_key, format.resolve())
                .await
                .map_err(|e| e as Box<dyn std::error::Error>)
        }
        Some(Commands::Logout) => commands::cloud::logout().await.map_err(|e| e as Box<dyn std::error::Error>),
        Some(Commands::Tenant { command }) => commands::tenant::handle(command).await.map_err(|e| e as Box<dyn std::error::Error>),
        Some(Commands::Automa { command }) => dispatch_scoped("automa", command, commands::automa::handle).await,
        Some(Commands::Runner { command }) => dispatch_scoped("runner", command, commands::runner::handle).await,
        Some(Commands::Cloud { command }) => dispatch_scoped("cloud", command, |cmd| async move {
            commands::cloud::handle(cmd).await.map_err(|e| e as Box<dyn std::error::Error>)
        }).await,
        Some(Commands::Browser { command }) => dispatch_scoped("browser", command, commands::browser::handle).await,
        Some(Commands::Proxy { command }) => commands::browser::handle_proxy(command).await,
        Some(Commands::Faker { command, args }) => commands::faker::handle(command, args).await,
        Some(Commands::Bridge { command }) => commands::bridge::handle(command).await,
        Some(Commands::Inbox { command }) => dispatch_scoped("inbox", command, commands::inbox::handle).await,
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
        automa_core::ui::Notify::error(e);
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
    args: &[String],
    edit: bool,
    show: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let service_name = match service {
        None => {
            if edit {
                eprintln!("Please specify a service to edit: specter config <service> --edit");
                eprintln!("Available services: bridge, faker, browser, automa, runner, cloud, system, inbox");
            } else {
                automa_core::config::ConfigRegistry::render_overview_card();
            }
            return Ok(());
        }
        Some(s) => s,
    };

    automa_core::config::ConfigController::handle_dispatch(service_name, args, edit, show)
}

