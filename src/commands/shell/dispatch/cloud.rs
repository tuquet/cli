pub async fn dispatch_cloud(cmd: &str, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        "login" => {
            let token = args.first().map(|s| s.to_string());
            crate::commands::cloud::login(None, token, None).await?;
        }
        "logout" => {
            crate::commands::cloud::logout().await?;
        }
        "whoami" => {
            crate::commands::cloud::whoami().await?;
        }
        "config" => {
            let edit = args.contains(&"--edit") || args.contains(&"-e");
            let show = args.contains(&"--show") || args.contains(&"-s");
            crate::commands::cloud::manage_config(edit, show)?;
        }
        other => {
            println!("Unknown cloud command '{}'. Type 'help' to see valid commands.", other);
        }
    }
    Ok(())
}
