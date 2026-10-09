use crate::ui::{badge_error, badge_online, Card};

pub async fn handle_proxy(
    command: Option<crate::cli::ProxyCommands>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (url, timeout, json) = match command {
        Some(crate::cli::ProxyCommands::Probe { url, timeout, json }) => (url, timeout, json),
        None => ("socks5://127.0.0.1:1080".to_string(), 5, false),
    };

    let res = crate::core::browser::ProxyProbe::probe(&url, timeout).await;

    if json {
        println!("{}", serde_json::to_string_pretty(&res)?);
        return Ok(());
    }

    println!();
    let mut card = Card::new("NETWORK PROXY PRE-FLIGHT PROBE");
    let badge = if res.alive {
        badge_online("OPERATIONAL")
    } else {
        badge_error("UNREACHABLE")
    };
    card.with_badge(badge);
    card.with_min_width(74);
    card.add_kv("Target Proxy", &res.proxy_url);
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
        card.with_footer("Proxy verified ready for antidetect browser profiles");
    } else {
        card.add_kv("Latency (RTT)", format!("{} ms (Failed)", res.rtt_ms));
        if let Some(ref err) = res.error {
            card.add_kv("Error Details", err);
        }
        card.with_footer("Proxy failed pre-flight probe. Browser sessions with this proxy will fail.");
    }
    card.print();
    println!();
    Ok(())
}
