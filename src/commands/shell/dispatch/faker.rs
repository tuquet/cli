use super::get_flag_value;
use crate::cli::FakerGenerateArgs;
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
    let gender = get_flag_value(args, "-g", "--gender").map(|s| s.to_string());
    let nat = get_flag_value(args, "--nat", "--nat").map(|s| s.to_string());
    let avatar = get_flag_value(args, "--avatar", "--avatar").map(|s| s.to_string());
    let domain = get_flag_value(args, "-d", "--domain").map(|s| s.to_string());
    let format = get_flag_value(args, "-f", "--format").unwrap_or("json").to_string();
    let card = args.contains(&"-c") || args.contains(&"--card");
    let table = args.contains(&"-t") || args.contains(&"--table");
    let csv = args.contains(&"--csv");
    let output = get_flag_value(args, "-o", "--output").map(std::path::PathBuf::from);

    match cmd {
        "card" | "show" | "inspect" => {
            let fmt = super::parse_format(args);
            crate::commands::faker::run_card(
                gender.as_deref(),
                nat.as_deref(),
                avatar.as_deref(),
                domain.as_deref(),
                fmt,
            )
            .await?;
        }
        "nationalities" | "nats" | "locales" => {
            let table = args.contains(&"-t") || args.contains(&"--table");
            crate::commands::faker::run_nationalities(table)?;
        }
        "id" | "ssn" | "cccd" | "national-id" => {
            let raw = args.contains(&"-r") || args.contains(&"--raw");
            crate::commands::faker::run_id(nat.as_deref(), raw)?;
        }
        "generate" | "gen" | "g" => {
            crate::commands::faker::run_generate(FakerGenerateArgs {
                count,
                gender,
                nat,
                avatar,
                domain,
                format,
                card,
                table,
                csv,
                output,
            })
            .await?;
        }
        "config" => {
            let edit = args.contains(&"--edit") || args.contains(&"-e");
            let show = args.contains(&"--show") || args.contains(&"-s");
            let positional: Vec<String> = args
                .iter()
                .filter(|a| !a.starts_with('-'))
                .map(|s| s.to_string())
                .collect();
            crate::commands::faker::manage_config(&positional, edit, show, domain.as_deref())?;
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
            crate::commands::faker::run_generate(FakerGenerateArgs {
                count: effective_count,
                gender,
                nat,
                avatar,
                domain,
                format,
                card,
                table,
                csv,
                output,
            })
            .await?;
        }
    }
    Ok(())
}
