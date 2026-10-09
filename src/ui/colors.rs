pub const CYAN: &str = "\x1b[38;2;56;189;248m";
pub const PURPLE: &str = "\x1b[38;2;168;85;247m";
pub const GREEN: &str = "\x1b[38;2;74;222;128m";
pub const AMBER: &str = "\x1b[38;2;251;191;36m";
pub const RED: &str = "\x1b[38;2;248;113;113m";
pub const MUTED: &str = "\x1b[38;2;148;163;184m";
pub const BORDER: &str = "\x1b[38;2;71;85;105m";
pub const BOLD: &str = "\x1b[1m";
pub const BOLD_WHITE: &str = "\x1b[1;37m";
pub const DIM: &str = "\x1b[2m";
pub const RESET: &str = "\x1b[0m";

/// Calculate the visible display width of a string, ignoring ANSI escape codes.
pub fn visible_width(s: &str) -> usize {
    let mut in_escape = false;
    let mut width = 0;
    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if c == 'm' {
                in_escape = false;
            }
        } else {
            // Count characters (approx width)
            width += 1;
        }
    }
    width
}
