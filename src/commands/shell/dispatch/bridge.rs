use super::get_flag_value;
use crate::commands::shell::{print_scope_help, ShellScope};

pub async fn dispatch_bridge(cmd: &str, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        "status" => {
            crate::commands::bridge::show_status().await?;
        }
        "start" => {
            let start_http = args.contains(&"--http");
            let enable_ssh = args.contains(&"--ssh");
            let foreground = args.contains(&"-f") || args.contains(&"--foreground");
            let tag_opt = get_flag_value(args, "-t", "--tag");
            let server_opt = {
                let mut iter = args.iter();
                let mut srv = None;
                while let Some(&arg) = iter.next() {
                    if arg.starts_with('-') {
                        iter.next();
                    } else {
                        srv = Some(arg);
                        break;
                    }
                }
                srv
            };
            crate::commands::bridge::start_bridge(crate::commands::bridge::BridgeStartOptions {
                server: server_opt,
                tag: tag_opt,
                http: start_http,
                ssh: enable_ssh,
                foreground,
            }).await?;
        }
        "stop" => {
            let server_opt = args.iter().find(|a| !a.starts_with('-')).copied();
            crate::commands::bridge::stop_bridge(server_opt).await?;
        }
        "enable" => {
            let server_opt = args.iter().find(|a| !a.starts_with('-')).copied();
            if let Some(srv) = server_opt {
                crate::commands::bridge::toggle_server(srv, true)?;
            } else {
                eprintln!("Usage: enable <server_id>");
            }
        }
        "disable" => {
            let server_opt = args.iter().find(|a| !a.starts_with('-')).copied();
            if let Some(srv) = server_opt {
                crate::commands::bridge::toggle_server(srv, false)?;
            } else {
                eprintln!("Usage: disable <server_id>");
            }
        }
        "check" => {
            crate::commands::bridge::check_config()?;
        }
        "config" => {
            let edit = args.contains(&"--edit") || args.contains(&"-e");
            let show = args.contains(&"--show") || args.contains(&"-s");
            crate::commands::bridge::manage_config(edit, show)?;
        }
        "help" => {
            print_scope_help(ShellScope::Bridge);
        }
        other => {
            println!("Unknown bridge command '{}'. Type 'help' to see valid commands.", other);
        }
    }
    Ok(())
}
