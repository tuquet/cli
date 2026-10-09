use crate::cli::ProfileCloudSubcommands;
use crate::ui::{badge_online, badge_warn, Card, Column, Table};

pub async fn handle_profile_cloud(
    command: Option<ProfileCloudSubcommands>,
) -> Result<(), Box<dyn std::error::Error>> {
    let app_config = crate::config::AppConfig::load();
    let creds = match crate::infrastructure::cloud_reporter::CloudReporter::load_credentials(&app_config.data_dir).await {
        Some(c) => c,
        None => {
            crate::ui::Notify::error(crate::constants::MSG_NOT_ENROLLED);
            eprintln!("Run 'specter login' to authenticate and pair your workstation first.\n");
            return Err("Workstation not enrolled with Specter Cloud".into());
        }
    };

    let cloud_url = creds.cloud_url.as_deref().unwrap_or("https://dswhacsoaxgpfnkaxnhz.supabase.co");
    let anon_key = creds.api_key.as_deref().unwrap_or(crate::infrastructure::cloud_reporter::DEFAULT_SUPABASE_ANON_KEY);

    let client = crate::infrastructure::cloud_reporter::CloudReporter::build_http_client();

    match command.unwrap_or(ProfileCloudSubcommands::List { json: false }) {
        ProfileCloudSubcommands::List { json } => {
            let rpc_url = format!("{}/rest/v1/rpc/list_browsers", cloud_url.trim_end_matches('/'));
            let res = client.post(&rpc_url)
                .header("apikey", anon_key)
                .header("Authorization", format!("Bearer {}", anon_key))
                .header("Content-Type", "application/json")
                .json(&serde_json::json!({
                    "p_device_id": creds.device_id,
                    "p_device_token": creds.device_token
                }))
                .send()
                .await?;

            if !res.status().is_success() {
                let err_text = res.text().await.unwrap_or_default();
                crate::ui::Notify::error(format!("Failed to fetch cloud browser profiles: {}", err_text));
                return Err(format!("Cloud RPC error: {}", err_text).into());
            }

            let browsers: Vec<serde_json::Value> = res.json().await?;

            if json {
                println!("{}", serde_json::to_string_pretty(&browsers)?);
                return Ok(());
            }

            println!();
            let mut header_card = Card::new("SPECTER CLOUD BROWSER FLEET");
            header_card.with_badge(badge_online(&format!("{} FLEET PROFILES", browsers.len())));
            header_card.with_min_width(74);
            header_card.add_kv("Tenant ID", creds.tenant_id.as_deref().unwrap_or("Personal"));
            header_card.add_kv("Cloud Endpoint", cloud_url);
            header_card.with_footer("Acquire exclusive lease with: specter browser profile cloud acquire <id>");
            header_card.print();
            println!();

            if browsers.is_empty() {
                let mut empty_card = Card::new("NO CLOUD PROFILES");
                empty_card.with_badge(badge_warn("EMPTY"));
                empty_card.with_min_width(74);
                empty_card.add_line("No browser profiles found in current cloud tenant.");
                empty_card.print();
                println!();
                return Ok(());
            }

            let columns = vec![
                Column { title: "Cloud ID".to_string(), min_width: 20, align_right: false },
                Column { title: "Name".to_string(), min_width: 20, align_right: false },
                Column { title: "Status".to_string(), min_width: 10, align_right: false },
                Column { title: "Seed (PRNG)".to_string(), min_width: 12, align_right: false },
                Column { title: "OS / Cores".to_string(), min_width: 14, align_right: false },
                Column { title: "Storage Delta".to_string(), min_width: 15, align_right: false },
                Column { title: "Lease Device".to_string(), min_width: 16, align_right: false },
            ];

            let mut table = Table::new(columns);
            for b in &browsers {
                let id_raw = b["id"].as_str().unwrap_or("-");
                let id_short = if id_raw.len() > 18 { format!("{}...", &id_raw[..15]) } else { id_raw.to_string() };
                let name = b["name"].as_str().unwrap_or("unnamed");
                let status = b["status"].as_str().unwrap_or("idle");
                let seed = b["fingerprint_seed"].as_u64().map(|s| s.to_string()).unwrap_or_else(|| "-".to_string());
                let os = b["os_platform"].as_str().unwrap_or("win");
                let cores = b["cpu_cores"].as_u64().unwrap_or(8);
                let os_cores = format!("{}/{}c", os, cores);
                let bytes = b["storage_size_bytes"].as_u64().unwrap_or(0);
                let size_str = if bytes > 1024 * 1024 {
                    format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
                } else if bytes > 1024 {
                    format!("{:.1} KB", bytes as f64 / 1024.0)
                } else {
                    format!("{} B", bytes)
                };
                let locked = b["locked_by_device_id"].as_str();
                let lease_str = match locked {
                    Some(d) if d == creds.device_id => "● This Node".to_string(),
                    Some(d) => format!("Locked ({})", &d[..8]),
                    None => "Available".to_string(),
                };

                table.add_row(vec![
                    id_short,
                    name.to_string(),
                    status.to_uppercase(),
                    seed,
                    os_cores,
                    size_str,
                    lease_str,
                ]);
            }
            table.print();
            println!();
            Ok(())
        }
        ProfileCloudSubcommands::Acquire { id, json } => {
            let rpc_url = format!("{}/rest/v1/rpc/acquire_browser", cloud_url.trim_end_matches('/'));
            let res = client.post(&rpc_url)
                .header("apikey", anon_key)
                .header("Authorization", format!("Bearer {}", anon_key))
                .header("Content-Type", "application/json")
                .json(&serde_json::json!({
                    "p_browser_id": id,
                    "p_device_id": creds.device_id,
                    "p_device_token": creds.device_token
                }))
                .send()
                .await?;

            let body: serde_json::Value = res.json().await?;
            let success = body.get("success").and_then(|v| v.as_bool()).unwrap_or(false);

            if !success {
                let err_msg = body.get("error").and_then(|v| v.as_str()).unwrap_or("Failed to acquire browser lease");
                if json {
                    println!("{}", serde_json::json!({ "success": false, "error": err_msg }));
                } else {
                    crate::ui::Notify::error(format!("Acquire failed: {}", err_msg));
                    if let Some(locked_device) = body.get("locked_by_device_id").and_then(|v| v.as_str()) {
                        eprintln!("  • Profile currently locked by device: {}", locked_device);
                    }
                    if let Some(locked_at) = body.get("locked_at").and_then(|v| v.as_str()) {
                        eprintln!("  • Locked at timestamp: {}", locked_at);
                    }
                }
                return Err(err_msg.into());
            }

            let browser = &body["browser"];
            let name = browser["name"].as_str().unwrap_or("cloud_profile");
            let seed = browser["fingerprint_seed"].as_u64().map(|s| s as u32);
            let os = browser["os_platform"].as_str().map(|s| s.to_string());
            let cores = browser["cpu_cores"].as_u64().map(|s| s as u32);
            let ram = browser["ram_gb"].as_u64().map(|s| s as u32);
            let proxy = browser["proxy"].as_str().filter(|p| !p.is_empty()).map(|s| s.to_string());
            let tz = browser["timezone"].as_str().map(|s| s.to_string());
            let loc = browser["locale"].as_str().map(|s| s.to_string());

            let base_dir = crate::core::browser::resolve_data_dir();
            let mut profile = crate::core::browser::BrowserProfile::generate(
                name, seed, os, cores, ram, proxy.clone(), tz, loc
            );
            profile.cloud_id = Some(id.clone());
            profile.storage_path = browser["storage_path"].as_str().map(|s| s.to_string());
            profile.storage_hash = browser["storage_hash"].as_str().map(|s| s.to_string());
            profile.storage_size_bytes = browser["storage_size_bytes"].as_u64();
            profile.cookies_count = browser["cookies_count"].as_u64().map(|c| c as u32);

            // Save metadata locally
            profile.save(&base_dir)?;

            // If storage_path exists locally or is accessible, restore session
            let mut restored_snapshot = false;
            if let Some(ref sp) = profile.storage_path {
                let sp_path = std::path::Path::new(sp);
                let archive_candidate = if sp_path.is_absolute() {
                    sp_path.to_path_buf()
                } else {
                    base_dir.join(sp)
                };
                if archive_candidate.exists() {
                    if let Ok(unpacked) = crate::core::browser::BrowserProfile::unpack_archive(&archive_candidate, &base_dir, profile.storage_hash.as_deref()) {
                        restored_snapshot = true;
                        let _ = unpacked.save(&base_dir);
                    }
                }
            }

            if json {
                println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                    "success": true,
                    "cloud_id": id,
                    "profile_id": profile.id,
                    "profile_name": profile.name,
                    "fingerprint_seed": profile.fingerprint_seed,
                    "restored_snapshot": restored_snapshot,
                    "lease": body
                }))?);
                return Ok(());
            }

            println!();
            let mut card = Card::new("CLOUD PROFILE LEASE ACQUIRED");
            card.with_badge(badge_online("ACQUIRED"));
            card.with_min_width(74);
            card.add_kv("Cloud Browser ID", &id);
            card.add_kv("Local Profile ID", &profile.id);
            card.add_kv("Profile Name", &profile.name);
            if let Some(s) = profile.fingerprint_seed {
                card.add_kv("PRNG Seed", format!("{} (Deterministic Hardware)", s));
            }
            card.add_kv("Hardware", format!("{} cores / {} GB RAM", profile.hardware_concurrency.unwrap_or(8), profile.device_memory_gb.unwrap_or(16)));
            if let Some(ref pxy) = proxy {
                card.add_kv("Configured Proxy", pxy);
            }
            if restored_snapshot {
                card.add_kv("Session Snapshot", "Restored (.tar.zst + LevelDB intact)");
            }
            card.with_footer(format!("Launch session with: specter browser launch {}", profile.id));
            card.print();
            println!();
            Ok(())
        }
        ProfileCloudSubcommands::Release { id, json } => {
            let base_dir = crate::core::browser::resolve_data_dir();
            // Load local profile matching id or cloud_id or name
            let profile = match crate::core::browser::BrowserProfile::load(&id, &base_dir) {
                Ok(p) => p,
                Err(_) => {
                    // Try finding by cloud_id in all local profiles
                    let all = crate::core::browser::BrowserProfile::list_all(&base_dir)?;
                    match all.into_iter().find(|p| p.cloud_id.as_deref() == Some(&id)) {
                        Some(p) => p,
                        None => {
                            let err = format!("No local profile matching ID or Cloud ID '{}'", id);
                            if json {
                                println!("{}", serde_json::json!({ "success": false, "error": err }));
                            } else {
                                crate::ui::Notify::error(format!("Profile not found: {}", err));
                            }
                            return Err(err.into());
                        }
                    }
                }
            };

            // Pack profile into .tar.zst delta
            let report = profile.pack(&base_dir, None)?;
            let cloud_target_id = profile.cloud_id.as_deref().unwrap_or(&id);
            let canonical_rel_storage = format!("profiles/{}.tar.zst", profile.id);

            let rpc_url = format!("{}/rest/v1/rpc/release_browser", cloud_url.trim_end_matches('/'));
            let res = client.post(&rpc_url)
                .header("apikey", anon_key)
                .header("Authorization", format!("Bearer {}", anon_key))
                .header("Content-Type", "application/json")
                .json(&serde_json::json!({
                    "p_browser_id": cloud_target_id,
                    "p_device_id": creds.device_id,
                    "p_device_token": creds.device_token,
                    "p_storage_path": canonical_rel_storage,
                    "p_storage_size_bytes": report.compressed_bytes,
                    "p_storage_hash": report.sha256_hash,
                    "p_metadata": {
                        "file_count": report.file_count,
                        "compression_ratio": report.compression_ratio
                    }
                }))
                .send()
                .await?;

            let body: serde_json::Value = res.json().await?;
            let success = body.get("success").and_then(|v| v.as_bool()).unwrap_or(false);

            if !success {
                let err_msg = body.get("error").and_then(|v| v.as_str()).unwrap_or("Failed to release browser lease");
                if json {
                    println!("{}", serde_json::json!({ "success": false, "error": err_msg }));
                } else {
                    crate::ui::Notify::error(format!("Release failed: {}", err_msg));
                }
                return Err(err_msg.into());
            }

            if json {
                println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                    "success": true,
                    "cloud_id": cloud_target_id,
                    "profile_id": profile.id,
                    "status": "idle",
                    "storage_path": canonical_rel_storage,
                    "compressed_bytes": report.compressed_bytes,
                    "sha256": report.sha256_hash,
                    "release": body
                }))?);
                return Ok(());
            }

            println!();
            let mut card = Card::new("CLOUD PROFILE LEASE RELEASED");
            card.with_badge(badge_online("IDLE"));
            card.with_min_width(74);
            card.add_kv("Cloud Browser ID", cloud_target_id);
            card.add_kv("Local Profile", format!("{} [{}]", profile.name, profile.id));
            card.add_kv("Delta Archive", format!("~/.specter/browser/{}", canonical_rel_storage));
            card.add_kv("Compressed Size", format!("{} bytes ({:.1}% ratio)", report.compressed_bytes, report.compression_ratio));
            card.add_kv("SHA-256 Digest", &report.sha256_hash);
            card.with_footer("Profile lease returned to cloud fleet. Ready for next automated run.");
            card.print();
            println!();
            Ok(())
        }
        ProfileCloudSubcommands::Push { id, storage_url, json } => {
            handle_profile_push(&id, storage_url.as_deref(), json).await
        }
        ProfileCloudSubcommands::Pull { id, storage_url, json } => {
            handle_profile_pull(&id, storage_url.as_deref(), json).await
        }
    }
}

pub async fn handle_profile_push(
    id: &str,
    storage_url_opt: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let app_config = crate::config::AppConfig::load();
    let base_dir = crate::core::browser::resolve_data_dir();

    // Load local profile
    let profile = match crate::core::browser::BrowserProfile::load(id, &base_dir) {
        Ok(p) => p,
        Err(_) => {
            let all = crate::core::browser::BrowserProfile::list_all(&base_dir)?;
            match all.into_iter().find(|p| p.cloud_id.as_deref() == Some(id) || p.name == id) {
                Some(p) => p,
                None => {
                    let err = format!("Local profile '{}' not found", id);
                    if json {
                        println!("{}", serde_json::json!({ "success": false, "error": err }));
                    } else {
                        crate::ui::Notify::error(&err);
                    }
                    return Err(err.into());
                }
            }
        }
    };

    // Pack profile into archive (.tar.zst) with cache sanitization
    let report = profile.pack(&base_dir, None)?;

    // Resolve Storage Hub URL
    let storage_base_url = storage_url_opt
        .map(|s| s.to_string())
        .or_else(|| std::env::var("SPECTER_STORAGE_URL").ok())
        .unwrap_or_else(|| "http://127.0.0.1:8080".to_string());
    let storage_base = storage_base_url.trim_end_matches('/');

    let creds = crate::infrastructure::cloud_reporter::CloudReporter::load_credentials(&app_config.data_dir).await;
    let device_token = creds.as_ref().map(|c| c.device_token.as_str()).unwrap_or("specter-local-token");
    let device_id = creds.as_ref().map(|c| c.device_id.as_str()).unwrap_or("specter-workstation");

    let client = crate::infrastructure::cloud_reporter::CloudReporter::build_http_client();

    // 1. Request presigned upload URL from Storage Hub
    let presigned_endpoint = format!("{}/api/profiles/upload-url", storage_base);
    let presigned_res = client.post(&presigned_endpoint)
        .header("X-Device-Token", device_token)
        .header("X-Device-Id", device_id)
        .header("X-Tenant-Id", "default")
        .json(&serde_json::json!({
            "profile_id": profile.id,
            "name": profile.name,
            "zip_size": report.compressed_bytes
        }))
        .send()
        .await?;

    if !presigned_res.status().is_success() {
        let err_body = presigned_res.text().await.unwrap_or_default();
        let err_msg = format!("Failed to request Presigned URL from Storage Hub: {}", err_body);
        if json {
            println!("{}", serde_json::json!({ "success": false, "error": err_msg }));
        } else {
            crate::ui::Notify::error(&err_msg);
        }
        return Err(err_msg.into());
    }

    let presigned_json: serde_json::Value = presigned_res.json().await?;
    let upload_url = presigned_json["upload_url"].as_str().ok_or("Missing upload_url in response")?;
    let storage_path = presigned_json["storage_path"].as_str().unwrap_or("").to_string();

    // 2. Stream upload archive directly to Cloudflare R2
    let file_bytes = std::fs::read(&report.archive_path)?;
    let put_res = client.put(upload_url)
        .header("Content-Type", "application/octet-stream")
        .body(file_bytes)
        .send()
        .await?;

    if !put_res.status().is_success() {
        let err_status = put_res.status();
        let err_msg = format!("R2 direct upload failed with status {}: {:?}", err_status, put_res.text().await.ok());
        if json {
            println!("{}", serde_json::json!({ "success": false, "error": err_msg }));
        } else {
            crate::ui::Notify::error(&err_msg);
        }
        return Err(err_msg.into());
    }

    // 3. Complete profile registration in Storage Hub D1
    let complete_endpoint = format!("{}/api/profiles/complete", storage_base);
    let _ = client.post(&complete_endpoint)
        .header("X-Device-Token", device_token)
        .header("X-Device-Id", device_id)
        .header("X-Tenant-Id", "default")
        .json(&serde_json::json!({
            "profile_id": profile.id,
            "name": profile.name,
            "storage_path": storage_path,
            "zip_size": report.compressed_bytes,
            "checksum_sha256": report.sha256_hash,
            "status": "idle"
        }))
        .send()
        .await;

    if json {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "success": true,
            "profile_id": profile.id,
            "name": profile.name,
            "storage_path": storage_path,
            "compressed_bytes": report.compressed_bytes,
            "sha256": report.sha256_hash,
            "compression_ratio": report.compression_ratio,
        }))?);
        return Ok(());
    }

    println!();
    let mut card = Card::new("SPECTER PROFILE PUSHED (CLOUDFLARE R2)");
    card.with_badge(badge_online("R2 SYNCED"));
    card.with_min_width(74);
    card.add_kv("Profile ID", &profile.id);
    card.add_kv("Profile Name", &profile.name);
    card.add_kv("Remote Key", &storage_path);
    card.add_kv("Archive Size", format!("{} bytes ({:.1}% ratio)", report.compressed_bytes, report.compression_ratio));
    card.add_kv("SHA-256 Digest", &report.sha256_hash);
    card.with_footer("Profile snapshot uploaded directly to R2 object storage with 0đ egress.");
    card.print();
    println!();

    Ok(())
}

pub async fn handle_profile_pull(
    id: &str,
    storage_url_opt: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let app_config = crate::config::AppConfig::load();
    let base_dir = crate::core::browser::resolve_data_dir();

    let storage_base_url = storage_url_opt
        .map(|s| s.to_string())
        .or_else(|| std::env::var("SPECTER_STORAGE_URL").ok())
        .unwrap_or_else(|| "http://127.0.0.1:8080".to_string());
    let storage_base = storage_base_url.trim_end_matches('/');

    let creds = crate::infrastructure::cloud_reporter::CloudReporter::load_credentials(&app_config.data_dir).await;
    let device_token = creds.as_ref().map(|c| c.device_token.as_str()).unwrap_or("specter-local-token");
    let device_id = creds.as_ref().map(|c| c.device_id.as_str()).unwrap_or("specter-workstation");

    let client = crate::infrastructure::cloud_reporter::CloudReporter::build_http_client();

    // 1. Request presigned download URL from Storage Hub
    let download_url_endpoint = format!("{}/api/profiles/{}/download-url", storage_base, id);
    let download_res = client.get(&download_url_endpoint)
        .header("X-Device-Token", device_token)
        .header("X-Device-Id", device_id)
        .header("X-Tenant-Id", "default")
        .send()
        .await?;

    if !download_res.status().is_success() {
        let err_body = download_res.text().await.unwrap_or_default();
        let err_msg = format!("Failed to request Download URL from Storage Hub: {}", err_body);
        if json {
            println!("{}", serde_json::json!({ "success": false, "error": err_msg }));
        } else {
            crate::ui::Notify::error(&err_msg);
        }
        return Err(err_msg.into());
    }

    let download_json: serde_json::Value = download_res.json().await?;
    let download_url = download_json["download_url"].as_str().ok_or("Missing download_url in response")?;
    let storage_path = download_json["storage_path"].as_str().unwrap_or("");

    // 2. Stream download archive from Cloudflare R2
    let get_res = client.get(download_url).send().await?;
    if !get_res.status().is_success() {
        let err_msg = format!("Failed to stream archive from R2: status {}", get_res.status());
        if json {
            println!("{}", serde_json::json!({ "success": false, "error": err_msg }));
        } else {
            crate::ui::Notify::error(&err_msg);
        }
        return Err(err_msg.into());
    }

    let archive_bytes = get_res.bytes().await?;
    let target_archive = base_dir.join(format!("{}.tar.zst", id));
    std::fs::write(&target_archive, &archive_bytes)?;

    // 3. Unpack into local profile directory
    let profile = crate::core::browser::BrowserProfile::unpack_archive(&target_archive, &base_dir, None)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "success": true,
            "profile_id": profile.id,
            "name": profile.name,
            "sandbox_dir": profile.get_sandbox_dir(&base_dir).display().to_string(),
            "downloaded_bytes": archive_bytes.len(),
            "storage_path": storage_path,
        }))?);
        return Ok(());
    }

    println!();
    let mut card = Card::new("SPECTER PROFILE PULLED (CLOUDFLARE R2)");
    card.with_badge(badge_online("RESTORED"));
    card.with_min_width(74);
    card.add_kv("Profile ID", &profile.id);
    card.add_kv("Profile Name", &profile.name);
    card.add_kv("Sandbox Directory", profile.get_sandbox_dir(&base_dir).display().to_string());
    card.add_kv("Downloaded Bytes", format!("{} bytes", archive_bytes.len()));
    card.with_footer(format!("Ready for stealth automation. Launch with: specter browser launch {}", profile.id));
    card.print();
    println!();

    Ok(())
}
