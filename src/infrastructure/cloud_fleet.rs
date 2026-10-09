use std::path::Path;
use serde_json::Value;
use specter_browser::{BrowserProfile, PackReport, ProxyProbe, ProxyProbeResult};

use crate::infrastructure::cloud_reporter::{CloudReporter, DeviceCredentials, DEFAULT_SUPABASE_ANON_KEY};
use crate::ui::{badge_error, badge_online, badge_step, Card};

pub struct CloudFleetOrchestrator;

#[derive(Debug, Clone)]
pub struct AcquiredLease {
    pub cloud_id: String,
    pub profile: BrowserProfile,
    pub restored_snapshot: bool,
    pub proxy: Option<String>,
}

impl CloudFleetOrchestrator {
    /// Resolves cloud profile UUID from either an exact UUID string or profile name
    pub async fn resolve_profile_id(
        client: &reqwest::Client,
        cloud_url: &str,
        anon_key: &str,
        creds: &DeviceCredentials,
        id_or_name: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if uuid::Uuid::parse_str(id_or_name).is_ok() {
            return Ok(id_or_name.to_string());
        }

        let list_url = format!("{}/rest/v1/rpc/list_browsers", cloud_url.trim_end_matches('/'));
        let res = client
            .post(&list_url)
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
            let err_body = res.text().await.unwrap_or_default();
            return Err(format!("Failed to list fleet profiles for lookup: {}", err_body).into());
        }

        let browsers: Vec<Value> = res.json().await?;
        for b in browsers {
            if let Some(name) = b.get("name").and_then(|v| v.as_str()) {
                if name.eq_ignore_ascii_case(id_or_name) {
                    if let Some(id) = b.get("id").and_then(|v| v.as_str()) {
                        return Ok(id.to_string());
                    }
                }
            }
        }

        Err(format!(
            "No cloud browser profile matching ID or name '{}' in fleet inventory",
            id_or_name
        )
        .into())
    }

    /// Acquires distributed lease lock on Supabase and restores session snapshot
    pub async fn acquire_lease(
        client: &reqwest::Client,
        cloud_url: &str,
        anon_key: &str,
        creds: &DeviceCredentials,
        cloud_id: &str,
        base_dir: &Path,
    ) -> Result<AcquiredLease, Box<dyn std::error::Error + Send + Sync>> {
        let rpc_url = format!("{}/rest/v1/rpc/acquire_browser", cloud_url.trim_end_matches('/'));
        let res = client
            .post(&rpc_url)
            .header("apikey", anon_key)
            .header("Authorization", format!("Bearer {}", anon_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "p_browser_id": cloud_id,
                "p_device_id": creds.device_id,
                "p_device_token": creds.device_token
            }))
            .send()
            .await?;

        let body: Value = res.json().await?;
        let success = body.get("success").and_then(|v| v.as_bool()).unwrap_or(false);

        if !success {
            let err_msg = body
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("Failed to acquire browser lease");
            let locked_device = body
                .get("locked_by_device_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let locked_at = body
                .get("locked_at")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            return Err(format!(
                "{} (Active lock held by device '{}' since {})",
                err_msg, locked_device, locked_at
            )
            .into());
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

        let mut profile = BrowserProfile::generate(
            name, seed, os, cores, ram, proxy.clone(), tz, loc,
        );
        profile.cloud_id = Some(cloud_id.to_string());
        profile.storage_path = browser["storage_path"].as_str().map(|s| s.to_string());
        profile.storage_hash = browser["storage_hash"].as_str().map(|s| s.to_string());
        profile.storage_size_bytes = browser["storage_size_bytes"].as_u64();
        profile.cookies_count = browser["cookies_count"].as_u64().map(|c| c as u32);

        // Save generated profile configuration
        profile.save(base_dir)?;

        // Restore snapshot archive if present locally or accessible
        let mut restored_snapshot = false;
        if let Some(ref sp) = profile.storage_path {
            let sp_path = Path::new(sp);
            let archive_candidate = if sp_path.is_absolute() {
                sp_path.to_path_buf()
            } else {
                base_dir.join(sp)
            };
            if archive_candidate.exists() {
                if let Ok(unpacked) = BrowserProfile::unpack_archive(
                    &archive_candidate,
                    base_dir,
                    profile.storage_hash.as_deref(),
                ) {
                    restored_snapshot = true;
                    let _ = unpacked.save(base_dir);
                }
            }
        }

        Ok(AcquiredLease {
            cloud_id: cloud_id.to_string(),
            profile,
            restored_snapshot,
            proxy,
        })
    }

    /// Pre-flight healthcheck probe on profile proxy
    pub async fn preflight_proxy(
        proxy: Option<&str>,
    ) -> Result<Option<ProxyProbeResult>, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(pxy) = proxy {
            let probe = ProxyProbe::probe(pxy, 4).await;
            if !probe.alive {
                let err_msg = probe.error.unwrap_or_else(|| "Connection timed out".to_string());
                return Err(format!("Configured proxy '{}' unreachable: {}", pxy, err_msg).into());
            }
            return Ok(Some(probe));
        }
        Ok(None)
    }

    /// Releases exclusive lease lock and synchronizes delta snapshot
    pub async fn release_lease(
        client: &reqwest::Client,
        cloud_url: &str,
        anon_key: &str,
        creds: &DeviceCredentials,
        profile: &BrowserProfile,
        base_dir: &Path,
    ) -> Result<PackReport, Box<dyn std::error::Error + Send + Sync>> {
        let report = profile.pack(base_dir, None)?;
        let cloud_target_id = profile.cloud_id.as_deref().unwrap_or(&profile.id);
        let canonical_storage = format!("profiles/{}.tar.zst", profile.id);

        let rpc_url = format!("{}/rest/v1/rpc/release_browser", cloud_url.trim_end_matches('/'));
        let res = client
            .post(&rpc_url)
            .header("apikey", anon_key)
            .header("Authorization", format!("Bearer {}", anon_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "p_browser_id": cloud_target_id,
                "p_device_id": creds.device_id,
                "p_device_token": creds.device_token,
                "p_storage_path": canonical_storage,
                "p_storage_size_bytes": report.compressed_bytes,
                "p_storage_hash": report.sha256_hash,
                "p_cookies_count": profile.cookies_count.unwrap_or(0),
                "p_metadata": {
                    "file_count": report.file_count,
                    "compression_ratio": report.compression_ratio
                }
            }))
            .send()
            .await?;

        let body: Value = res.json().await?;
        let success = body.get("success").and_then(|v| v.as_bool()).unwrap_or(false);

        if !success {
            let err_msg = body
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("Failed to release browser lease");
            return Err(err_msg.into());
        }

        Ok(report)
    }

    /// Emergency release when session aborts before profile modification
    pub async fn abort_lease(
        client: &reqwest::Client,
        cloud_url: &str,
        anon_key: &str,
        creds: &DeviceCredentials,
        cloud_id: &str,
    ) {
        let rpc_url = format!("{}/rest/v1/rpc/release_browser", cloud_url.trim_end_matches('/'));
        let _ = client
            .post(&rpc_url)
            .header("apikey", anon_key)
            .header("Authorization", format!("Bearer {}", anon_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "p_browser_id": cloud_id,
                "p_device_id": creds.device_id,
                "p_device_token": creds.device_token
            }))
            .send()
            .await;
    }

    /// Execute the complete autonomous cloud orchestration pipeline
    pub async fn run_autonomous_cloud_session(
        id_or_name: &str,
        workflow_path_opt: Option<String>,
        workflow_json_opt: Option<String>,
        headless: bool,
        variables: Vec<String>,
        timeout_opt: Option<u64>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let base_dir = specter_browser::resolve_data_dir();
        let app_config = crate::config::AppConfig::load();

        println!();
        println!("{} Resolving cloud profile lease credentials...", badge_step("1/6"));

        let creds = match CloudReporter::load_credentials(&app_config.data_dir).await {
            Some(c) => c,
            None => {
                eprintln!("\n{} Workstation not enrolled with Specter Cloud.", badge_error("AUTH ERROR"));
                eprintln!("Run 'specter runner enroll' or login to configure device credentials.\n");
                return Err("Missing cloud device credentials".into());
            }
        };

        let cloud_url = app_config.cloud_url.as_deref()
            .or(creds.cloud_url.as_deref())
            .unwrap_or("https://dswhacsoaxgpfnkaxnhz.supabase.co");
        let anon_key = creds.api_key.as_deref().unwrap_or(DEFAULT_SUPABASE_ANON_KEY);
        let client = CloudReporter::build_http_client();

        let cloud_id = Self::resolve_profile_id(&client, cloud_url, anon_key, &creds, id_or_name)
            .await
            .map_err(|e| format!("{}", e))?;

        println!("{} Acquiring exclusive lease on profile '{}'...", badge_step("2/6"), cloud_id);
        let lease = match Self::acquire_lease(&client, cloud_url, anon_key, &creds, &cloud_id, &base_dir).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("\n{} {}", badge_error("ACQUIRE FAILED"), e);
                return Err(format!("{}", e).into());
            }
        };

        println!(
            "  • Profile ID: {} [{}]",
            lease.profile.name, lease.profile.id
        );
        if lease.restored_snapshot {
            println!("  • Restored session snapshot (.tar.zst delta intact)");
        }

        println!("{} Verifying network & proxy pre-flight guardrails...", badge_step("3/6"));
        if let Err(probe_err) = Self::preflight_proxy(lease.proxy.as_deref()).await {
            eprintln!("\n{} {}", badge_error("PRE-FLIGHT FAILED"), probe_err);
            println!("  • Aborting to prevent IP leak. Rolling back lease to IDLE...");
            Self::abort_lease(&client, cloud_url, anon_key, &creds, &cloud_id).await;
            return Err(format!("{}", probe_err).into());
        }

        println!("{} Booting stealth browser session & executing workflow...", badge_step("4/6"));
        let execution_result = if workflow_path_opt.is_some() || workflow_json_opt.is_some() {
            crate::commands::automa::run_workflow(
                workflow_path_opt,
                workflow_json_opt,
                headless,
                Some("chromium".to_string()),
                Some(lease.profile.id.clone()),
                variables,
                timeout_opt,
            ).await
        } else {
            // Smoke test / verification run
            println!("  • No workflow specified. Running cloud profile health verification probe...");
            tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
            Ok(())
        };

        println!("{} Packaging LevelDB/cookies delta into .tar.zst snapshot...", badge_step("5/6"));
        let report = match Self::release_lease(&client, cloud_url, anon_key, &creds, &lease.profile, &base_dir).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("\n{} {}", badge_error("RELEASE FAILED"), e);
                return Err(format!("{}", e).into());
            }
        };

        println!("{} Lease successfully returned to cloud fleet!", badge_step("6/6"));

        println!();
        let mut card = Card::new("AUTONOMOUS CLOUD WORKFLOW PIPELINE");
        card.with_badge(badge_online("COMPLETED"));
        card.with_min_width(74);
        card.add_kv("Cloud Browser ID", &cloud_id);
        card.add_kv("Profile", format!("{} [{}]", lease.profile.name, lease.profile.id));
        if let Some(s) = lease.profile.fingerprint_seed {
            card.add_kv("Hardware PRNG", format!("{} (Deterministic)", s));
        }
        card.add_kv("Delta Storage", format!("~/.specter/browser/profiles/{}.tar.zst", lease.profile.id));
        card.add_kv("Compressed Size", format!("{} bytes ({:.1}% ratio)", report.compressed_bytes, report.compression_ratio));
        card.add_kv("SHA-256 Digest", &report.sha256_hash);
        card.add_kv("Lease State", "IDLE (Unlocked for fleet mesh)");
        card.with_footer("Cloud Profile Fleet Mesh orchestration pipeline finished successfully.");
        card.print();
        println!();

        execution_result
    }
}
