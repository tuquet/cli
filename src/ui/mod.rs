pub mod badges;
pub mod banner;
pub mod card;
pub mod colors;
pub mod notify;
pub mod table;

pub use badges::{badge_error, badge_offline, badge_online, badge_step, badge_warn, status_pill};
pub use banner::{render_hero, render_update_banner};
pub use card::Card;
pub use colors::visible_width;
pub use notify::Notify;
pub use table::{
    create_network_topology_card, create_tabular_card, default_workstation_endpoints, Column,
    NetworkEndpoint, Table, TabularRow,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_network_topology_card_standby() {
        let endpoints = default_workstation_endpoints(
            1080,
            false,
            8118,
            false,
            2222,
            false,
            Some((8765, false)),
        );
        assert_eq!(endpoints.len(), 4);

        let card = create_network_topology_card("NETWORK TOPOLOGY & LISTENERS", &endpoints, None);
        let rendered = card.render();

        assert!(rendered.contains("NETWORK TOPOLOGY & LISTENERS"));
        assert!(rendered.contains("STANDBY"));
        assert!(rendered.contains("PROTOCOL"));
        assert!(rendered.contains("ENDPOINT"));
        assert!(rendered.contains("ROLE / TARGET"));
        assert!(rendered.contains("STATUS"));
        assert!(rendered.contains("SOCKS5"));
        assert!(rendered.contains("127.0.0.1:1080"));
        assert!(rendered.contains("Dynamic Proxy (SSH)"));
        assert!(rendered.contains("HTTP Relay"));
        assert!(rendered.contains("127.0.0.1:8118"));
        assert!(rendered.contains("HTTP-to-SOCKS5 (:1080)"));
        assert!(rendered.contains("TCP Tunnel"));
        assert!(rendered.contains("127.0.0.1:2222"));
        assert!(rendered.contains("Cloudflare Access (VPS)"));
        assert!(rendered.contains("Daemon API"));
        assert!(rendered.contains("127.0.0.1:8765"));
        assert!(rendered.contains("Runner Worker (RPC/WS)"));
    }

    #[test]
    fn test_create_network_topology_card_active() {
        let endpoints = default_workstation_endpoints(1080, true, 8118, false, 2222, true, None);
        assert_eq!(endpoints.len(), 3);

        let card = create_network_topology_card(
            "NETWORK BRIDGE & LISTENERS",
            &endpoints,
            Some("Custom footer tip"),
        );
        let rendered = card.render();

        assert!(rendered.contains("ACTIVE WORKLOADS"));
        assert!(rendered.contains("ACTIVE"));
        assert!(rendered.contains("Custom footer tip"));
    }

    #[test]
    fn test_visible_width_ansi_stripping() {
        let badge = badge_online("ACTIVE");
        assert_eq!(visible_width(&badge), 8); // '●' (1) + ' ' (1) + 'ACTIVE' (6)

        let standby = badge_offline("STANDBY");
        assert_eq!(visible_width(&standby), 9); // '○' (1) + ' ' (1) + 'STANDBY' (7)
    }

    #[test]
    fn test_create_tabular_card_generic() {
        let rows = vec![
            TabularRow::new("Workstation", "HNDW-NDTU6", "Local Node Host", badge_online("READY")),
            TabularRow::new("Cloud Target", "supabase.co", "Cloud Mesh API", badge_online("ENROLLED")),
        ];
        let card = create_tabular_card(
            "SYSTEM & CLOUD TOPOLOGY",
            Some(badge_online("PROD")),
            ["COMPONENT", "IDENTITY / TARGET", "ROLE / DETAILS", "STATUS"],
            &rows,
            Some("System operational"),
            72,
        );
        let rendered = card.render();
        assert!(rendered.contains("SYSTEM & CLOUD TOPOLOGY"));
        assert!(rendered.contains("PROD"));
        assert!(rendered.contains("Workstation"));
        assert!(rendered.contains("HNDW-NDTU6"));
        assert!(rendered.contains("Local Node Host"));
        assert!(rendered.contains("READY"));
        assert!(rendered.contains("System operational"));
    }
}
