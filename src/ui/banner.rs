/// Clean minimal hero without heavy ASCII art
pub fn render_hero(
    version: &str,
    cloud_status: &str,
    daemon_status: &str,
    browser_status: &str,
) -> String {
    format!(
        "tq v{version} — Autonomous Browser Automation & Distributed Mesh Runtime\n  {cloud_status}  {daemon_status}  {browser_status}\n"
    )
}

/// Clean 1-line update notification
pub fn render_update_banner(current: &str, latest: &str) -> String {
    format!("🔔 Update available: v{current} → v{latest} (run `tq upgrade`)")
}
