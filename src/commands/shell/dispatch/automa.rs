use std::path::PathBuf;
use super::get_flag_value;

pub async fn dispatch_automa(cmd: &str, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        "run" => {
            if args.is_empty() {
                println!("Usage: run <workflow.json | stored_id> [--headless] [--browser <name>] [--timeout <sec>]");
            } else {
                let wf = Some(args[0].to_string());
                let headless = args.contains(&"--headless");
                let browser = get_flag_value(args, "-b", "--browser").map(|s| s.to_string());
                let timeout = get_flag_value(args, "-t", "--timeout").and_then(|s| s.parse::<u64>().ok());

                crate::commands::automa::run_workflow(
                    wf, None, headless, browser, None, Vec::new(), timeout,
                )
                .await?;
            }
        }
        "list" => {
            let format = super::parse_format(args);
            let search = args.iter().find(|a| !a.starts_with('-')).map(|s| s.to_string());
            crate::commands::automa::list_workflows(search, false, false, format).await?;
        }
        "inspect" => {
            if args.is_empty() {
                println!("Usage: inspect <workflow.json | stored_id>");
            } else {
                crate::commands::automa::inspect_workflow(args[0])?;
            }
        }
        "import" => {
            if args.is_empty() {
                println!("Usage: import <file.json> [--id <id>] [--name <name>]");
            } else {
                let file = PathBuf::from(args[0]);
                let id = get_flag_value(args, "-i", "--id").map(|s| s.to_string());
                let name = get_flag_value(args, "-n", "--name").map(|s| s.to_string());
                crate::commands::automa::import_workflow(file, id, name, None).await?;
            }
        }
        "export" => {
            if args.is_empty() {
                println!("Usage: export <workflow_id> [output_file.json]");
            } else {
                let id = args[0].to_string();
                let output = args.get(1).map(PathBuf::from);
                crate::commands::automa::export_workflow(id, output).await?;
            }
        }
        "delete" => {
            if args.is_empty() {
                println!("Usage: delete <workflow_id> [--vault]");
            } else {
                let id = args[0].to_string();
                let from_vault = args.contains(&"--vault");
                crate::commands::automa::delete_workflow(id, from_vault).await?;
            }
        }
        "studio" => {
            crate::commands::automa::open_studio()?;
        }
        "config" => {
            let edit = args.contains(&"--edit") || args.contains(&"-e");
            let show = args.contains(&"--show") || args.contains(&"-s");
            let positional: Vec<String> = args
                .iter()
                .filter(|a| !a.starts_with('-'))
                .map(|s| s.to_string())
                .collect();
            crate::commands::automa::manage_config(&positional, edit, show)?;
        }
        other => {
            println!("Unknown automa command '{}'. Type 'help' to see valid commands.", other);
        }
    }
    Ok(())
}
