use crate::cli::ProfileCommands;
use crate::ui::{badge_error, badge_online, badge_warn, Card, Column, Table};

pub async fn handle_profile(
    command: Option<ProfileCommands>,
) -> Result<(), Box<dyn std::error::Error>> {
    let base_dir = crate::core::browser::resolve_data_dir();

    match command {
        None => {
            let profiles = crate::core::browser::BrowserProfile::list_all(&base_dir)?;
            let serialized = profiles
                .iter()
                .map(|p| {
                    let bytes = p.calculate_disk_size(&base_dir);
                    serde_json::json!({
                        "id": p.id,
                        "name": p.name,
                        "seed": p.fingerprint_seed,
                        "os": p.os_platform,
                        "browser": p.browser_brand,
                        "cores": p.hardware_concurrency,
                        "ram_gb": p.device_memory_gb,
                        "proxy": p.proxy,
                        "disk_bytes": bytes,
                        "sandbox_path": p.get_sandbox_dir(&base_dir).display().to_string(),
                    })
                })
                .collect::<Vec<_>>();
            println!("{}", serde_json::to_string_pretty(&serialized)?);
            Ok(())
        }
        Some(ProfileCommands::List { format }) => {
            let profiles = crate::core::browser::BrowserProfile::list_all(&base_dir)?;
            if format.resolve().is_json() {
                let serialized = profiles
                    .iter()
                    .map(|p| {
                        let bytes = p.calculate_disk_size(&base_dir);
                        serde_json::json!({
                            "id": p.id,
                            "name": p.name,
                            "seed": p.fingerprint_seed,
                            "os": p.os_platform,
                            "browser": p.browser_brand,
                            "cores": p.hardware_concurrency,
                            "ram_gb": p.device_memory_gb,
                            "proxy": p.proxy,
                            "disk_bytes": bytes,
                            "sandbox_path": p.get_sandbox_dir(&base_dir).display().to_string(),
                        })
                    })
                    .collect::<Vec<_>>();
                println!("{}", serde_json::to_string_pretty(&serialized)?);
                return Ok(());
            }

            println!();
            let mut header_card = Card::new("ANTIDETECT BROWSER PROFILES");
            header_card.with_badge(badge_online(&format!("{} REGISTERED", profiles.len())));
            header_card.with_min_width(74);
            header_card.add_kv("Storage SSOT", base_dir.join("profiles").display().to_string());
            header_card.with_footer("Run 'specter browser launch <id>' to start or 'profile create <name>' to add");
            header_card.print();
            println!();

            if profiles.is_empty() {
                let mut empty_card = Card::new("NO PROFILES FOUND");
                empty_card.with_badge(badge_warn("EMPTY"));
                empty_card.with_min_width(74);
                empty_card.add_line("No browser profiles found in ~/.specter/browser/profiles/.");
                empty_card.add_line("Create your first profile with:");
                empty_card.add_line("  specter browser profile create 'My-Profile-1'");
                empty_card.print();
                println!();
                return Ok(());
            }

            let columns = vec![
                Column { title: "Profile ID".to_string(), min_width: 22, align_right: false },
                Column { title: "Name".to_string(), min_width: 18, align_right: false },
                Column { title: "PRNG Seed".to_string(), min_width: 12, align_right: false },
                Column { title: "OS / Brand".to_string(), min_width: 14, align_right: false },
                Column { title: "Cores / RAM".to_string(), min_width: 13, align_right: false },
                Column { title: "Disk Footprint".to_string(), min_width: 14, align_right: false },
                Column { title: "Proxy".to_string(), min_width: 16, align_right: false },
            ];

            let mut table = Table::new(columns);
            for p in &profiles {
                let bytes = p.calculate_disk_size(&base_dir);
                let size_str = if bytes > 1024 * 1024 {
                    format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
                } else if bytes > 1024 {
                    format!("{:.1} KB", bytes as f64 / 1024.0)
                } else {
                    format!("{} B", bytes)
                };

                let seed_str = p.fingerprint_seed.map(|s| s.to_string()).unwrap_or_else(|| "-".to_string());
                let os_brand = format!("{}/{}", p.os_platform.as_deref().unwrap_or("win"), p.browser_brand.as_deref().unwrap_or("Chrome"));
                let hw = format!("{}c/{}G", p.hardware_concurrency.unwrap_or(8), p.device_memory_gb.unwrap_or(16));
                let proxy_str = p.proxy.as_deref().unwrap_or("-");

                table.add_row(vec![
                    p.id.clone(),
                    p.name.clone(),
                    seed_str,
                    os_brand,
                    hw,
                    size_str,
                    proxy_str.to_string(),
                ]);
            }
            println!("{}", table.render());
            Ok(())
        }
        Some(ProfileCommands::Create { name, seed, os, cores, ram, proxy, timezone, locale }) => {
            let profile = crate::core::browser::BrowserProfile::generate(
                name,
                seed,
                Some(os),
                cores,
                ram,
                proxy,
                Some(timezone),
                Some(locale),
            );

            let meta_path = profile.save(&base_dir)?;
            let mut card = Card::new("BROWSER PROFILE CREATED");
            card.with_badge(badge_online("READY"));
            card.with_min_width(74);
            card.add_kv("Profile ID", &profile.id);
            card.add_kv("Profile Name", &profile.name);
            card.add_kv("PRNG Seed", profile.fingerprint_seed.map(|s| s.to_string()).unwrap_or_default());
            card.add_kv("Hardware", format!("{} cores / {} GB RAM", profile.hardware_concurrency.unwrap_or(8), profile.device_memory_gb.unwrap_or(16)));
            card.add_kv("Timezone / Lang", format!("{} / {}", profile.timezone.as_deref().unwrap_or("-"), profile.lang.as_deref().unwrap_or("-")));
            if let Some(ref pxy) = profile.proxy {
                card.add_kv("Proxy", format!("{} (WebRTC UDP Shielded)", pxy));
            }
            card.add_kv("Sandbox Directory", profile.get_sandbox_dir(&base_dir).display().to_string());
            card.add_kv("Config File", meta_path.display().to_string());
            card.with_footer(format!("Launch with: specter browser launch {}", profile.id));
            println!();
            card.print();
            println!();
            Ok(())
        }
        Some(ProfileCommands::Inspect { id, format }) => {
            let profile = match crate::core::browser::BrowserProfile::load(&id, &base_dir) {
                Ok(p) => p,
                Err(e) => {
                    crate::ui::Notify::error(&e);
                    return Err(e.into());
                }
            };

            if format.resolve().is_json() {
                println!("{}", serde_json::to_string_pretty(&profile)?);
                return Ok(());
            }

            let bytes = profile.calculate_disk_size(&base_dir);
            let size_str = if bytes > 1024 * 1024 {
                format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
            } else if bytes > 1024 {
                format!("{:.2} KB", bytes as f64 / 1024.0)
            } else {
                format!("{} bytes", bytes)
            };

            let mut card = Card::new("BROWSER PROFILE SPECIFICATION");
            card.with_badge(badge_online("INSPECTED"));
            card.with_min_width(74);
            card.add_kv("ID", &profile.id);
            card.add_kv("Name", &profile.name);
            card.add_kv("Fingerprint Seed", profile.fingerprint_seed.map(|s| format!("{} (Deterministic C++ PRNG)", s)).unwrap_or_else(|| "Random".to_string()));
            card.add_kv("Platform / Version", format!("{} ({})", profile.os_platform.as_deref().unwrap_or("windows"), profile.os_version.as_deref().unwrap_or("10.0.0")));
            card.add_kv("Browser Brand", profile.browser_brand.as_deref().unwrap_or("Chrome"));
            card.add_kv("Hardware Cores", profile.hardware_concurrency.unwrap_or(8).to_string());
            card.add_kv("Device Memory", format!("{} GB", profile.device_memory_gb.unwrap_or(16)));
            card.add_kv("Locale / Lang", profile.lang.as_deref().unwrap_or("vi-VN"));
            card.add_kv("Accept-Language", profile.accept_lang.as_deref().unwrap_or("vi-VN,vi,en-US,en"));
            card.add_kv("Timezone", profile.timezone.as_deref().unwrap_or("Asia/Ho_Chi_Minh"));
            card.add_kv("Proxy", profile.proxy.as_deref().unwrap_or("Direct (None)"));
            card.add_kv("WebRTC Shield", profile.webrtc_mode.as_deref().unwrap_or("proxy_shielded"));
            card.add_kv("Disk Footprint", size_str);
            card.add_kv("Sandbox Path", profile.get_sandbox_dir(&base_dir).display().to_string());
            card.with_footer(format!("Launch with: specter browser launch {}", profile.id));
            println!();
            card.print();
            println!();
            Ok(())
        }
        Some(ProfileCommands::Delete { id, force: _ }) => {
            match crate::core::browser::BrowserProfile::delete(&id, &base_dir) {
                Ok(true) => {
                    let mut card = Card::new("BROWSER PROFILE");
                    card.with_badge(badge_online("DELETED"));
                    card.with_min_width(68);
                    card.add_kv("Target ID", id);
                    card.add_line("Profile metadata and sandbox directory removed successfully.");
                    println!();
                    card.print();
                    println!();
                    Ok(())
                }
                Ok(false) => {
                    let mut card = Card::new("BROWSER PROFILE");
                    card.with_badge(badge_warn("NOT FOUND"));
                    card.with_min_width(68);
                    card.add_kv("Target ID", id);
                    card.add_line("No profile directory found to delete.");
                    println!();
                    card.print();
                    println!();
                    Ok(())
                }
                Err(e) => {
                    crate::ui::Notify::error(format!("Failed to delete profile: {}", e));
                    Err(e.into())
                }
            }
        }
        Some(ProfileCommands::Pack { id, output, level, json }) => {
            let profile = match crate::core::browser::BrowserProfile::load(&id, &base_dir) {
                Ok(p) => p,
                Err(e) => {
                    if json {
                        println!("{}", serde_json::json!({ "success": false, "error": e.to_string() }));
                    } else {
                        crate::ui::Notify::error(&e);
                    }
                    return Err(e.into());
                }
            };

            let default_out = base_dir.join("profiles").join(format!("{}.tar.zst", profile.id));
            let target_out = output.as_deref().unwrap_or(&default_out);

            let report = profile.pack(&base_dir, Some(target_out))?;

            if json {
                println!("{}", serde_json::json!({
                    "success": true,
                    "profile_id": profile.id,
                    "profile_name": profile.name,
                    "archive_path": report.archive_path.display().to_string(),
                    "file_count": report.file_count,
                    "uncompressed_bytes": report.uncompressed_bytes,
                    "compressed_bytes": report.compressed_bytes,
                    "compression_ratio": report.compression_ratio,
                    "sha256": report.sha256_hash,
                }));
                return Ok(());
            }

            let mut card = Card::new("PROFILE SNAPSHOT PACKED (ZSTD)");
            card.with_badge(badge_online("COMPRESSED"));
            card.with_min_width(74);
            card.add_kv("Profile ID", &profile.id);
            card.add_kv("Profile Name", &profile.name);
            card.add_kv("Archive File", report.archive_path.display().to_string());
            card.add_kv("Archived Files", format!("{} state files (cache filtered)", report.file_count));
            card.add_kv("Uncompressed", format!("{:.2} MB", report.uncompressed_bytes as f64 / (1024.0 * 1024.0)));
            card.add_kv("Compressed", format!("{:.2} MB (Level: {})", report.compressed_bytes as f64 / (1024.0 * 1024.0), level));
            card.add_kv("Compression Ratio", format!("{:.1}% saved", report.compression_ratio));
            card.add_kv("SHA-256 Digest", &report.sha256_hash);
            card.with_footer("Snapshot ready for Supabase Cloud Storage sync or offline backup");
            println!();
            card.print();
            println!();
            Ok(())
        }
        Some(ProfileCommands::Unpack { archive, hash, json }) => {
            if !archive.exists() {
                if json {
                    println!("{}", serde_json::json!({ "success": false, "error": format!("Archive file not found: {}", archive.display()) }));
                } else {
                    crate::ui::Notify::error(format!("Archive file not found at: {}", archive.display()));
                }
                return Err("Archive not found".into());
            }

            let profile = match crate::core::browser::BrowserProfile::unpack_archive(&archive, &base_dir, hash.as_deref()) {
                Ok(p) => p,
                Err(e) => {
                    if json {
                        println!("{}", serde_json::json!({ "success": false, "error": e.to_string() }));
                    } else {
                        crate::ui::Notify::error(format!("Failed to restore profile: {}", e));
                    }
                    return Err(e.into());
                }
            };

            if json {
                println!("{}", serde_json::json!({
                    "success": true,
                    "profile_id": profile.id,
                    "profile_name": profile.name,
                    "sandbox_dir": profile.get_sandbox_dir(&base_dir).display().to_string(),
                    "sha256": profile.storage_hash,
                    "verified": true,
                }));
                return Ok(());
            }

            let mut card = Card::new("PROFILE RESTORED FROM SNAPSHOT");
            card.with_badge(badge_online("RESTORED"));
            card.with_min_width(74);
            card.add_kv("Profile ID", &profile.id);
            card.add_kv("Profile Name", &profile.name);
            if let Some(seed) = profile.fingerprint_seed {
                card.add_kv("Fingerprint Seed", format!("{} (Verified)", seed));
            }
            card.add_kv("Sandbox Directory", profile.get_sandbox_dir(&base_dir).display().to_string());
            if let Some(ref h) = profile.storage_hash {
                card.add_kv("Verified SHA-256", h);
            }
            card.with_footer(format!("Launch with: specter browser launch {}", profile.id));
            println!();
            card.print();
            println!();
            Ok(())
        }
        Some(ProfileCommands::TestProxy { id, timeout, json }) => {
            let profile = match crate::core::browser::BrowserProfile::load(&id, &base_dir) {
                Ok(p) => p,
                Err(e) => {
                    if json {
                        println!("{}", serde_json::json!({ "success": false, "error": e.to_string() }));
                    } else {
                        crate::ui::Notify::error(&e);
                    }
                    return Err(e.into());
                }
            };

            let proxy_url = match profile.proxy {
                Some(ref p) if !p.trim().is_empty() => p.clone(),
                _ => {
                    if json {
                        println!("{}", serde_json::json!({ "success": false, "error": "Profile has no proxy configured (direct connection)" }));
                    } else {
                        println!();
                        let mut card = Card::new("BROWSER PROFILE PROXY");
                        card.with_badge(badge_warn("NO PROXY"));
                        card.with_min_width(74);
                        card.add_kv("Profile ID", &profile.id);
                        card.add_kv("Profile Name", &profile.name);
                        card.add_line("This profile is configured for DIRECT connection (no proxy).");
                        card.with_footer("Set proxy with: specter browser profile create <name> --proxy socks5://...");
                        card.print();
                        println!();
                    }
                    return Ok(());
                }
            };

            let res = crate::core::browser::ProxyProbe::probe(&proxy_url, timeout).await;

            if json {
                println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                    "success": res.alive,
                    "profile_id": profile.id,
                    "profile_name": profile.name,
                    "proxy": res,
                }))?);
                return Ok(());
            }

            println!();
            let mut card = Card::new("PROFILE PROXY PRE-FLIGHT PROBE");
            let badge = if res.alive {
                badge_online("OPERATIONAL")
            } else {
                badge_error("UNREACHABLE")
            };
            card.with_badge(badge);
            card.with_min_width(74);
            card.add_kv("Profile ID", &profile.id);
            card.add_kv("Profile Name", &profile.name);
            card.add_kv("Configured Proxy", &res.proxy_url);
            card.add_kv("Protocol", res.protocol.to_uppercase());
            if res.alive {
                card.add_kv("Latency (RTT)", format!("{} ms", res.rtt_ms));
                card.add_kv("Egress IP", res.egress_ip.as_deref().unwrap_or("Hidden / Direct"));
                if let Some(ref loc) = res.country {
                    card.add_kv("Country", loc);
                }
                if let Some(ref c) = res.colo {
                    card.add_kv("Edge Datacenter", format!("{} (Cloudflare)", c));
                }
                card.add_kv("WebRTC Shield", "Protected (--disable-non-proxied-udp)");
                card.with_footer("Profile proxy is healthy and ready for antidetect automation");
            } else {
                card.add_kv("Latency (RTT)", format!("{} ms (Failed)", res.rtt_ms));
                if let Some(ref err) = res.error {
                    card.add_kv("Error Details", err);
                }
                card.with_footer("Proxy failed pre-flight. Launching this profile will fail without --skip-proxy-check");
            }
            card.print();
            println!();
            Ok(())
        }
        Some(ProfileCommands::Push { id, storage_url, json }) => super::cloud::handle_profile_push(&id, storage_url.as_deref(), json).await,
        Some(ProfileCommands::Pull { id, storage_url, json }) => super::cloud::handle_profile_pull(&id, storage_url.as_deref(), json).await,
        Some(ProfileCommands::Cloud { command }) => super::cloud::handle_profile_cloud(command).await,
    }
}
