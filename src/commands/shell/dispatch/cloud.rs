use super::parse_format;

pub async fn dispatch_cloud(cmd: &str, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
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
        "whoami" => {
            let fmt = parse_format(args);
            crate::commands::cloud::whoami(fmt).await.map_err(|e| format!("{}", e))?;
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
        "config" => {
            let edit = args.contains(&"--edit") || args.contains(&"-e");
            let show = args.contains(&"--show") || args.contains(&"-s");
            let positional: Vec<String> = args
                .iter()
                .filter(|a| !a.starts_with('-'))
                .map(|s| s.to_string())
                .collect();
            crate::commands::cloud::manage_config(&positional, edit, show)
                .map_err(|e| format!("{}", e))?;
        }
        other => {
            println!("Unknown cloud command '{}'. Type 'help' to see valid commands.", other);
        }
    }
    Ok(())
}
