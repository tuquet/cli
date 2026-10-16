use crate::ui::{
    Card, ClackOption, ClackSpinner, badge_error, badge_online, clack_cancel, clack_intro,
    clack_outro, clack_select, clack_text, is_interactive_tty,
};

pub async fn handle_proxy(
    command: Option<crate::cli::ProxyCommands>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (url, timeout, json) = match command {
        Some(crate::cli::ProxyCommands::Probe { url, timeout, json }) => (url, timeout, json),
        None => {
            if !is_interactive_tty() {
                ("socks5://127.0.0.1:1080".to_string(), 5, true)
            } else {
                clack_intro(
                    "spc proxy",
                    "Network Proxy Pre-Flight Probe & WebRTC Verifier",
                );
                let opts = vec![
                    ClackOption::new(
                        "Probe Local SOCKS5 Bridge (127.0.0.1:1080)",
                        "Verify SSH Dynamic Proxy tunnel egress & Cloudflare datacenter",
                        "socks5://127.0.0.1:1080",
                    ),
                    ClackOption::new(
                        "Probe Local HTTP Relay (127.0.0.1:8118)",
                        "Verify Embedded HTTP-to-SOCKS5 adapter on port 8118",
                        "http://127.0.0.1:8118",
                    ),
                    ClackOption::new(
                        "Probe Custom Proxy URL",
                        "Enter a custom socks5:// or http:// proxy endpoint",
                        "custom",
                    ),
                ];
                let Some(idx) = clack_select("Select proxy endpoint to probe:", &opts, 0, 6)?
                else {
                    clack_cancel("Proxy probe cancelled.");
                    return Ok(());
                };
                let target_url = if idx == 2 {
                    let Some(u) =
                        clack_text("Enter proxy URL:", Some("socks5://127.0.0.1:1080"), None)?
                    else {
                        clack_cancel("Cancelled.");
                        return Ok(());
                    };
                    u
                } else {
                    opts[idx].summary.clone()
                };
                (target_url, 5, false)
            }
        }
    };

    let sp = if !json {
        Some(ClackSpinner::start(format!(
            "Probing proxy {} (timeout: {}s)...",
            url, timeout
        )))
    } else {
        None
    };

    let res = crate::core::browser::ProxyProbe::probe(&url, timeout).await;

    if let Some(s) = sp {
        if res.alive {
            s.finish(format!(
                "Proxy operational ({} ms • Egress: {})",
                res.rtt_ms,
                res.egress_ip.as_deref().unwrap_or("Direct")
            ));
        } else {
            s.fail(format!("Proxy unreachable ({})", url));
        }
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&res)?);
        return Ok(());
    }

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
        card.add_kv(
            "Egress IP",
            res.egress_ip.as_deref().unwrap_or("Hidden / Direct"),
        );
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
        card.with_footer(
            "Proxy failed pre-flight probe. Browser sessions with this proxy will fail.",
        );
    }
    card.print();
    clack_outro("Proxy probe complete.");
    Ok(())
}
