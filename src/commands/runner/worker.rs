use crate::config::AppConfig;

pub async fn run_cloud_worker(
    cloud_profile: Option<String>,
    workflow: Option<String>,
    headless: bool,
    interval_secs: u64,
    once: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load();
    let creds = match crate::infrastructure::cloud_reporter::CloudReporter::load_credentials(&config.data_dir).await {
        Some(c) => c,
        None => {
            crate::ui::Notify::error(crate::constants::MSG_NOT_ENROLLED);
            eprintln!("Run 'specter runner enroll' to register this device.\n");
            return Err("Missing cloud device credentials".into());
        }
    };

    let client = crate::infrastructure::cloud_reporter::CloudReporter::build_http_client();
    let cloud_url = config.cloud_url.as_deref()
        .or(creds.cloud_url.as_deref())
        .unwrap_or("https://dswhacsoaxgpfnkaxnhz.supabase.co");
    let anon_key = creds.api_key.as_deref().unwrap_or(crate::infrastructure::cloud_reporter::DEFAULT_SUPABASE_ANON_KEY);

    let mut card = crate::ui::Card::new("AUTONOMOUS CLOUD FLEET WORKER");
    card.with_badge(crate::ui::badge_online("ONLINE"));
    card.with_min_width(74);
    card.add_kv("Device ID", &creds.device_id);
    card.add_kv("Device Name", &creds.name);
    card.add_kv("Cloud Endpoint", cloud_url);
    if let Some(ref p) = cloud_profile {
        card.add_kv("Target Profile", p);
    } else {
        card.add_kv("Fleet Scope", "Tenant Profile Pool");
    }
    card.add_kv("Poll Interval", format!("{} seconds", interval_secs));
    card.with_footer("Autonomous cloud mesh worker active. Press Ctrl+C to terminate cleanly.");
    println!();
    card.print();
    println!();

    if let Some(target) = cloud_profile {
        println!(">> Executing autonomous pipeline on cloud profile '{}'...", target);
        crate::infrastructure::cloud_fleet::CloudFleetOrchestrator::run_autonomous_cloud_session(
            &target,
            workflow,
            None,
            headless,
            Vec::new(),
            None,
        ).await?;
        return Ok(());
    }

    loop {
        let list_url = format!("{}/rest/v1/rpc/list_browsers", cloud_url.trim_end_matches('/'));
        let res = client.post(&list_url)
            .header("apikey", anon_key)
            .header("Authorization", format!("Bearer {}", anon_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "p_device_id": creds.device_id,
                "p_device_token": creds.device_token
            }))
            .send()
            .await;

        let now_epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let time_str = format!("{:02}:{:02}:{:02}", (now_epoch / 3600) % 24, (now_epoch / 60) % 60, now_epoch % 60);

        match res {
            Ok(r) if r.status().is_success() => {
                let browsers: Vec<serde_json::Value> = r.json().await.unwrap_or_default();
                let idle_count = browsers.iter().filter(|b| b["status"].as_str() == Some("idle")).count();
                let running_count = browsers.iter().filter(|b| b["status"].as_str() == Some("running")).count();
                println!(
                    "[{}] Fleet mesh online: {} total profiles ({} IDLE, {} RUNNING) | Node: {}",
                    time_str,
                    browsers.len(),
                    idle_count,
                    running_count,
                    creds.name
                );
            }
            Ok(r) => {
                eprintln!("[{}] Failed to query fleet: HTTP {}", time_str, r.status());
            }
            Err(e) => {
                eprintln!("[{}] Network error polling fleet: {}", time_str, e);
            }
        }

        if once {
            break;
        }

        tokio::select! {
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(interval_secs)) => {},
            _ = tokio::signal::ctrl_c() => {
                println!();
                crate::ui::Notify::shutdown(crate::constants::MSG_SHUTDOWN_SIGNAL);
                break;
            }
        }
    }

    Ok(())
}
