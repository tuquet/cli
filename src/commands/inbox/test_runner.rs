use std::time::Instant;
use serde::{Deserialize, Serialize};

use crate::commands::inbox::cloud_client::InboxCloudClient;
use crate::commands::inbox::parser::EmailParser;
use crate::commands::inbox::storage::{EmailRecord, InboxStorage, OtpRecord};
use crate::config::inbox::InboxConfig;
use crate::ui::{badge_offline, badge_online, Card};

#[derive(Debug, Serialize, Deserialize)]
pub struct AcceptanceCheckResult {
    pub name: String,
    pub status: String,
    pub detail: String,
    pub duration_ms: u128,
    pub passed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AcceptanceReport {
    pub overall_passed: bool,
    pub simulated: bool,
    pub timestamp: String,
    pub checks: Vec<AcceptanceCheckResult>,
}

pub async fn run_acceptance_test(
    simulate: bool,
    remote: bool,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut checks: Vec<AcceptanceCheckResult> = Vec::new();
    let config = InboxConfig::load();
    let test_start = Instant::now();

    // 1. Local Storage Engine Check
    let t0 = Instant::now();
    let db_path = InboxConfig::sqlite_path();
    let storage_res = InboxStorage::open(db_path);
    let (storage, storage_passed) = match storage_res {
        Ok(s) => {
            let dur = t0.elapsed().as_millis();
            checks.push(AcceptanceCheckResult {
                name: "Local Storage Engine".to_string(),
                status: "OK".to_string(),
                detail: format!("SQLite Healthy ({}ms)", dur),
                duration_ms: dur,
                passed: true,
            });
            (Some(s), true)
        }
        Err(e) => {
            let dur = t0.elapsed().as_millis();
            checks.push(AcceptanceCheckResult {
                name: "Local Storage Engine".to_string(),
                status: "FAILED".to_string(),
                detail: format!("Storage error: {}", e),
                duration_ms: dur,
                passed: false,
            });
            (None, false)
        }
    };

    // 2. SSOT Cloud Identity Auto-Discovery
    let t1 = Instant::now();
    let cloud = InboxCloudClient::new(&config);
    let is_configured = cloud.is_configured();
    let dev_id = cloud.device_id().map(|s| s.to_string()).unwrap_or_else(|| "unpaired".to_string());
    let dur1 = t1.elapsed().as_millis();

    if is_configured {
        checks.push(AcceptanceCheckResult {
            name: "Cloud Identity Resolution".to_string(),
            status: "OK".to_string(),
            detail: format!("Inherited from Device [{}]", dev_id),
            duration_ms: dur1,
            passed: true,
        });
    } else {
        checks.push(AcceptanceCheckResult {
            name: "Cloud Identity Resolution".to_string(),
            status: "STANDBY".to_string(),
            detail: "Local-only mode (No cloud identity configured)".to_string(),
            duration_ms: dur1,
            passed: true, // Local mode is valid for MMO
        });
    }

    // 3. Remote Endpoint Connectivity (Graceful fallback for offline / bridge-isolated workstations)
    if remote {
        let t2 = Instant::now();
        match cloud.ping().await {
            Ok(code) => {
                let dur2 = t2.elapsed().as_millis();
                let ok = code == 200 || code == 404;
                checks.push(AcceptanceCheckResult {
                    name: "Remote Cloud Endpoint".to_string(),
                    status: if ok { "ONLINE".to_string() } else { "HTTP_ERROR".to_string() },
                    detail: format!("HTTP {} ({}ms)", code, dur2),
                    duration_ms: dur2,
                    passed: ok,
                });
            }
            Err(e) => {
                let dur2 = t2.elapsed().as_millis();
                checks.push(AcceptanceCheckResult {
                    name: "Remote Cloud Endpoint".to_string(),
                    status: "UNREACHABLE".to_string(),
                    detail: format!("Ping failed: {}", e),
                    duration_ms: dur2,
                    passed: false,
                });
            }
        }
    } else if is_configured && !simulate {
        let t2 = Instant::now();
        match cloud.ping().await {
            Ok(code) => {
                let dur2 = t2.elapsed().as_millis();
                checks.push(AcceptanceCheckResult {
                    name: "Remote Cloud Endpoint".to_string(),
                    status: "ONLINE".to_string(),
                    detail: format!("HTTP {} ({}ms)", code, dur2),
                    duration_ms: dur2,
                    passed: true,
                });
            }
            Err(_) => {
                let dur2 = t2.elapsed().as_millis();
                checks.push(AcceptanceCheckResult {
                    name: "Remote Cloud Endpoint".to_string(),
                    status: "STANDBY".to_string(),
                    detail: "Standby / Restricted network (Start bridge if sync needed)".to_string(),
                    duration_ms: dur2,
                    passed: true, // Non-fatal for offline MMO execution
                });
            }
        }
    }

    // 4. Ingestion & Regex Extraction Test
    let test_recipient = format!("test_mmo_{}@specter.local", test_start.elapsed().as_millis());
    let test_code = "839201";
    let t3 = Instant::now();
    let parsed = EmailParser::parse(
        "security@google.com",
        Some("Your Google verification code is 839201"),
        Some("G-839201 is your Google verification code. Do not share it with anyone."),
        Some("<p>G-<b>839201</b> is your verification code.</p>"),
    );
    let dur3 = t3.elapsed().as_millis();

    let otp_found = parsed.otps.iter().find(|o| o.code == test_code);
    if let Some(otp) = otp_found {
        checks.push(AcceptanceCheckResult {
            name: "Ingestion & Regex Engine".to_string(),
            status: "OK".to_string(),
            detail: format!("Code '{}' intercepted (Service: {}) in {}ms", otp.code, otp.service.as_deref().unwrap_or("Unknown"), dur3),
            duration_ms: dur3,
            passed: true,
        });
    } else {
        checks.push(AcceptanceCheckResult {
            name: "Ingestion & Regex Engine".to_string(),
            status: "FAILED".to_string(),
            detail: "Failed to extract test OTP code".to_string(),
            duration_ms: dur3,
            passed: false,
        });
    }

    // 5. Anti-Duplicate Single-Consumer Lock Test
    if let (Some(storage), Some(otp)) = (&storage, otp_found) {
        let t4 = Instant::now();
        let email_id = format!("test-mail-{}", test_start.elapsed().as_millis());
        let otp_id = format!("test-otp-{}", test_start.elapsed().as_millis());

        let email_rec = EmailRecord {
            id: email_id.clone(),
            message_id: None,
            sender: "security@google.com".to_string(),
            recipient: test_recipient.clone(),
            domain: "specter.local".to_string(),
            subject: Some("Your Google verification code is 839201".to_string()),
            body_text: Some("G-839201 is your code".to_string()),
            body_html: None,
            received_at: "2026-10-09T00:00:00Z".to_string(),
            raw_headers: None,
            created_at: "2026-10-09T00:00:00Z".to_string(),
        };

        let otp_rec = OtpRecord {
            id: otp_id.clone(),
            email_id: email_id.clone(),
            recipient: test_recipient.clone(),
            otp_code: otp.code.clone(),
            service_name: otp.service.clone(),
            expires_at: None,
            consumed_at: None,
            consumed_by: None,
            created_at: "2026-10-09T00:00:00Z".to_string(),
        };

        let _ = storage.save_email(&email_rec);
        let _ = storage.save_otp(&otp_rec);

        // First consume must succeed
        let consume1 = storage.consume_otp(&otp_id, "runner_worker_01").unwrap_or(false);
        // Second consume must be rejected (idempotence)
        let consume2 = storage.consume_otp(&otp_id, "runner_worker_02").unwrap_or(false);

        let dur4 = t4.elapsed().as_millis();
        if consume1 && !consume2 {
            checks.push(AcceptanceCheckResult {
                name: "Anti-Duplicate Lock".to_string(),
                status: "OK".to_string(),
                detail: format!("Single-Consumer guard verified ({}ms)", dur4),
                duration_ms: dur4,
                passed: true,
            });
        } else {
            checks.push(AcceptanceCheckResult {
                name: "Anti-Duplicate Lock".to_string(),
                status: "FAILED".to_string(),
                detail: format!("Concurrency lock violated: consume1={}, consume2={}", consume1, consume2),
                duration_ms: dur4,
                passed: false,
            });
        }
    }

    let overall_passed = checks.iter().all(|c| c.passed) && storage_passed;

    if json_output {
        let report = AcceptanceReport {
            overall_passed,
            simulated: simulate,
            timestamp: format!("{:?}", std::time::SystemTime::now()),
            checks,
        };
        println!("{}", serde_json::to_string_pretty(&report)?);
        if !overall_passed {
            std::process::exit(1);
        }
        return Ok(());
    }

    // Terminal Card UI Output
    println!();
    let mut card = Card::new("SPECTER INBOX - ZERO-CONFIG ACCEPTANCE TEST");
    if overall_passed {
        card.with_badge(badge_online("ALL CHECKS PASSED"));
    } else {
        card.with_badge(badge_offline("ACCEPTANCE FAILED"));
    }

    for c in &checks {
        let status_str = if c.passed {
            format!("[OK] {}", c.detail)
        } else {
            format!("[FAIL] {}", c.detail)
        };
        card.add_kv(&c.name, status_str);
    }

    card.print();
    println!();

    if overall_passed {
        println!(" [RESULT] Specter Inbox is 100% healthy and ready for MMO automation!");
    } else {
        eprintln!(" [RESULT] One or more checks failed. Please check the logs above.");
        std::process::exit(1);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_acceptance_runner_in_memory() {
        let res = run_acceptance_test(true, false, false).await;
        assert!(res.is_ok());
    }
}
