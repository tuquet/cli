use super::colors;

/// Brand Hero Banner with Styled ASCII Art Logo
pub fn render_hero(
    version: &str,
    cloud_status: &str,
    daemon_status: &str,
    browser_status: &str,
) -> String {
    let reset = colors::RESET;
    let bold_white = colors::BOLD_WHITE;
    let muted = colors::MUTED;
    let cyan = colors::CYAN;

    let c1 = "\x1b[38;2;56;189;248m";
    let c2 = "\x1b[38;2;96;165;250m";
    let c3 = "\x1b[38;2;129;140;248m";
    let c4 = "\x1b[38;2;168;85;247m";
    let c5 = "\x1b[38;2;192;132;252m";

    format!(
"
{c1}  ______          ____                  __ {reset}
{c2} /_  __/_  __    / __ \\__  __  ___     / /_{reset}
{c3}  / /  / / / /  / / / // / / // _ \\   / __/{reset}   {cyan}v{version}{reset}
{c4} / /  / /_/ /  / /_/ // /_/ //  __/  / /_  {reset}
{c5}/_/   \\__,_/   \\___\\_\\\\__,_/ \\___/   \\__/  {reset}

  {bold_white}Autonomous Browser Automation & Distributed Mesh Runtime{reset}
  {cloud_status}  {daemon_status}  {browser_status}
  {muted}Press [Tab] for autocomplete, 'help' for commands, 'exit' to quit{reset}
"
    )
}

/// Render a sleek notification banner when a newer CLI release is available
pub fn render_update_banner(current: &str, latest: &str) -> String {
    let amber = colors::AMBER;
    let bold = colors::BOLD;
    let white = colors::BOLD_WHITE;
    let reset = colors::RESET;
    let muted = colors::MUTED;
    let cyan = colors::CYAN;

    format!(
"  {amber}╭──────────────────────────────────────────────────────────────────────────╮{reset}
  {amber}│{reset}  🔔 {bold}{white}Update available:{reset} {muted}v{current}{reset} → {cyan}v{latest}{reset}  {muted}(Run 'specter upgrade' or 'scoop update'){reset} {amber}│{reset}
  {amber}╰──────────────────────────────────────────────────────────────────────────╯{reset}"
    )
}
