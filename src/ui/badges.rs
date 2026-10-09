use super::colors;

pub fn badge_online(text: &str) -> String {
    format!(
        "{GREEN}●{RESET} {BOLD_WHITE}{}{RESET}",
        text,
        GREEN = colors::GREEN,
        RESET = colors::RESET,
        BOLD_WHITE = colors::BOLD_WHITE,
    )
}

pub fn badge_offline(text: &str) -> String {
    format!(
        "{MUTED}○{RESET} {MUTED}{}{RESET}",
        text,
        MUTED = colors::MUTED,
        RESET = colors::RESET,
    )
}

pub fn badge_warn(text: &str) -> String {
    format!(
        "{AMBER}●{RESET} {AMBER}{}{RESET}",
        text,
        AMBER = colors::AMBER,
        RESET = colors::RESET,
    )
}

pub fn badge_error(text: &str) -> String {
    format!(
        "{RED}●{RESET} {RED}{}{RESET}",
        text,
        RED = colors::RED,
        RESET = colors::RESET,
    )
}

pub fn badge_step(text: &str) -> String {
    format!(
        "{CYAN}[+]{RESET} {BOLD_WHITE}{}{RESET}",
        text,
        CYAN = colors::CYAN,
        RESET = colors::RESET,
        BOLD_WHITE = colors::BOLD_WHITE,
    )
}

pub fn status_pill(label: &str, value: &str, is_online: bool) -> String {
    if is_online {
        format!(
            "{BORDER}[{RESET}{MUTED}{}:{RESET}{GREEN}● {}{RESET}{BORDER}]{RESET}",
            label,
            value,
            BORDER = colors::BORDER,
            RESET = colors::RESET,
            MUTED = colors::MUTED,
            GREEN = colors::GREEN,
        )
    } else {
        format!(
            "{BORDER}[{RESET}{MUTED}{}:{RESET}{MUTED}○ {}{RESET}{BORDER}]{RESET}",
            label,
            value,
            BORDER = colors::BORDER,
            RESET = colors::RESET,
            MUTED = colors::MUTED,
        )
    }
}
