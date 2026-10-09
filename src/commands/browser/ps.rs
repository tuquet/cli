use crate::ui::{badge_offline, badge_online, respond_with, Card, Column, OutputFormat, Table};

pub async fn handle_ps(format: OutputFormat) -> Result<(), Box<dyn std::error::Error>> {
    let base_dir = crate::core::browser::resolve_data_dir();
    let sessions = crate::core::browser::pid_tracker::list_sessions(&base_dir, true);

    respond_with(format, &sessions, |s_list| {
        println!();
        let mut header_card = Card::new("ACTIVE BROWSER PROFILES");
        header_card.with_badge(badge_online(&format!("{} RUNNING", s_list.len())));
        header_card.with_min_width(74);
        header_card.add_kv("Session SSOT", base_dir.join("pids").display().to_string());
        header_card.with_footer("Run 'specter browser stop <id>' to terminate or 'specter browser launch <id>' to open");
        header_card.print();
        println!();

        if s_list.is_empty() {
            let mut empty_card = Card::new("NO RUNNING INSTANCES");
            empty_card.with_badge(badge_offline("IDLE"));
            empty_card.with_min_width(74);
            empty_card.add_line("No active browser profiles are currently running.");
            empty_card.add_line("Launch a profile with:");
            empty_card.add_line("  specter browser launch <profile_name>");
            empty_card.with_footer("List stored profiles with: specter browser profile list");
            empty_card.print();
            println!();
            return;
        }

    let columns = vec![
        Column { title: "Profile Name".to_string(), min_width: 20, align_right: false },
        Column { title: "Profile ID".to_string(), min_width: 24, align_right: false },
        Column { title: "PID".to_string(), min_width: 8, align_right: true },
        Column { title: "Mode".to_string(), min_width: 14, align_right: false },
        Column { title: "CDP Port".to_string(), min_width: 10, align_right: false },
        Column { title: "Uptime".to_string(), min_width: 10, align_right: false },
        Column { title: "Proxy".to_string(), min_width: 20, align_right: false },
        Column { title: "Status".to_string(), min_width: 12, align_right: false },
    ];

    let mut table = Table::new(columns);
    for s in &sessions {
        let mode_str = if s.mode == "extension" { "Stealth (Zero-Port)" } else { "CDP Bridge" };
        let port_str = if s.port > 0 { s.port.to_string() } else { "Disabled".to_string() };
        let proxy_str = s.proxy.as_deref().unwrap_or("Direct");
        let proxy_short = if proxy_str.len() > 20 {
            format!("{}...", &proxy_str[..17])
        } else {
            proxy_str.to_string()
        };

        table.add_row(vec![
            s.profile_name.clone(),
            s.profile_id.clone(),
            s.pid.to_string(),
            mode_str.to_string(),
            port_str,
            s.uptime_formatted(),
            proxy_short,
            "● RUNNING".to_string(),
        ]);
    }
    table.print();
    println!();
})
}
