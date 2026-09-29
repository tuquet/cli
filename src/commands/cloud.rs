use crate::cli::CloudSubcommands;
use crate::config::AppConfig;
use crate::infrastructure::cloud_reporter::CloudReporter;

pub async fn handle(command: CloudSubcommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        CloudSubcommands::Login { url, token, name } => login(url, token, name).await,
        CloudSubcommands::Logout => logout().await,
        CloudSubcommands::Whoami => whoami().await,
    }
}

pub async fn login(
    url: Option<String>,
    token: Option<String>,
    name: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load();
    let cloud_url = url.or(config.cloud_url).unwrap_or_else(|| "https://cloud.tuquet.com".to_string());
    let enrollment_token = token.as_deref().or(config.cloud_enrollment_token.as_deref());
    match CloudReporter::login(&cloud_url, enrollment_token, name.as_deref(), &config.data_dir).await {
        Ok(creds) => {
            println!("\x1b[32m[SUCCESS] Workstation enrolled successfully!\x1b[0m");
            println!("Device ID:   {}", creds.device_id);
            println!("Device Name: {}", creds.name);
            println!("Tenant ID:   {}", creds.tenant_id.as_deref().unwrap_or("none"));
            Ok(())
        }
        Err(e) => {
            eprintln!("\x1b[31m[ERROR] Enrollment failed: {}\x1b[0m", e);
            std::process::exit(1);
        }
    }
}

pub async fn logout() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load();
    match CloudReporter::logout(&config.data_dir).await {
        Ok(true) => {
            println!("\x1b[32m[SUCCESS] Logged out and removed local cloud credentials.\x1b[0m");
            Ok(())
        }
        Ok(false) => {
            println!("No active cloud session found.");
            Ok(())
        }
        Err(e) => {
            eprintln!("\x1b[31m[ERROR] Logout failed: {}\x1b[0m", e);
            std::process::exit(1);
        }
    }
}

pub async fn whoami() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load();
    if let Some(creds) = CloudReporter::whoami(&config.data_dir).await {
        println!("Device ID:   {}", creds.device_id);
        println!("Device Name: {}", creds.name);
        println!("Tenant ID:   {}", creds.tenant_id.as_deref().unwrap_or("none"));
        println!("Cloud URL:   {}", creds.cloud_url.as_deref().unwrap_or("none"));
    } else {
        println!("Not logged in to Tuquet Cloud.");
    }
    Ok(())
}
