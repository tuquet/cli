use crate::ui::{Column, OutputFormat, Table, respond_with};

pub async fn handle_ps(format: OutputFormat) -> Result<(), Box<dyn std::error::Error>> {
    let base_dir = crate::core::browser::resolve_data_dir();
    let sessions = crate::core::browser::pid_tracker::list_sessions(&base_dir, true);

    respond_with(format, &sessions, |s_list| {
        if s_list.is_empty() {
            println!("No active browser profiles are currently running.");
            return;
        }

        let columns = vec![
            Column::new("Profile").with_priority(1),
            Column::new("ID").with_priority(1),
            Column::new("PID").right_aligned().with_priority(1),
            Column::new("Port").with_priority(2),
            Column::new("Mode").with_priority(3),
            Column::new("Uptime").with_priority(2),
            Column::new("Proxy").with_priority(3),
            Column::new("Status").with_priority(1),
        ];

        let mut table = Table::new(columns);
        for s in &sessions {
            let mode_str = if s.mode == "extension" {
                "stealth"
            } else {
                "cdp"
            };
            let port_str = if s.port > 0 {
                s.port.to_string()
            } else {
                "-".to_string()
            };
            let proxy_str = s.proxy.as_deref().unwrap_or("direct");

            table.add_row(vec![
                s.profile_name.clone(),
                s.profile_id.clone(),
                s.pid.to_string(),
                port_str,
                mode_str.to_string(),
                s.uptime_formatted(),
                proxy_str.to_string(),
                "RUNNING".to_string(),
            ]);
        }
        table.print();
    })
}
