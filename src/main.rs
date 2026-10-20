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
        Some(Commands::Dev {
            project,
            script,
            list,
            path,
            pm,
            extra_args,
            format,
        }) => {
            commands::dev::handle(
                project,
                script,
                list,
                path,
                pm,
                extra_args,
                format.resolve(),
            )
            .await
        }
        Some(Commands::Port {
            query,
            kill,
            all,
            list,
            format,
        }) => commands::port::handle(query, kill, all, list, format.resolve()).await,
        Some(Commands::Code {
            project,
            editor,
            list,
            path,
            format,
        }) => commands::code::handle(project, editor, list, path, format.resolve()).await,
        Some(Commands::Git {
            repo,
            action,
            all,
            list,
            path,
            format,
        }) => commands::git::handle(repo, action, all, list, path, format.resolve()).await,
        Some(Commands::Env {
            project,
            check,
            switch_profile,
            list,
            path,
            format,
        }) => {
            commands::env_cmd::handle(project, check, switch_profile, list, path, format.resolve())
                .await
        }
        Some(Commands::Upgrade) => automa_core::infrastructure::updater::run_upgrade().await,
        Some(Commands::Status { format }) => {
            commands::status::show_dashboard(format.resolve()).await
        }
        Some(Commands::Doctor { fix, format }) => {
            commands::doctor::run(fix, format.resolve()).await
        }
        Some(Commands::Bootstrap { force }) => commands::doctor::bootstrap(force).await,
        Some(Commands::Config {
            service,
            args,
            edit,
            show,
        }) => handle_global_config(service.as_deref(), &args, edit, show).await,
        Some(Commands::Whoami { format }) => commands::cloud::whoami(format.resolve())
            .await
            .map_err(|e| e as Box<dyn std::error::Error>),
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
        }) => commands::cloud::login(
            url,
            email,
            password,
            otp,
            code,
            token,
            tenant,
            name,
            api_key,
            format.resolve(),
        )
        .await
        .map_err(|e| e as Box<dyn std::error::Error>),
        Some(Commands::Logout) => commands::cloud::logout()
            .await
            .map_err(|e| e as Box<dyn std::error::Error>),
        Some(Commands::Tenant { command }) => commands::tenant::handle(command)
            .await
            .map_err(|e| e as Box<dyn std::error::Error>),
        Some(Commands::Automa { command }) => match command {
            Some(cmd) => commands::automa::handle(cmd).await,
            None => commands::automa::handle_interactive().await,
        },
        Some(Commands::Runner { command }) => match command {
            Some(cmd) => commands::runner::handle(cmd).await,
            None => commands::runner::handle_interactive().await,
        },
        Some(Commands::Cloud { command }) => match command {
            Some(cmd) => commands::cloud::handle(cmd)
                .await
                .map_err(|e| e as Box<dyn std::error::Error>),
            None => commands::cloud::handle_interactive().await,
        },
        Some(Commands::Browser { command }) => match command {
            Some(cmd) => commands::browser::handle(cmd).await,
            None => commands::browser::handle_interactive().await,
        },
        Some(Commands::Proxy { command }) => commands::browser::handle_proxy(command).await,
        Some(Commands::Faker { command, args }) => commands::faker::handle(command, args).await,
        Some(Commands::Bridge { command }) => commands::bridge::handle(command).await,
        Some(Commands::Inbox { command }) => match command {
            Some(cmd) => commands::inbox::handle(cmd).await,
            None => commands::inbox::handle_interactive().await,
        },
        Some(Commands::Shell { service }) => commands::shell::run(service.as_deref()).await,
        Some(Commands::Mcp) => commands::mcp::run().await,
        Some(Commands::Schema { command, json }) => commands::schema::handle(command, json).await,
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

async fn handle_global_config(
    service: Option<&str>,
    args: &[String],
    edit: bool,
    show: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let service_name = match service {
        None => {
            if edit {
                eprintln!("Please specify a service to edit: tq config <service> --edit");
                eprintln!(
                    "Available services: bridge, faker, browser, automa, runner, cloud, system, inbox"
                );
            } else if !show && automa_core::ui::is_interactive_tty() {
                return automa_core::config::ConfigController::interactive_global_config();
            } else {
                automa_core::config::ConfigRegistry::render_overview_card();
            }
            return Ok(());
        }
        Some(s) => s,
    };

    automa_core::config::ConfigController::handle_dispatch(service_name, args, edit, show)
}
