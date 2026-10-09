use super::{
    dispatch_automa, dispatch_bridge, dispatch_browser, dispatch_cloud, dispatch_faker,
    dispatch_runner, parse_format,
};

pub async fn dispatch_global(cmd: &str, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        "status" => {
            let fmt = parse_format(args);
            crate::commands::status::show_dashboard(fmt).await?;
        }
        "doctor" => {
            let fmt = parse_format(args);
            crate::commands::doctor::run(false, fmt).await?;
        }
        "whoami" => {
            let fmt = parse_format(args);
            crate::commands::cloud::whoami(fmt).await.map_err(|e| format!("{}", e))?;
        }
        "login" => {
            let token = args.first().map(|s| s.to_string());
            let fmt = parse_format(args);
            crate::commands::cloud::login(None, None, None, false, None, token, None, None, None, fmt)
                .await
                .map_err(|e| format!("{}", e))?;
        }
        "logout" => {
            crate::commands::cloud::logout().await.map_err(|e| format!("{}", e))?;
        }
        "tenant" | "workspace" => {
            let sub = args.first().copied().unwrap_or("current");
            let sub_args = if args.is_empty() { &[][..] } else { &args[1..] };
            let fmt = parse_format(sub_args);

            match sub {
                "list" | "ls" => {
                    crate::commands::tenant::list_tenants(fmt)
                        .await
                        .map_err(|e| format!("{}", e))?;
                }
                "switch" | "use" | "select" => {
                    if let Some(target) = args.get(1) {
                        crate::commands::tenant::switch_tenant(target, fmt)
                            .await
                            .map_err(|e| format!("{}", e))?;
                    } else {
                        eprintln!("Usage: tenant switch <slug|uuid>");
                    }
                }
                _ => {
                    crate::commands::tenant::show_current_tenant(fmt)
                        .await
                        .map_err(|e| format!("{}", e))?;
                }
            }
        }
        "upgrade" | "update" => {
            crate::infrastructure::updater::run_upgrade().await?;
        }
        "config" => {
            let edit = args.contains(&"--edit") || args.contains(&"-e");
            let show = args.contains(&"--show") || args.contains(&"-s");
            let non_flag_args: Vec<String> = args
                .iter()
                .filter(|a| !a.starts_with('-'))
                .map(|s| s.to_string())
                .collect();

            let (service, sub_args) = if non_flag_args.is_empty() {
                (None, &[][..])
            } else {
                (Some(non_flag_args[0].as_str()), &non_flag_args[1..])
            };

            match service {
                None => {
                    if edit {
                        println!("Please specify a service to edit: config <service> --edit");
                        println!("Available: bridge, faker, browser, automa, runner, cloud, system, inbox");
                    } else {
                        crate::config::ConfigRegistry::render_overview_card();
                    }
                }
                Some(srv) => {
                    crate::config::ConfigController::handle_dispatch(srv, sub_args, edit, show)?;
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
