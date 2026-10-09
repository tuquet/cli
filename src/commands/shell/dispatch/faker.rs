use super::get_flag_value;
use crate::commands::shell::{print_scope_help, ShellScope};

pub async fn dispatch_faker(cmd: &str, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let count = get_flag_value(args, "-n", "--count")
        .and_then(|v| v.parse().ok())
        .or_else(|| {
            let mut iter = args.iter();
            while let Some(&arg) = iter.next() {
                if arg.starts_with('-') {
                    iter.next();
                } else if !arg.is_empty() && arg.chars().all(|c| c.is_ascii_digit()) {
                    return arg.parse().ok();
                }
            }
            None
        })
        .unwrap_or(1);
    let gender = get_flag_value(args, "-g", "--gender");
    let nat = get_flag_value(args, "--nat", "--nat").or(Some("VN"));
    let avatar = get_flag_value(args, "--avatar", "--avatar").or(Some("real"));
    let domain = get_flag_value(args, "-d", "--domain");
    let format = get_flag_value(args, "-f", "--format").unwrap_or("table");
    let output = get_flag_value(args, "-o", "--output").map(std::path::PathBuf::from);

    match cmd {
        "card" | "show" | "inspect" => {
            crate::commands::faker::run_card(gender, nat, avatar, domain).await?;
        }
        "generate" | "gen" | "g" => {
            crate::commands::faker::run_generate(
                count,
                gender,
                nat,
                avatar,
                domain,
                format,
                output,
            ).await?;
        }
        "config" => {
            let edit = args.contains(&"--edit") || args.contains(&"-e");
            let show = args.contains(&"--show") || args.contains(&"-s");
            crate::commands::faker::manage_config(edit, show, domain)?;
        }
        "help" => {
            print_scope_help(ShellScope::Faker);
        }
        _ => {
            let effective_count = if cmd.chars().all(|c| c.is_ascii_digit()) {
                cmd.parse().unwrap_or(1)
            } else {
                count
            };
            crate::commands::faker::run_generate(
                effective_count,
                gender,
                nat,
                avatar,
                domain,
                format,
                output,
            ).await?;
        }
    }
    Ok(())
}
