use super::get_flag_value;

pub async fn dispatch_browser(cmd: &str, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        "status" => {
            let format = super::parse_format(args);
            crate::commands::browser::handle(crate::cli::BrowserCommands::Status {
                format: format.into(),
            }).await?;
        }
        "install" => {
            let force = args.contains(&"--force");
            let ver = args.iter().find(|&&a| !a.starts_with('-')).map(|s| s.to_string());
            crate::commands::browser::handle(crate::cli::BrowserCommands::Install { force, version: ver }).await?;
        }
        "search" | "releases" => {
            crate::commands::browser::handle(crate::cli::BrowserCommands::Search { remote: false }).await?;
        }
        "list" | "ls" => {
            let format = super::parse_format(args);
            crate::commands::browser::handle(crate::cli::BrowserCommands::List {
                format: format.into(),
            }).await?;
        }
        "use" => {
            if let Some(target) = args.first() {
                crate::commands::browser::handle(crate::cli::BrowserCommands::Use { version: target.to_string() }).await?;
            } else {
                eprintln!("Usage: use <version> (e.g. '148', 'v148', '144', 'lts')");
            }
        }
        "clean" => {
            crate::commands::browser::handle(crate::cli::BrowserCommands::Clean).await?;
        }
        "path" => {
            crate::commands::browser::handle(crate::cli::BrowserCommands::Path).await?;
        }
        "ext" => {
            let subcmd_str = args.first().copied();
            let ext_subcmd = match subcmd_str {
                Some("list") => Some(crate::cli::ExtCommands::List),
                Some("catalog") => {
                    let query = args.get(1).map(|s| s.to_string());
                    Some(crate::cli::ExtCommands::Catalog { query })
                }
                Some("install") => {
                    let id = args.get(1).copied().unwrap_or("automa").to_string();
                    let force = args.contains(&"-f") || args.contains(&"--force");
                    Some(crate::cli::ExtCommands::Install { id, force })
                }
                Some("add") => {
                    let path_str = args.get(1).copied().unwrap_or(".");
                    Some(crate::cli::ExtCommands::Add {
                        path: std::path::PathBuf::from(path_str),
                        id: args.get(2).map(|s| s.to_string()),
                    })
                }
                Some("remove") => {
                    let id = args.get(1).copied().unwrap_or("").to_string();
                    Some(crate::cli::ExtCommands::Remove { id })
                }
                Some("enable") => {
                    let id = args.get(1).copied().unwrap_or("").to_string();
                    Some(crate::cli::ExtCommands::Enable { id })
                }
                Some("disable") => {
                    let id = args.get(1).copied().unwrap_or("").to_string();
                    Some(crate::cli::ExtCommands::Disable { id })
                }
                Some("info") => {
                    let id = args.get(1).copied().unwrap_or("automa").to_string();
                    Some(crate::cli::ExtCommands::Info { id })
                }
                Some("path") => {
                    let id = args.get(1).copied().unwrap_or("automa").to_string();
                    Some(crate::cli::ExtCommands::Path { id })
                }
                Some("launch") => {
                    Some(crate::cli::ExtCommands::Launch {
                        ext: args.get(1).map(|s| s.to_string()),
                        browser: "chrome".to_string(),
                    })
                }
                _ => None,
            };
            crate::commands::browser::handle_ext(ext_subcmd, "chrome", None).await?;
        }
        "launch" | "start" | "open" => {
            let positional_args: Vec<&str> = args.iter().filter(|&&a| !a.starts_with('-')).copied().collect();
            let target = positional_args.first().copied();
            let url_arg = positional_args.get(1).copied();

            let (profile, url) = match (target, url_arg) {
                (Some(t), Some(u)) => (t.to_string(), Some(u.to_string())),
                (Some(t), None) => {
                    if t.starts_with("http://")
                        || t.starts_with("https://")
                        || t.starts_with("about:")
                        || t.starts_with("chrome://")
                        || t.starts_with("file://")
                    {
                        ("default".to_string(), Some(t.to_string()))
                    } else {
                        (t.to_string(), None)
                    }
                }
                _ => ("default".to_string(), None),
            };

            let cdp = args.contains(&"--cdp");
            let port = get_flag_value(args, "-p", "--port")
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(0);
            let foreground = args.contains(&"-f") || args.contains(&"--foreground");
            let headless = args.contains(&"--headless");
            let detach = args.contains(&"-d") || args.contains(&"--detach");
            let proxy = get_flag_value(args, "--proxy", "--proxy").map(|s| s.to_string());
            let mode = get_flag_value(args, "-m", "--mode").unwrap_or("extension").to_string();
            let no_cdp = args.contains(&"--no-cdp");
            let force = args.contains(&"--force");
            let skip_proxy_check = args.contains(&"--skip-proxy-check");
            crate::commands::browser::launch_browser(profile, cdp, port, foreground, headless, url, detach, proxy, mode, no_cdp, force, skip_proxy_check).await?;
        }
        "ps" | "running" | "processes" => {
            let format = super::parse_format(args);
            crate::commands::browser::handle_ps(format).await?;
        }
        "stop" | "kill" | "close" => {
            let profile = args.iter().find(|&&a| !a.starts_with('-')).map(|s| s.to_string());
            let all = args.contains(&"-a") || args.contains(&"--all");
            let force = args.contains(&"-f") || args.contains(&"--force");
            crate::commands::browser::handle_stop(profile, all, force).await?;
        }
        "verify" | "demo" | "live" => {
            let url = args.iter().find(|&&a| !a.starts_with('-')).map(|s| s.to_string());
            let headless = args.contains(&"--headless");
            let timeout = get_flag_value(args, "-t", "--timeout")
                .and_then(|t| t.parse::<u64>().ok())
                .unwrap_or(30);
            crate::commands::browser::verify_stealth_presentation(url, headless, timeout).await?;
        }
        "profile" | "profiles" => {
            let subcmd = args.first().copied();
            let sub_args = if args.len() > 1 { &args[1..] } else { &[] };
            match subcmd {
                None | Some("list") | Some("ls") => {
                    let format = super::parse_format(sub_args);
                    crate::commands::browser::handle_profile(Some(crate::cli::ProfileCommands::List { format: format.into() })).await?;
                }
                Some("create") | Some("new") | Some("add") => {
                    let name = sub_args.iter().find(|&&a| !a.starts_with('-')).copied();
                    if let Some(name) = name {
                        let seed = get_flag_value(sub_args, "-s", "--seed").and_then(|s| s.parse::<u32>().ok());
                        let os = get_flag_value(sub_args, "--os", "--os").unwrap_or("windows").to_string();
                        let cores = get_flag_value(sub_args, "--cores", "--cores").and_then(|s| s.parse::<u32>().ok());
                        let ram = get_flag_value(sub_args, "--ram", "--ram").and_then(|s| s.parse::<u32>().ok());
                        let proxy = get_flag_value(sub_args, "--proxy", "--proxy").map(|s| s.to_string());
                        let timezone = get_flag_value(sub_args, "--timezone", "--timezone").unwrap_or("Asia/Ho_Chi_Minh").to_string();
                        let locale = get_flag_value(sub_args, "--locale", "--locale").unwrap_or("vi-VN").to_string();
                        crate::commands::browser::handle_profile(Some(crate::cli::ProfileCommands::Create {
                            name: name.to_string(),
                            seed,
                            os,
                            cores,
                            ram,
                            proxy,
                            timezone,
                            locale,
                        })).await?;
                    } else {
                        println!("Usage: profile create <name> [--seed <n>] [--os <os>] [--cores <n>] [--ram <gb>] [--proxy <url>] [--timezone <tz>] [--locale <loc>]");
                    }
                }
                Some("inspect") | Some("show") | Some("info") => {
                    if let Some(id) = sub_args.first() {
                        let format = super::parse_format(sub_args);
                        crate::commands::browser::handle_profile(Some(crate::cli::ProfileCommands::Inspect { id: id.to_string(), format: format.into() })).await?;
                    } else {
                        println!("Usage: profile inspect <id>");
                    }
                }
                Some("delete") | Some("remove") | Some("rm") => {
                    if let Some(id) = sub_args.iter().find(|&&a| !a.starts_with('-')) {
                        let force = sub_args.contains(&"-f") || sub_args.contains(&"--force");
                        crate::commands::browser::handle_profile(Some(crate::cli::ProfileCommands::Delete { id: id.to_string(), force })).await?;
                    } else {
                        println!("Usage: profile delete <id> [--force]");
                    }
                }
                Some("pack") | Some("compress") | Some("archive") => {
                    if let Some(id) = sub_args.iter().find(|&&a| !a.starts_with('-')) {
                        let output = get_flag_value(sub_args, "-o", "--output").map(std::path::PathBuf::from);
                        let level = get_flag_value(sub_args, "-l", "--level").and_then(|s| s.parse::<i32>().ok()).unwrap_or(3);
                        let json = sub_args.contains(&"--json");
                        crate::commands::browser::handle_profile(Some(crate::cli::ProfileCommands::Pack { id: id.to_string(), output, level, json })).await?;
                    } else {
                        println!("Usage: profile pack <id> [-o <path>] [-l <level>] [--json]");
                    }
                }
                Some("unpack") | Some("restore") | Some("extract") => {
                    if let Some(archive) = sub_args.iter().find(|&&a| !a.starts_with('-')) {
                        let hash = get_flag_value(sub_args, "--hash", "--hash").map(|s| s.to_string());
                        let json = sub_args.contains(&"--json");
                        crate::commands::browser::handle_profile(Some(crate::cli::ProfileCommands::Unpack { archive: std::path::PathBuf::from(archive), hash, json })).await?;
                    } else {
                        println!("Usage: profile unpack <archive.tar.zst> [--hash <sha256>] [--json]");
                    }
                }
                Some("test-proxy") | Some("check-proxy") => {
                    if let Some(id) = sub_args.iter().find(|&&a| !a.starts_with('-')) {
                        let timeout = get_flag_value(sub_args, "-t", "--timeout").and_then(|s| s.parse::<u64>().ok()).unwrap_or(5);
                        let json = sub_args.contains(&"--json");
                        crate::commands::browser::handle_profile(Some(crate::cli::ProfileCommands::TestProxy { id: id.to_string(), timeout, json })).await?;
                    } else {
                        println!("Usage: profile test-proxy <id> [--timeout <sec>] [--json]");
                    }
                }
                Some("cloud") => {
                    let cloud_subcmd = sub_args.first().copied();
                    let cloud_args = if sub_args.len() > 1 { &sub_args[1..] } else { &[] };
                    match cloud_subcmd {
                        None | Some("list") | Some("ls") => {
                            let json = cloud_args.contains(&"--json");
                            crate::commands::browser::handle_profile(Some(crate::cli::ProfileCommands::Cloud {
                                command: Some(crate::cli::ProfileCloudSubcommands::List { json }),
                            })).await?;
                        }
                        Some("acquire") | Some("lock") => {
                            if let Some(id) = cloud_args.iter().find(|&&a| !a.starts_with('-')) {
                                let json = cloud_args.contains(&"--json");
                                crate::commands::browser::handle_profile(Some(crate::cli::ProfileCommands::Cloud {
                                    command: Some(crate::cli::ProfileCloudSubcommands::Acquire { id: id.to_string(), json }),
                                })).await?;
                            } else {
                                println!("Usage: profile cloud acquire <id> [--json]");
                            }
                        }
                        Some("release") | Some("unlock") => {
                            if let Some(id) = cloud_args.iter().find(|&&a| !a.starts_with('-')) {
                                let json = cloud_args.contains(&"--json");
                                crate::commands::browser::handle_profile(Some(crate::cli::ProfileCommands::Cloud {
                                    command: Some(crate::cli::ProfileCloudSubcommands::Release { id: id.to_string(), json }),
                                })).await?;
                            } else {
                                println!("Usage: profile cloud release <id> [--json]");
                            }
                        }
                        Some(other) => {
                            println!("Unknown profile cloud command '{}'. Type 'help' to see valid commands.", other);
                        }
                    }
                }
                Some(other) => {
                    println!("Unknown profile command '{}'. Type 'help' to see valid commands.", other);
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
            crate::commands::browser::manage_config(&positional, edit, show)?;
        }
        other => {
            println!("Unknown browser command '{}'. Type 'help' to see valid commands.", other);
        }
    }
    Ok(())
}
