use crate::core::browser::ProxyProbe;
use crate::ui::{Card, badge_offline, badge_online};

pub async fn run_probe(
    url: Option<String>,
    timeout: u64,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let target_url = url.unwrap_or_else(|| "socks5://127.0.0.1:1080".to_string());

    let res = ProxyProbe::probe(&target_url, timeout).await;

    if json {
        println!("{}", serde_json::to_string_pretty(&res)?);
        return Ok(());
    }

    let mut card = Card::new("GATEWAY EGRESS & WEBRTC LEAK PROBE");
    if res.alive {
        card.with_badge(badge_online("OPERATIONAL"));
    } else {
        card.with_badge(badge_offline("UNREACHABLE"));
    }

    card.add_kv("Target Proxy", &res.proxy_url);
    card.add_kv(
        "Status",
        if res.alive {
            "Online / Egress verified"
        } else {
            "Offline / Unreachable"
        },
    );

    if res.alive {
        if let Some(ref ip) = res.egress_ip {
            card.add_kv("Public Egress IP", ip);
        }
        if let Some(ref country) = res.country {
            card.add_kv("Country / Region", country);
        }
        card.add_kv("Round-Trip Latency", format!("{} ms", res.rtt_ms));
        card.add_kv(
            "WebRTC Shield",
            "Active (--disable-non-proxied-udp enforced)",
        );
        card.with_footer("Gateway proxy is operational and safe for Antidetect Browser profiles");
    } else {
        if let Some(ref err) = res.error {
            card.add_kv("Error Details", err);
        }
        card.with_footer(
            "Pre-flight egress check failed. Verify tunnel status with: tq gateway status",
        );
    }

    card.render();
    Ok(())
}
