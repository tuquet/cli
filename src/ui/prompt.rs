use super::colors::{self, visible_width};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{self, Clear, ClearType},
};
use std::io::{self, IsTerminal, Write};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

// @clack/prompts Unicode Box-Drawing & State Symbols
pub const S_BAR_START: &str = "┌";
pub const S_BAR: &str = "│";
pub const S_BAR_END: &str = "└";
pub const S_STEP_ACTIVE: &str = "◆";
pub const S_STEP_SUBMIT: &str = "◇";
pub const S_STEP_WARN: &str = "▲";
pub const S_STEP_CANCEL: &str = "■";
pub const S_STEP_SKIP: &str = "○";
pub const S_RADIO_ACTIVE: &str = "●";
pub const S_RADIO_INACTIVE: &str = "○";
pub const S_CHECK_ACTIVE: &str = "◼";
pub const S_CHECK_INACTIVE: &str = "◻";
pub const S_PASSWORD_MASK: char = '▪';

/// Check if interactive TTY mode is active for human user
#[inline]
pub fn is_interactive_tty() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal()
}

/// Strip ANSI escape codes for compact Non-TTY / AI Agent output
pub fn strip_ansi_codes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for ch in chars.by_ref() {
                    if ch.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Option item for `@clack/prompts`-style interactive selector
#[derive(Debug, Clone)]
pub struct ClackOption {
    /// Primary label shown next to radio circle or checkbox
    pub label: String,
    /// Secondary muted hint shown beside the active/inactive option
    pub hint: String,
    /// Compact summary shown after selection is confirmed (`◇`)
    pub summary: String,
    /// Searchable text used when live-filtering by typing
    pub filter_text: String,
}

impl ClackOption {
    pub fn new(
        label: impl Into<String>,
        hint: impl Into<String>,
        summary: impl Into<String>,
    ) -> Self {
        let l = label.into();
        let h = hint.into();
        let s = summary.into();
        let filter_text = format!(
            "{} {} {}",
            strip_ansi_codes(&l),
            strip_ansi_codes(&h),
            strip_ansi_codes(&s)
        )
        .to_lowercase();
        Self {
            label: l,
            hint: h,
            summary: s,
            filter_text,
        }
    }
}

/// Render `@clack/prompts` intro header (`┌  badge  subtitle`)
/// In Non-TTY (Agent mode), prints a single token-compact line without box clutter.
pub fn clack_intro(badge: &str, subtitle: &str) {
    if io::stdout().is_terminal() {
        println!();
        println!(
            "  \x1b[90m{S_BAR_START}\x1b[0m  \x1b[46;30;1m {badge} \x1b[0m \x1b[90m{subtitle}\x1b[0m"
        );
        println!("  \x1b[90m{S_BAR}\x1b[0m");
    } else {
        println!(
            "[{}] {}",
            strip_ansi_codes(badge),
            strip_ansi_codes(subtitle)
        );
    }
}

/// Render `@clack/prompts` outro footer (`└  message`)
pub fn clack_outro(message: &str) {
    if io::stdout().is_terminal() {
        println!("  \x1b[90m{S_BAR_END}\x1b[0m  {message}");
        println!();
    } else {
        println!("[DONE] {}", strip_ansi_codes(message));
    }
}

/// Render `@clack/prompts` cancel footer (`└  message`)
pub fn clack_cancel(message: &str) {
    if io::stdout().is_terminal() {
        println!("  \x1b[90m{S_BAR_END}\x1b[0m  \x1b[31m{message}\x1b[0m");
        println!();
    } else {
        eprintln!("[CANCELLED] {}", strip_ansi_codes(message));
    }
}

/// Render a completed step along the Clack rail (`◇  message`)
pub fn clack_step_ok(message: impl std::fmt::Display) {
    let msg = message.to_string();
    if io::stdout().is_terminal() {
        println!("  \x1b[32m{S_STEP_SUBMIT}\x1b[0m  {msg}");
        println!("  \x1b[90m{S_BAR}\x1b[0m");
    } else {
        println!("[OK] {}", strip_ansi_codes(&msg));
    }
}

/// Render an informational step along the Clack rail (`◆  message`)
pub fn clack_step_info(message: impl std::fmt::Display) {
    let msg = message.to_string();
    if io::stdout().is_terminal() {
        println!("  \x1b[36m{S_STEP_ACTIVE}\x1b[0m  {msg}");
        println!("  \x1b[90m{S_BAR}\x1b[0m");
    } else {
        println!("[INFO] {}", strip_ansi_codes(&msg));
    }
}

/// Render a warning step along the Clack rail (`▲  message`)
pub fn clack_step_warn(message: impl std::fmt::Display) {
    let msg = message.to_string();
    if io::stderr().is_terminal() {
        eprintln!("  \x1b[33m{S_STEP_WARN}\x1b[0m  \x1b[33m{msg}\x1b[0m");
        eprintln!("  \x1b[90m{S_BAR}\x1b[0m");
    } else {
        eprintln!("[WARN] {}", strip_ansi_codes(&msg));
    }
}

/// Render an error step along the Clack rail (`■  message`)
pub fn clack_step_err(message: impl std::fmt::Display) {
    let msg = message.to_string();
    if io::stderr().is_terminal() {
        eprintln!("  \x1b[31m{S_STEP_CANCEL}\x1b[0m  \x1b[31m{msg}\x1b[0m");
        eprintln!("  \x1b[90m{S_BAR}\x1b[0m");
    } else {
        eprintln!("[ERROR] {}", strip_ansi_codes(&msg));
    }
}

/// Render a skipped step along the Clack rail (`○  message`)
pub fn clack_step_skip(message: impl std::fmt::Display) {
    let msg = message.to_string();
    if io::stdout().is_terminal() {
        println!("  \x1b[90m{S_STEP_SKIP}  {msg}\x1b[0m");
        println!("  \x1b[90m{S_BAR}\x1b[0m");
    } else {
        println!("[SKIP] {}", strip_ansi_codes(&msg));
    }
}

/// Render a boxed `@clack/prompts` note attached to the vertical rail
pub fn clack_note(title: &str, lines: &[impl AsRef<str>]) {
    if !io::stdout().is_terminal() {
        let joined: Vec<String> = lines
            .iter()
            .map(|l| strip_ansi_codes(l.as_ref()).trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        println!("[{}] {}", strip_ansi_codes(title), joined.join(" | "));
        return;
    }

    let title_w = visible_width(title);
    let max_line_w = lines
        .iter()
        .map(|l| visible_width(l.as_ref()))
        .max()
        .unwrap_or(0);
    let inner_w = title_w.max(max_line_w).max(28) + 4;

    let top_dashes = inner_w.saturating_sub(title_w + 3);
    println!(
        "  \x1b[36m{S_STEP_SUBMIT}\x1b[0m  \x1b[1;97m{title}\x1b[0m \x1b[90m{}╮\x1b[0m",
        "─".repeat(top_dashes)
    );
    for line in lines {
        let s = line.as_ref();
        let lw = visible_width(s);
        let pad = " ".repeat(inner_w.saturating_sub(lw + 2));
        println!("  \x1b[90m{S_BAR}\x1b[0m  {s}{pad}\x1b[90m│\x1b[0m");
    }
    println!("  \x1b[90m├{}╯\x1b[0m", "─".repeat(inner_w + 1));
    println!("  \x1b[90m{S_BAR}\x1b[0m");
}

/// Render a multi-line block (such as a Table or Card) indented along the Clack rail (`│`)
pub fn clack_block(rendered: &str) {
    if !io::stdout().is_terminal() {
        for line in rendered.lines() {
            let clean = strip_ansi_codes(line);
            let trimmed = clean.trim();
            if !trimmed.is_empty()
                && !trimmed.starts_with('╭')
                && !trimmed.starts_with('├')
                && !trimmed.starts_with('╰')
            {
                println!("{}", trimmed);
            }
        }
        return;
    }

    for line in rendered.lines() {
        println!("  \x1b[90m{S_BAR}\x1b[0m  {}", line.trim_start());
    }
    println!("  \x1b[90m{S_BAR}\x1b[0m");
}

/// RAII `@clack/prompts` animated spinner (`◒ ◐ ◓ ◑`)
/// Automatically cleans up terminal line on completion, failure, or drop.
/// In Non-TTY (Agent mode), emits zero animation frames to save tokens.
pub struct ClackSpinner {
    active: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
    start_time: Instant,
    is_tty: bool,
}

impl ClackSpinner {
    pub fn start(message: impl Into<String>) -> Self {
        let msg = message.into();
        let is_tty = io::stderr().is_terminal() && io::stdout().is_terminal();
        let active = Arc::new(AtomicBool::new(true));
        let start_time = Instant::now();

        let handle = if is_tty {
            let active_clone = Arc::clone(&active);
            let msg_clone = msg.clone();
            Some(std::thread::spawn(move || {
                let frames = ["◒", "◐", "◓", "◑"];
                let mut idx = 0usize;
                let _ = execute!(io::stderr(), cursor::Hide);
                while active_clone.load(Ordering::Relaxed) {
                    let frame = frames[idx % frames.len()];
                    eprint!("\r\x1b[2K  \x1b[35m{frame}\x1b[0m  \x1b[90m{msg_clone}\x1b[0m");
                    let _ = io::stderr().flush();
                    idx = idx.wrapping_add(1);
                    std::thread::sleep(Duration::from_millis(80));
                }
            }))
        } else {
            None
        };

        Self {
            active,
            handle,
            start_time,
            is_tty,
        }
    }

    fn stop_thread(&mut self) {
        if self.active.swap(false, Ordering::SeqCst) {
            if let Some(h) = self.handle.take() {
                let _ = h.join();
            }
            if self.is_tty {
                eprint!("\r\x1b[2K");
                let _ = io::stderr().flush();
                let _ = execute!(io::stderr(), cursor::Show);
            }
        }
    }

    /// Stop spinner and erase line silently (e.g. before opening an interactive selector)
    pub fn clear(mut self) {
        self.stop_thread();
    }

    /// Complete spinner with a `◇` success step along the Clack rail
    pub fn finish(mut self, message: impl std::fmt::Display) {
        let elapsed = self.start_time.elapsed();
        self.stop_thread();
        let time_badge = if elapsed.as_millis() >= 100 {
            format!(
                " {MUTED}({:.1}s){RESET}",
                elapsed.as_secs_f64(),
                MUTED = colors::MUTED,
                RESET = colors::RESET
            )
        } else {
            String::new()
        };
        clack_step_ok(format!("{}{}", message, time_badge));
    }

    /// Complete spinner with a `■` error step along the Clack rail
    pub fn fail(mut self, message: impl std::fmt::Display) {
        self.stop_thread();
        clack_step_err(message);
    }
}

impl Drop for ClackSpinner {
    fn drop(&mut self) {
        self.stop_thread();
    }
}

/// Interactive `@clack/prompts` single-select with live type-to-filter, viewport scrolling, and step collapse.
/// In Non-TTY (Agent mode), fails fast with available options instead of hanging on stdin.
pub fn clack_select(
    message: &str,
    options: &[ClackOption],
    default_idx: usize,
    max_visible: usize,
) -> io::Result<Option<usize>> {
    if options.is_empty() {
        return Ok(None);
    }

    // Non-TTY Zero-Hang Guard for AI Agents & CI pipelines
    if !is_interactive_tty() {
        let available: Vec<String> = options
            .iter()
            .take(15)
            .map(|o| strip_ansi_codes(&o.label))
            .collect();
        return Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            format!(
                "Non-interactive terminal: cannot prompt '{}'. Pass target explicitly via CLI arguments. Available options: [{}]",
                strip_ansi_codes(message),
                available.join(", ")
            ),
        ));
    }

    let mut stdout = io::stdout();
    let mut filter = String::new();
    let mut cursor_pos = default_idx.min(options.len().saturating_sub(1));
    let mut scroll_offset = 0usize;
    let max_rows = max_visible.max(5);

    terminal::enable_raw_mode()?;
    let _ = execute!(stdout, cursor::Hide);

    let mut rendered_lines = render_clack_select_frame(
        &mut stdout,
        message,
        options,
        &filter,
        &mut cursor_pos,
        &mut scroll_offset,
        max_rows,
    )?;

    let result: io::Result<Option<usize>> = loop {
        let ev = event::read()?;
        if let Event::Key(key) = ev {
            if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                continue;
            }

            let filtered_indices = compute_filtered_indices(options, &filter);
            let count = filtered_indices.len();

            match key.code {
                KeyCode::Esc => break Ok(None),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    break Ok(None);
                }
                KeyCode::Enter => {
                    if let Some(&orig_idx) = filtered_indices.get(cursor_pos) {
                        break Ok(Some(orig_idx));
                    }
                }
                KeyCode::Up | KeyCode::BackTab => {
                    if count > 0 {
                        if cursor_pos == 0 {
                            cursor_pos = count - 1;
                        } else {
                            cursor_pos -= 1;
                        }
                    }
                }
                KeyCode::Down | KeyCode::Tab => {
                    if count > 0 {
                        cursor_pos = (cursor_pos + 1) % count;
                    }
                }
                KeyCode::PageUp => {
                    if count > 0 {
                        cursor_pos = cursor_pos.saturating_sub(max_rows);
                    }
                }
                KeyCode::PageDown => {
                    if count > 0 {
                        cursor_pos = (cursor_pos + max_rows).min(count - 1);
                    }
                }
                KeyCode::Home => {
                    cursor_pos = 0;
                }
                KeyCode::End => {
                    if count > 0 {
                        cursor_pos = count - 1;
                    }
                }
                KeyCode::Backspace => {
                    filter.pop();
                    cursor_pos = 0;
                    scroll_offset = 0;
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    filter.clear();
                    cursor_pos = 0;
                    scroll_offset = 0;
                }
                KeyCode::Char(ch)
                    if !key.modifiers.contains(KeyModifiers::CONTROL)
                        && !key.modifiers.contains(KeyModifiers::ALT) =>
                {
                    filter.push(ch);
                    cursor_pos = 0;
                    scroll_offset = 0;
                }
                _ => continue,
            }

            clear_rendered_lines(&mut stdout, rendered_lines)?;
            rendered_lines = render_clack_select_frame(
                &mut stdout,
                message,
                options,
                &filter,
                &mut cursor_pos,
                &mut scroll_offset,
                max_rows,
            )?;
        }
    };

    clear_rendered_lines(&mut stdout, rendered_lines)?;
    let _ = execute!(stdout, cursor::Show);
    terminal::disable_raw_mode()?;
    platform_impl::ensure_normal_console_mode();

    match result {
        Ok(Some(idx)) => {
            let chosen = &options[idx];
            println!("  \x1b[32m{S_STEP_SUBMIT}\x1b[0m  \x1b[1m{message}\x1b[0m");
            println!(
                "  \x1b[90m{S_BAR}\x1b[0m  \x1b[36m{}\x1b[0m",
                chosen.summary
            );
            println!("  \x1b[90m{S_BAR}\x1b[0m");
            Ok(Some(idx))
        }
        Ok(None) => {
            println!("  \x1b[31m{S_STEP_CANCEL}\x1b[0m  \x1b[1m{message}\x1b[0m");
            println!("  \x1b[90m{S_BAR}\x1b[0m  \x1b[90;3mCancelled\x1b[0m");
            println!("  \x1b[90m{S_BAR}\x1b[0m");
            Ok(None)
        }
        Err(e) => Err(e),
    }
}

/// Interactive `@clack/prompts` multi-select with Spacebar toggle (`◼`/`◻`), Ctrl+A select-all, and live filter.
/// If user presses Enter without checking any box via Space, selects the currently focused item.
pub fn clack_multiselect(
    message: &str,
    options: &[ClackOption],
    initial_selected: &[usize],
    max_visible: usize,
) -> io::Result<Option<Vec<usize>>> {
    if options.is_empty() {
        return Ok(None);
    }

    if !is_interactive_tty() {
        let available: Vec<String> = options
            .iter()
            .take(15)
            .map(|o| strip_ansi_codes(&o.label))
            .collect();
        return Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            format!(
                "Non-interactive terminal: cannot prompt '{}'. Available options: [{}]",
                strip_ansi_codes(message),
                available.join(", ")
            ),
        ));
    }

    let mut stdout = io::stdout();
    let mut filter = String::new();
    let mut cursor_pos = 0usize;
    let mut scroll_offset = 0usize;
    let max_rows = max_visible.max(5);
    let mut checked: Vec<bool> = vec![false; options.len()];
    for &idx in initial_selected {
        if idx < checked.len() {
            checked[idx] = true;
        }
    }

    terminal::enable_raw_mode()?;
    let _ = execute!(stdout, cursor::Hide);

    let mut rendered_lines = render_clack_multiselect_frame(
        &mut stdout,
        message,
        options,
        &checked,
        &filter,
        &mut cursor_pos,
        &mut scroll_offset,
        max_rows,
    )?;

    let result: io::Result<Option<Vec<usize>>> = loop {
        let ev = event::read()?;
        if let Event::Key(key) = ev {
            if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                continue;
            }

            let filtered_indices = compute_filtered_indices(options, &filter);
            let count = filtered_indices.len();

            match key.code {
                KeyCode::Esc => break Ok(None),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    break Ok(None);
                }
                KeyCode::Enter => {
                    let mut chosen: Vec<usize> = checked
                        .iter()
                        .enumerate()
                        .filter_map(|(i, &is_on)| if is_on { Some(i) } else { None })
                        .collect();
                    if chosen.is_empty() {
                        if let Some(&orig_idx) = filtered_indices.get(cursor_pos) {
                            chosen.push(orig_idx);
                        }
                    }
                    if !chosen.is_empty() {
                        break Ok(Some(chosen));
                    }
                }
                KeyCode::Char(' ') => {
                    if let Some(&orig_idx) = filtered_indices.get(cursor_pos) {
                        checked[orig_idx] = !checked[orig_idx];
                    }
                }
                KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    let all_filtered_checked = filtered_indices.iter().all(|&i| checked[i]);
                    for &i in &filtered_indices {
                        checked[i] = !all_filtered_checked;
                    }
                }
                KeyCode::Up | KeyCode::BackTab => {
                    if count > 0 {
                        if cursor_pos == 0 {
                            cursor_pos = count - 1;
                        } else {
                            cursor_pos -= 1;
                        }
                    }
                }
                KeyCode::Down | KeyCode::Tab => {
                    if count > 0 {
                        cursor_pos = (cursor_pos + 1) % count;
                    }
                }
                KeyCode::PageUp => {
                    if count > 0 {
                        cursor_pos = cursor_pos.saturating_sub(max_rows);
                    }
                }
                KeyCode::PageDown => {
                    if count > 0 {
                        cursor_pos = (cursor_pos + max_rows).min(count - 1);
                    }
                }
                KeyCode::Backspace => {
                    filter.pop();
                    cursor_pos = 0;
                    scroll_offset = 0;
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    filter.clear();
                    cursor_pos = 0;
                    scroll_offset = 0;
                }
                KeyCode::Char(ch)
                    if !key.modifiers.contains(KeyModifiers::CONTROL)
                        && !key.modifiers.contains(KeyModifiers::ALT) =>
                {
                    filter.push(ch);
                    cursor_pos = 0;
                    scroll_offset = 0;
                }
                _ => continue,
            }

            clear_rendered_lines(&mut stdout, rendered_lines)?;
            rendered_lines = render_clack_multiselect_frame(
                &mut stdout,
                message,
                options,
                &checked,
                &filter,
                &mut cursor_pos,
                &mut scroll_offset,
                max_rows,
            )?;
        }
    };

    clear_rendered_lines(&mut stdout, rendered_lines)?;
    let _ = execute!(stdout, cursor::Show);
    terminal::disable_raw_mode()?;
    platform_impl::ensure_normal_console_mode();

    match result {
        Ok(Some(indices)) => {
            let summaries: Vec<&str> = indices
                .iter()
                .map(|&i| options[i].summary.as_str())
                .collect();
            println!("  \x1b[32m{S_STEP_SUBMIT}\x1b[0m  \x1b[1m{message}\x1b[0m");
            println!(
                "  \x1b[90m{S_BAR}\x1b[0m  \x1b[36m{}\x1b[0m \x1b[90m({} selected)\x1b[0m",
                summaries.join(", "),
                indices.len()
            );
            println!("  \x1b[90m{S_BAR}\x1b[0m");
            Ok(Some(indices))
        }
        Ok(None) => {
            println!("  \x1b[31m{S_STEP_CANCEL}\x1b[0m  \x1b[1m{message}\x1b[0m");
            println!("  \x1b[90m{S_BAR}\x1b[0m  \x1b[90;3mCancelled\x1b[0m");
            println!("  \x1b[90m{S_BAR}\x1b[0m");
            Ok(None)
        }
        Err(e) => Err(e),
    }
}

/// Interactive `@clack/prompts` text input along the vertical rail (`│`)
pub fn clack_text(
    message: &str,
    placeholder: Option<&str>,
    default_val: Option<&str>,
) -> io::Result<Option<String>> {
    if !is_interactive_tty() {
        if let Some(def) = default_val {
            return Ok(Some(def.to_string()));
        }
        return Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            format!(
                "Non-interactive terminal: missing required input for '{}'. Pass value via CLI flag.",
                strip_ansi_codes(message)
            ),
        ));
    }

    let mut stdout = io::stdout();
    let mut value = String::new();
    let hint_placeholder = placeholder.or(default_val).unwrap_or("");

    terminal::enable_raw_mode()?;
    let _ = execute!(stdout, cursor::Hide);

    let mut rendered_lines =
        render_clack_text_frame(&mut stdout, message, &value, hint_placeholder, false)?;

    let result: io::Result<Option<String>> = loop {
        let ev = event::read()?;
        if let Event::Key(key) = ev {
            if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                continue;
            }
            match key.code {
                KeyCode::Esc => break Ok(None),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    break Ok(None);
                }
                KeyCode::Enter => {
                    let trimmed = value.trim().to_string();
                    if trimmed.is_empty() {
                        if let Some(def) = default_val {
                            break Ok(Some(def.to_string()));
                        }
                    }
                    break Ok(Some(trimmed));
                }
                KeyCode::Backspace => {
                    value.pop();
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    value.clear();
                }
                KeyCode::Char(ch)
                    if !key.modifiers.contains(KeyModifiers::CONTROL)
                        && !key.modifiers.contains(KeyModifiers::ALT) =>
                {
                    value.push(ch);
                }
                _ => continue,
            }

            clear_rendered_lines(&mut stdout, rendered_lines)?;
            rendered_lines =
                render_clack_text_frame(&mut stdout, message, &value, hint_placeholder, false)?;
        }
    };

    clear_rendered_lines(&mut stdout, rendered_lines)?;
    let _ = execute!(stdout, cursor::Show);
    terminal::disable_raw_mode()?;
    platform_impl::ensure_normal_console_mode();

    match result {
        Ok(Some(val)) => {
            let display_val = if val.is_empty() {
                "(empty)"
            } else {
                val.as_str()
            };
            println!("  \x1b[32m{S_STEP_SUBMIT}\x1b[0m  \x1b[1m{message}\x1b[0m");
            println!("  \x1b[90m{S_BAR}\x1b[0m  \x1b[36m{display_val}\x1b[0m");
            println!("  \x1b[90m{S_BAR}\x1b[0m");
            Ok(Some(val))
        }
        Ok(None) => {
            println!("  \x1b[31m{S_STEP_CANCEL}\x1b[0m  \x1b[1m{message}\x1b[0m");
            println!("  \x1b[90m{S_BAR}\x1b[0m  \x1b[90;3mCancelled\x1b[0m");
            println!("  \x1b[90m{S_BAR}\x1b[0m");
            Ok(None)
        }
        Err(e) => Err(e),
    }
}

/// Interactive `@clack/prompts` password input masked with `▪` along the vertical rail (`│`)
pub fn clack_password(message: &str) -> io::Result<Option<String>> {
    if !is_interactive_tty() {
        return Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            format!(
                "Non-interactive terminal: missing required secret input for '{}'. Pass via CLI flag or env.",
                strip_ansi_codes(message)
            ),
        ));
    }

    let mut stdout = io::stdout();
    let mut value = String::new();

    terminal::enable_raw_mode()?;
    let _ = execute!(stdout, cursor::Hide);

    let mut rendered_lines = render_clack_text_frame(&mut stdout, message, &value, "", true)?;

    let result: io::Result<Option<String>> = loop {
        let ev = event::read()?;
        if let Event::Key(key) = ev {
            if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                continue;
            }
            match key.code {
                KeyCode::Esc => break Ok(None),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    break Ok(None);
                }
                KeyCode::Enter => {
                    break Ok(Some(value.trim().to_string()));
                }
                KeyCode::Backspace => {
                    value.pop();
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    value.clear();
                }
                KeyCode::Char(ch)
                    if !key.modifiers.contains(KeyModifiers::CONTROL)
                        && !key.modifiers.contains(KeyModifiers::ALT) =>
                {
                    value.push(ch);
                }
                _ => continue,
            }

            clear_rendered_lines(&mut stdout, rendered_lines)?;
            rendered_lines = render_clack_text_frame(&mut stdout, message, &value, "", true)?;
        }
    };

    clear_rendered_lines(&mut stdout, rendered_lines)?;
    let _ = execute!(stdout, cursor::Show);
    terminal::disable_raw_mode()?;
    platform_impl::ensure_normal_console_mode();

    match result {
        Ok(Some(val)) => {
            let masked = S_PASSWORD_MASK
                .to_string()
                .repeat(val.chars().count().clamp(6, 12));
            println!("  \x1b[32m{S_STEP_SUBMIT}\x1b[0m  \x1b[1m{message}\x1b[0m");
            println!("  \x1b[90m{S_BAR}\x1b[0m  \x1b[90m{masked}\x1b[0m");
            println!("  \x1b[90m{S_BAR}\x1b[0m");
            Ok(Some(val))
        }
        Ok(None) => {
            println!("  \x1b[31m{S_STEP_CANCEL}\x1b[0m  \x1b[1m{message}\x1b[0m");
            println!("  \x1b[90m{S_BAR}\x1b[0m  \x1b[90;3mCancelled\x1b[0m");
            println!("  \x1b[90m{S_BAR}\x1b[0m");
            Ok(None)
        }
        Err(e) => Err(e),
    }
}

/// Interactive `@clack/prompts` Yes/No confirmation prompt (`● Yes / ○ No`)
pub fn clack_confirm(message: &str, default_yes: bool) -> io::Result<Option<bool>> {
    if !is_interactive_tty() {
        return Ok(Some(default_yes));
    }

    let mut stdout = io::stdout();
    let mut active_yes = default_yes;

    terminal::enable_raw_mode()?;
    let _ = execute!(stdout, cursor::Hide);

    let mut rendered_lines = render_clack_confirm_frame(&mut stdout, message, active_yes)?;

    let result: io::Result<Option<bool>> = loop {
        let ev = event::read()?;
        if let Event::Key(key) = ev {
            if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                continue;
            }
            match key.code {
                KeyCode::Esc => break Ok(None),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    break Ok(None);
                }
                KeyCode::Enter => break Ok(Some(active_yes)),
                KeyCode::Left
                | KeyCode::Right
                | KeyCode::Up
                | KeyCode::Down
                | KeyCode::Tab
                | KeyCode::BackTab => {
                    active_yes = !active_yes;
                }
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    break Ok(Some(true));
                }
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    break Ok(Some(false));
                }
                _ => continue,
            }

            clear_rendered_lines(&mut stdout, rendered_lines)?;
            rendered_lines = render_clack_confirm_frame(&mut stdout, message, active_yes)?;
        }
    };

    clear_rendered_lines(&mut stdout, rendered_lines)?;
    let _ = execute!(stdout, cursor::Show);
    terminal::disable_raw_mode()?;
    platform_impl::ensure_normal_console_mode();

    match result {
        Ok(Some(yes)) => {
            let label = if yes { "Yes" } else { "No" };
            println!("  \x1b[32m{S_STEP_SUBMIT}\x1b[0m  \x1b[1m{message}\x1b[0m");
            println!("  \x1b[90m{S_BAR}\x1b[0m  \x1b[36m{label}\x1b[0m");
            println!("  \x1b[90m{S_BAR}\x1b[0m");
            Ok(Some(yes))
        }
        Ok(None) => {
            println!("  \x1b[31m{S_STEP_CANCEL}\x1b[0m  \x1b[1m{message}\x1b[0m");
            println!("  \x1b[90m{S_BAR}\x1b[0m  \x1b[90;3mCancelled\x1b[0m");
            println!("  \x1b[90m{S_BAR}\x1b[0m");
            Ok(None)
        }
        Err(e) => Err(e),
    }
}

fn render_clack_text_frame(
    stdout: &mut io::Stdout,
    message: &str,
    value: &str,
    placeholder: &str,
    is_password: bool,
) -> io::Result<usize> {
    let mut lines: Vec<String> = Vec::with_capacity(4);
    lines.push(format!(
        "  \x1b[36m{S_STEP_ACTIVE}\x1b[0m  \x1b[1;97m{message}\x1b[0m"
    ));

    let input_display = if value.is_empty() {
        if placeholder.is_empty() {
            "\x1b[36m▏\x1b[0m".to_string()
        } else {
            format!("\x1b[36m▏\x1b[90m{placeholder}\x1b[0m")
        }
    } else if is_password {
        let masked = S_PASSWORD_MASK.to_string().repeat(value.chars().count());
        format!("\x1b[1;97m{masked}\x1b[36m▏\x1b[0m")
    } else {
        format!("\x1b[1;97m{value}\x1b[36m▏\x1b[0m")
    };

    lines.push(format!("  \x1b[36m{S_BAR}\x1b[0m  {input_display}"));
    lines.push(format!("  \x1b[36m{S_BAR_END}\x1b[0m"));

    for line in &lines {
        write!(stdout, "{}\r\n", line)?;
    }
    stdout.flush()?;
    Ok(lines.len())
}

fn render_clack_confirm_frame(
    stdout: &mut io::Stdout,
    message: &str,
    active_yes: bool,
) -> io::Result<usize> {
    let mut lines: Vec<String> = Vec::with_capacity(4);
    lines.push(format!(
        "  \x1b[36m{S_STEP_ACTIVE}\x1b[0m  \x1b[1;97m{message}\x1b[0m"
    ));

    let toggle = if active_yes {
        format!(
            "\x1b[32m{S_RADIO_ACTIVE}\x1b[0m \x1b[1;97mYes\x1b[0m   \x1b[90m/   {S_RADIO_INACTIVE} No\x1b[0m"
        )
    } else {
        format!(
            "\x1b[90m{S_RADIO_INACTIVE} Yes   /\x1b[0m   \x1b[32m{S_RADIO_ACTIVE}\x1b[0m \x1b[1;97mNo\x1b[0m"
        )
    };

    lines.push(format!("  \x1b[36m{S_BAR}\x1b[0m  {toggle}"));
    lines.push(format!("  \x1b[36m{S_BAR_END}\x1b[0m"));

    for line in &lines {
        write!(stdout, "{}\r\n", line)?;
    }
    stdout.flush()?;
    Ok(lines.len())
}

fn compute_filtered_indices(options: &[ClackOption], filter: &str) -> Vec<usize> {
    let q = filter.trim().to_lowercase();
    if q.is_empty() {
        return (0..options.len()).collect();
    }
    let tokens: Vec<&str> = q.split_whitespace().collect();
    options
        .iter()
        .enumerate()
        .filter(|(_, opt)| tokens.iter().all(|t| opt.filter_text.contains(t)))
        .map(|(i, _)| i)
        .collect()
}

fn clear_rendered_lines(stdout: &mut io::Stdout, lines: usize) -> io::Result<()> {
    if lines > 0 {
        execute!(
            stdout,
            cursor::MoveToColumn(0),
            cursor::MoveUp(lines as u16),
            Clear(ClearType::FromCursorDown)
        )?;
    }
    Ok(())
}

fn render_clack_select_frame(
    stdout: &mut io::Stdout,
    message: &str,
    options: &[ClackOption],
    filter: &str,
    cursor_pos: &mut usize,
    scroll_offset: &mut usize,
    max_rows: usize,
) -> io::Result<usize> {
    let term_cols = terminal::size().map(|(c, _)| c as usize).unwrap_or(100);
    let max_line_width = term_cols.saturating_sub(4).max(40);

    let filtered = compute_filtered_indices(options, filter);
    let count = filtered.len();

    if count == 0 {
        *cursor_pos = 0;
        *scroll_offset = 0;
    } else {
        if *cursor_pos >= count {
            *cursor_pos = count - 1;
        }
        if *cursor_pos < *scroll_offset {
            *scroll_offset = *cursor_pos;
        } else if *cursor_pos >= *scroll_offset + max_rows {
            *scroll_offset = *cursor_pos + 1 - max_rows;
        }
    }

    let mut lines: Vec<String> = Vec::new();

    // 1. Active Step Header (`◆  message`)
    lines.push(format!(
        "  \x1b[36m{S_STEP_ACTIVE}\x1b[0m  \x1b[1;97m{message}\x1b[0m"
    ));

    // 2. Filter / Search bar when list is scrollable or filter is active
    if !filter.is_empty() {
        lines.push(format!(
            "  \x1b[36m{S_BAR}\x1b[0m  \x1b[36mFilter:\x1b[0m \x1b[1;97m{filter}\x1b[36m▏\x1b[0m \x1b[90m({count}/{} matched)\x1b[0m",
            options.len()
        ));
    } else if options.len() > max_rows {
        lines.push(format!(
            "  \x1b[36m{S_BAR}\x1b[0m  \x1b[90mType to search/filter... ({}/{})\x1b[0m",
            if count > 0 { *cursor_pos + 1 } else { 0 },
            count
        ));
    }

    // 3. Option rows
    if count == 0 {
        lines.push(format!(
            "  \x1b[36m{S_BAR}\x1b[0m  \x1b[33mNo matches for '{filter}' (press Backspace)\x1b[0m"
        ));
    } else {
        let end = (*scroll_offset + max_rows).min(count);
        let visible_slice = &filtered[*scroll_offset..end];
        let label_pad = visible_slice
            .iter()
            .map(|&idx| visible_width(&options[idx].label))
            .max()
            .unwrap_or(12)
            .clamp(12, 34);

        for (rel_i, &orig_idx) in visible_slice.iter().enumerate() {
            let abs_i = *scroll_offset + rel_i;
            let is_active = abs_i == *cursor_pos;
            let opt = &options[orig_idx];

            let lw = visible_width(&opt.label);
            let pad = " ".repeat(label_pad.saturating_sub(lw) + 2);

            let used_cols = 8 + lw + pad.len();
            let avail_hint = max_line_width.saturating_sub(used_cols);
            let hint_str = truncate_plain(&opt.hint, avail_hint);

            if is_active {
                let hint_part = if hint_str.is_empty() {
                    String::new()
                } else {
                    format!("{pad}\x1b[36m{hint_str}\x1b[0m")
                };
                lines.push(format!(
                    "  \x1b[36m{S_BAR}\x1b[0m  \x1b[32m{S_RADIO_ACTIVE}\x1b[0m \x1b[1;97m{}\x1b[0m{hint_part}",
                    opt.label
                ));
            } else {
                let hint_part = if hint_str.is_empty() {
                    String::new()
                } else {
                    format!("{pad}\x1b[90m{hint_str}\x1b[0m")
                };
                lines.push(format!(
                    "  \x1b[36m{S_BAR}\x1b[0m  \x1b[90m{S_RADIO_INACTIVE} {}\x1b[0m{hint_part}",
                    opt.label
                ));
            }
        }
    }

    // 4. Keybinding helper bar & bottom cap (`└`)
    let scroll_badge = if count > max_rows {
        format!(" • {}/{}", *cursor_pos + 1, count)
    } else {
        String::new()
    };
    lines.push(format!(
        "  \x1b[36m{S_BAR}\x1b[0m  \x1b[90m↑/↓ move • type to filter • Enter select • Esc cancel{scroll_badge}\x1b[0m"
    ));
    lines.push(format!("  \x1b[36m{S_BAR_END}\x1b[0m"));

    for line in &lines {
        write!(stdout, "{}\r\n", line)?;
    }
    stdout.flush()?;

    Ok(lines.len())
}

#[allow(clippy::too_many_arguments)]
fn render_clack_multiselect_frame(
    stdout: &mut io::Stdout,
    message: &str,
    options: &[ClackOption],
    checked: &[bool],
    filter: &str,
    cursor_pos: &mut usize,
    scroll_offset: &mut usize,
    max_rows: usize,
) -> io::Result<usize> {
    let term_cols = terminal::size().map(|(c, _)| c as usize).unwrap_or(100);
    let max_line_width = term_cols.saturating_sub(4).max(40);

    let filtered = compute_filtered_indices(options, filter);
    let count = filtered.len();
    let total_checked = checked.iter().filter(|&&c| c).count();

    if count == 0 {
        *cursor_pos = 0;
        *scroll_offset = 0;
    } else {
        if *cursor_pos >= count {
            *cursor_pos = count - 1;
        }
        if *cursor_pos < *scroll_offset {
            *scroll_offset = *cursor_pos;
        } else if *cursor_pos >= *scroll_offset + max_rows {
            *scroll_offset = *cursor_pos + 1 - max_rows;
        }
    }

    let mut lines: Vec<String> = Vec::new();

    lines.push(format!(
        "  \x1b[36m{S_STEP_ACTIVE}\x1b[0m  \x1b[1;97m{message}\x1b[0m \x1b[90m({total_checked} checked)\x1b[0m"
    ));

    if !filter.is_empty() {
        lines.push(format!(
            "  \x1b[36m{S_BAR}\x1b[0m  \x1b[36mFilter:\x1b[0m \x1b[1;97m{filter}\x1b[36m▏\x1b[0m \x1b[90m({count}/{} matched)\x1b[0m",
            options.len()
        ));
    }

    if count == 0 {
        lines.push(format!(
            "  \x1b[36m{S_BAR}\x1b[0m  \x1b[33mNo matches for '{filter}' (press Backspace)\x1b[0m"
        ));
    } else {
        let end = (*scroll_offset + max_rows).min(count);
        let visible_slice = &filtered[*scroll_offset..end];
        let label_pad = visible_slice
            .iter()
            .map(|&idx| visible_width(&options[idx].label))
            .max()
            .unwrap_or(12)
            .clamp(12, 34);

        for (rel_i, &orig_idx) in visible_slice.iter().enumerate() {
            let abs_i = *scroll_offset + rel_i;
            let is_active = abs_i == *cursor_pos;
            let is_checked = checked[orig_idx];
            let opt = &options[orig_idx];

            let lw = visible_width(&opt.label);
            let pad = " ".repeat(label_pad.saturating_sub(lw) + 2);
            let used_cols = 8 + lw + pad.len();
            let avail_hint = max_line_width.saturating_sub(used_cols);
            let hint_str = truncate_plain(&opt.hint, avail_hint);

            let box_icon = if is_checked {
                format!("\x1b[32m{S_CHECK_ACTIVE}\x1b[0m")
            } else if is_active {
                format!("\x1b[36m{S_CHECK_INACTIVE}\x1b[0m")
            } else {
                format!("\x1b[90m{S_CHECK_INACTIVE}\x1b[0m")
            };

            if is_active {
                let hint_part = if hint_str.is_empty() {
                    String::new()
                } else {
                    format!("{pad}\x1b[36m{hint_str}\x1b[0m")
                };
                lines.push(format!(
                    "  \x1b[36m{S_BAR}\x1b[0m  {box_icon} \x1b[1;97m{}\x1b[0m{hint_part}",
                    opt.label
                ));
            } else {
                let hint_part = if hint_str.is_empty() {
                    String::new()
                } else {
                    format!("{pad}\x1b[90m{hint_str}\x1b[0m")
                };
                let label_style = if is_checked { "\x1b[97m" } else { "\x1b[90m" };
                lines.push(format!(
                    "  \x1b[36m{S_BAR}\x1b[0m  {box_icon} {label_style}{}\x1b[0m{hint_part}",
                    opt.label
                ));
            }
        }
    }

    lines.push(format!(
        "  \x1b[36m{S_BAR}\x1b[0m  \x1b[90m↑/↓ move • Space toggle • Ctrl+A all • Enter confirm • Esc cancel\x1b[0m"
    ));
    lines.push(format!("  \x1b[36m{S_BAR_END}\x1b[0m"));

    for line in &lines {
        write!(stdout, "{}\r\n", line)?;
    }
    stdout.flush()?;

    Ok(lines.len())
}

fn truncate_plain(s: &str, max_chars: usize) -> String {
    if max_chars < 4 {
        return String::new();
    }
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

/// Prompt user for standard text input (delegates to `@clack/prompts` `clack_text` on TTY)
pub fn prompt_input(label: &str, default: Option<&str>) -> io::Result<String> {
    match clack_text(label, default, default)? {
        Some(val) => Ok(val),
        None => Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "Input cancelled by user",
        )),
    }
}

/// Prompt user for password input (delegates to `@clack/prompts` `clack_password` on TTY)
pub fn prompt_password(label: &str) -> io::Result<String> {
    match clack_password(label)? {
        Some(val) => Ok(val),
        None => Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "Password input cancelled by user",
        )),
    }
}

/// Display an interactive choice list (`@clack/prompts` style on TTY, zero-hang error on Non-TTY)
pub fn prompt_select(label: &str, options: &[&str], default_idx: usize) -> io::Result<usize> {
    let clack_opts: Vec<ClackOption> = options
        .iter()
        .map(|&o| ClackOption::new(o, "", o))
        .collect();
    match clack_select(label, &clack_opts, default_idx, 10)? {
        Some(idx) => Ok(idx),
        None => Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "Selection cancelled by user",
        )),
    }
}

#[cfg(windows)]
#[allow(clippy::upper_case_acronyms)]
mod platform_impl {
    type HANDLE = *mut std::ffi::c_void;
    type BOOL = i32;
    type DWORD = u32;

    const STD_INPUT_HANDLE: DWORD = 0xFFFFFFF6; // ((DWORD)-10)
    const ENABLE_PROCESSED_INPUT: DWORD = 0x0001;
    const ENABLE_LINE_INPUT: DWORD = 0x0002;
    const ENABLE_ECHO_INPUT: DWORD = 0x0004;

    unsafe extern "system" {
        fn GetStdHandle(nStdHandle: DWORD) -> HANDLE;
        fn GetConsoleMode(hConsoleHandle: HANDLE, lpMode: *mut DWORD) -> BOOL;
        fn SetConsoleMode(hConsoleHandle: HANDLE, dwMode: DWORD) -> BOOL;
    }

    pub fn ensure_normal_console_mode() {
        unsafe {
            let handle = GetStdHandle(STD_INPUT_HANDLE);
            if !handle.is_null() && handle as isize != -1 {
                let mut mode: DWORD = 0;
                if GetConsoleMode(handle, &mut mode) != 0 {
                    SetConsoleMode(
                        handle,
                        mode | ENABLE_PROCESSED_INPUT | ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT,
                    );
                }
            }
        }
    }
}

#[cfg(not(windows))]
mod platform_impl {
    pub fn ensure_normal_console_mode() {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_ansi_codes() {
        let input = "\x1b[32m● ONLINE\x1b[0m \x1b[1;97mmy-vps\x1b[0m";
        assert_eq!(strip_ansi_codes(input), "● ONLINE my-vps");
    }

    #[test]
    fn test_compute_filtered_indices() {
        let opts = vec![
            ClackOption::new("my-vps", "1080", "my-vps"),
            ClackOption::new("my-pc", "disabled", "my-pc"),
        ];
        assert_eq!(compute_filtered_indices(&opts, "vps"), vec![0]);
        assert_eq!(compute_filtered_indices(&opts, "1080"), vec![0]);
        assert_eq!(compute_filtered_indices(&opts, ""), vec![0, 1]);
    }
}
