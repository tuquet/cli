use super::{
    dispatch_automa, dispatch_bridge, dispatch_browser, dispatch_cloud, dispatch_faker,
    dispatch_runner,
};

pub async fn dispatch_global(cmd: &str, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        "status" => {
            crate::commands::status::show_dashboard(false).await?;
        }
        "doctor" => {
            crate::commands::doctor::run(false).await?;
        }
        "whoami" => {
            crate::commands::cloud::whoami().await?;
        }
        "login" => {
            let token = args.first().map(|s| s.to_string());
            crate::commands::cloud::login(None, token, None).await?;
        }
        "logout" => {
            crate::commands::cloud::logout().await?;
        }
        "upgrade" | "update" => {
            crate::infrastructure::updater::run_upgrade().await?;
        }
        "config" => {
            let edit = args.contains(&"--edit") || args.contains(&"-e");
            let show = args.contains(&"--show") || args.contains(&"-s");
            let service_opt = args.iter().find(|a| !a.starts_with('-')).copied();
            match service_opt {
                None => {
                    if edit {
                        println!("Please specify a service to edit: config <service> --edit");
                        println!("Available: bridge, faker, browser, automa, runner, cloud, system");
                    } else {
                        crate::config::ConfigRegistry::render_overview_card();
                    }
                }
                Some("bridge" | "tunnel" | "vps") => {
                    crate::commands::bridge::manage_config(edit, show)?;
                }
                Some("faker" | "user" | "persona") => {
                    crate::commands::faker::manage_config(edit, show, None)?;
                }
                Some("browser" | "chromium" | "chrome") => {
                    crate::commands::browser::manage_config(edit, show)?;
                }
                Some("automa" | "workflow") => {
                    crate::commands::automa::manage_config(edit, show)?;
                }
                Some("runner" | "daemon") => {
                    crate::commands::runner::manage_config(edit, show)?;
                }
                Some("cloud" | "system") => {
                    crate::commands::cloud::manage_config(edit, show)?;
                }
                Some(unknown) => {
                    println!("Unknown service '{}'. Available: bridge, faker, browser, automa, runner, cloud, system", unknown);
                }
            }
        }
        "automa" => {
            if args.is_empty() {
                println!("Usage: automa <run | list | inspect | import | export | delete | studio>");
            } else {
                dispatch_automa(args[0], &args[1..]).await?;
            }
        }
        "runner" => {
            if args.is_empty() {
                println!("Usage: runner <start | stop | restart | status | logs | probe | export-openapi | setup-ext>");
            } else {
                dispatch_runner(args[0], &args[1..]).await?;
            }
        }
        "cloud" => {
            if args.is_empty() {
                println!("Usage: cloud <login | logout | whoami>");
            } else {
                dispatch_cloud(args[0], &args[1..]).await?;
            }
        }
        "browser" => {
            if args.is_empty() {
                println!("Usage: browser <status | install | clean | path | ext>");
            } else {
                dispatch_browser(args[0], &args[1..]).await?;
            }
        }
        "bridge" => {
            if args.is_empty() {
                dispatch_bridge("status", &[]).await?;
            } else {
                dispatch_bridge(args[0], &args[1..]).await?;
            }
        }
        "faker" | "user" => {
            if args.is_empty() {
                dispatch_faker("card", &[]).await?;
            } else {
                dispatch_faker(args[0], &args[1..]).await?;
            }
        }
        // Direct Global convenience commands
        "run" | "list" | "inspect" | "import" | "export" | "delete" | "studio" => {
            dispatch_automa(cmd, args).await?;
        }
        "install" | "clean" | "path" => {
            dispatch_browser(cmd, args).await?;
        }
        "enable" | "disable" => {
            dispatch_bridge(cmd, args).await?;
        }
        other => {
            println!("Unknown global command: '{}'. Type 'help' or 'use <service>' to enter a scope.", other);
        }
    }
    Ok(())
}
