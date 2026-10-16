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
                    crate::commands::tenant::switch_tenant(args.get(1).copied(), fmt)
                        .await
                        .map_err(|e| format!("{}", e))?;
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
        "dev" => {
            let list = args.contains(&"--list") || args.contains(&"-l");
            let fmt = parse_format(args);
            let mut pm: Option<String> = None;
            let mut path: Option<std::path::PathBuf> = None;
            let mut positional: Vec<String> = Vec::new();
            let mut extra_args: Vec<String> = Vec::new();

            let mut i = 0;
            while i < args.len() {
                match args[i] {
                    "--" => {
                        extra_args.extend(args[i + 1..].iter().map(|s| s.to_string()));
                        break;
                    }
                    "--list" | "-l" | "--json" | "-j" | "--table" | "-t" | "--card" | "-c" => {}
                    "--pm" => {
                        if let Some(val) = args.get(i + 1) {
                            pm = Some((*val).to_string());
                            i += 1;
                        }
                    }
                    "--path" | "-P" => {
                        if let Some(val) = args.get(i + 1) {
                            path = Some(std::path::PathBuf::from(*val));
                            i += 1;
                        }
                    }
                    "--format" | "-f" => {
                        i += 1;
                    }
                    arg if !arg.starts_with('-') => {
                        positional.push(arg.to_string());
                    }
                    _ => {}
                }
                i += 1;
            }

            let project = positional.first().cloned();
            let script = positional.get(1).cloned();
            crate::commands::dev::handle(project, script, list, path, pm, extra_args, fmt).await?;
        }
        "port" => {
            let list = args.contains(&"--list") || args.contains(&"-l");
            let kill = args.contains(&"--kill") || args.contains(&"-k");
            let all = args.contains(&"--all") || args.contains(&"-a");
            let fmt = parse_format(args);
            let query = args
                .iter()
                .find(|a| !a.starts_with('-'))
                .map(|s| (*s).to_string());
            crate::commands::port::handle(query, kill, all, list, fmt).await?;
        }
        "code" => {
            let list = args.contains(&"--list") || args.contains(&"-l");
            let fmt = parse_format(args);
            let mut editor: Option<String> = None;
            let mut path: Option<std::path::PathBuf> = None;
            let mut project: Option<String> = None;

            let mut i = 0;
            while i < args.len() {
                match args[i] {
                    "--list" | "-l" | "--json" | "-j" | "--table" | "-t" | "--card" | "-c" => {}
                    "--editor" | "-e" => {
                        if let Some(val) = args.get(i + 1) {
                            editor = Some((*val).to_string());
                            i += 1;
                        }
                    }
                    "--path" | "-P" => {
                        if let Some(val) = args.get(i + 1) {
                            path = Some(std::path::PathBuf::from(*val));
                            i += 1;
                        }
                    }
                    "--format" | "-f" => {
                        i += 1;
                    }
                    arg if !arg.starts_with('-') && project.is_none() => {
                        project = Some(arg.to_string());
                    }
                    _ => {}
                }
                i += 1;
            }
            crate::commands::code::handle(project, editor, list, path, fmt).await?;
        }
        "git" => {
            let all = args.contains(&"--all") || args.contains(&"-a");
            let list = args.contains(&"--list") || args.contains(&"-l");
            let fmt = parse_format(args);
            let mut path: Option<std::path::PathBuf> = None;
            let mut positional: Vec<String> = Vec::new();

            let mut i = 0;
            while i < args.len() {
                match args[i] {
                    "--all" | "-a" | "--list" | "-l" | "--json" | "-j" | "--table" | "-t" | "--card" | "-c" => {}
                    "--path" | "-P" => {
                        if let Some(val) = args.get(i + 1) {
                            path = Some(std::path::PathBuf::from(*val));
                            i += 1;
                        }
                    }
                    "--format" | "-f" => {
                        i += 1;
                    }
                    arg if !arg.starts_with('-') => {
                        positional.push(arg.to_string());
                    }
                    _ => {}
                }
                i += 1;
            }
            let repo = positional.first().cloned();
            let action = positional.get(1).cloned();
            crate::commands::git::handle(repo, action, all, list, path, fmt).await?;
        }
        "env" => {
            let check = args.contains(&"--check") || args.contains(&"-c");
            let list = args.contains(&"--list") || args.contains(&"-l");
            let fmt = parse_format(args);
            let mut switch_profile: Option<String> = None;
            let mut path: Option<std::path::PathBuf> = None;
            let mut project: Option<String> = None;

            let mut i = 0;
            while i < args.len() {
                match args[i] {
                    "--check" | "-c" | "--list" | "-l" | "--json" | "-j" | "--table" | "-t" | "--card" => {}
                    "--switch" | "-s" => {
                        if let Some(val) = args.get(i + 1) {
                            switch_profile = Some((*val).to_string());
                            i += 1;
                        }
                    }
                    "--path" | "-P" => {
                        if let Some(val) = args.get(i + 1) {
                            path = Some(std::path::PathBuf::from(*val));
                            i += 1;
                        }
                    }
                    "--format" | "-f" => {
                        i += 1;
                    }
                    arg if !arg.starts_with('-') && project.is_none() => {
                        project = Some(arg.to_string());
                    }
                    _ => {}
                }
                i += 1;
            }
            crate::commands::env_cmd::handle(project, check, switch_profile, list, path, fmt).await?;
        }
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
