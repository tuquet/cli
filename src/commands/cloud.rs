use crate::cli::CloudSubcommands;
use crate::config::AppConfig;
use crate::infrastructure::cloud_reporter::CloudReporter;
use crate::ui::{badge_offline, badge_online, Card};

pub async fn handle(command: CloudSubcommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        CloudSubcommands::Login { url, token, name } => login(url, token, name).await,
        CloudSubcommands::Logout => logout().await,
        CloudSubcommands::Whoami => whoami().await,
        CloudSubcommands::Config { edit, show } => manage_config(edit, show),
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
            let mut card = Card::new("CLOUD");
            card.with_badge(badge_online("ENROLLED"));
            card.with_min_width(64);
            card.add_kv("Device ID", &creds.device_id);
            card.add_kv("Device Name", &creds.name);
            card.add_kv("Tenant ID", creds.tenant_id.as_deref().unwrap_or("Personal Workspace"));
            card.add_kv("Endpoint", creds.cloud_url.as_deref().unwrap_or(&cloud_url));
            card.with_footer("Workstation successfully paired with Tuquet Cloud fleet");
            println!();
            card.print();
            println!();
            Ok(())
        }
        Err(e) => {
            Err(format!("Enrollment failed: {}", e).into())
        }
    }
}

pub async fn logout() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load();
    match CloudReporter::logout(&config.data_dir).await {
        Ok(true) => {
            let mut card = Card::new("CLOUD");
            card.with_badge(badge_offline("LOGGED OUT"));
            card.with_min_width(64);
            card.add_line("Removed local cloud pairing credentials and session identity.");
            card.with_footer("Run 'specter login' to enroll again with Tuquet Cloud");
            println!();
            card.print();
            println!();
            Ok(())
        }
        Ok(false) => {
            let mut card = Card::new("CLOUD");
            card.with_badge(badge_offline("DISCONNECTED"));
            card.with_min_width(64);
            card.add_line("No active cloud session found on this machine.");
            println!();
            card.print();
            println!();
            Ok(())
        }
        Err(e) => {
            Err(format!("Logout failed: {}", e).into())
        }
    }
}

pub async fn whoami() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load();
    if let Some(creds) = CloudReporter::whoami(&config.data_dir).await {
        let is_prod = creds.cloud_url.as_deref().map(|u| u.contains("dswhacsoaxgpfnkaxnhz") || u.contains("supabase")).unwrap_or(false);
        let badge_text = if is_prod { "ENROLLED (PROD)" } else { "ENROLLED" };
        let mut card = Card::new("CLOUD");
        card.with_badge(badge_online(badge_text));
        card.with_min_width(64);
        card.add_kv("Device ID", &creds.device_id);
        card.add_kv("Device Name", &creds.name);
        card.add_kv("Tenant ID", creds.tenant_id.as_deref().unwrap_or("Personal Workspace"));
        card.add_kv("Endpoint", creds.cloud_url.as_deref().unwrap_or("https://dswhacsoaxgpfnkaxnhz.supabase.co"));
        println!();
        card.print();
        println!();
    } else {
        let mut card = Card::new("CLOUD");
        card.with_badge(badge_offline("DISCONNECTED"));
        card.with_min_width(64);
        card.add_line("Workstation not enrolled with cloud fleet.");
        card.with_footer("Run 'specter login' to authenticate with Tuquet Cloud");
        println!();
        card.print();
        println!();
    }
    Ok(())
}

pub fn manage_config(edit: bool, show: bool) -> Result<(), Box<dyn std::error::Error>> {
    let path = crate::config::SystemConfig::config_path();

    if edit {
        crate::config::ConfigRegistry::open_in_editor(&path)?;
    } else if show {
        let config = crate::config::SystemConfig::load();
        println!();
        let mut card = Card::new("SYSTEM & CLOUD CONFIGURATION");
        card.with_badge(badge_online("SSOT READY"));
        card.with_min_width(70);
        card.add_kv("Config File", path.display().to_string());
        card.add_kv("Machine Name", &config.machine_name);
        card.add_kv("Cloud Endpoint", &config.cloud_url);
        card.add_kv("Update Channel", &config.update_channel);
        card.add_kv("Environment", &config.environment);
        card.add_kv("Auto Update Check", if config.auto_check_update { "enabled" } else { "disabled" });
        card.with_footer("Tip: edit with 'tuquet cloud config --edit'");
        card.print();
        println!();
    } else {
        println!("{}", path.display());
    }

    Ok(())
}

