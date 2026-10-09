use crate::config::AppConfig;
use super::supervisor::get_log_file_path;

pub async fn show_logs(follow: bool, lines: usize) -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load();
    let log_file = get_log_file_path(&config.data_dir);
    if !log_file.exists() {
        let mut card = crate::ui::Card::new("RUNNER");
        card.with_badge(crate::ui::badge_warn("NO LOGS FOUND"));
        card.with_min_width(64);
        card.add_kv("Expected Log File", log_file.display().to_string());
        card.add_line("Daemon has not written any background logs yet.");
        card.with_footer("Start daemon in background with 'specter runner start -d'");
        println!();
        card.print();
        println!();
        return Ok(());
    }

    let content = std::fs::read_to_string(&log_file)?;
    let all_lines: Vec<&str> = content.lines().collect();
    let start_idx = all_lines.len().saturating_sub(lines);

    println!("\x1b[1;36m╭─ RUNNER LOGS ({}) ─\x1b[0m", log_file.display());
    for line in &all_lines[start_idx..] {
        println!("│ {}", line);
    }
    println!("\x1b[1;36m╰───────────────────────────────────────────────────\x1b[0m");

    if follow {
        use std::io::{BufRead, BufReader, Seek, SeekFrom};
        let file = std::fs::File::open(&log_file)?;
        let mut reader = BufReader::new(file);
        reader.seek(SeekFrom::End(0))?;

        let mut line_buf = String::new();
        loop {
            line_buf.clear();
            let bytes = reader.read_line(&mut line_buf)?;
            if bytes > 0 {
                print!("│ {}", line_buf);
            } else {
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            }
        }
    }

    Ok(())
}
