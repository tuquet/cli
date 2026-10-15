use std::io::{self, IsTerminal, Write};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{self, Clear, ClearType},
};
use super::colors::visible_width;

// @clack/prompts Unicode Box-Drawing & State Symbols
const S_BAR_START: &str = "┌";
const S_BAR: &str = "│";
const S_BAR_END: &str = "└";
const S_STEP_ACTIVE: &str = "◆";
const S_STEP_SUBMIT: &str = "◇";
const S_STEP_CANCEL: &str = "■";
const S_RADIO_ACTIVE: &str = "●";
const S_RADIO_INACTIVE: &str = "○";

/// Option item for `@clack/prompts`-style interactive selector
#[derive(Debug, Clone)]
pub struct ClackOption {
    /// Primary label shown next to radio circle
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
        let filter_text = format!("{} {} {}", l, h, s).to_lowercase();
        Self {
            label: l,
            hint: h,
            summary: s,
            filter_text,
        }
    }
}

/// Render `@clack/prompts` intro header (`┌  badge  subtitle`)
pub fn clack_intro(badge: &str, subtitle: &str) {
    println!();
    println!(
        "  \x1b[90m{S_BAR_START}\x1b[0m  \x1b[46;30;1m {badge} \x1b[0m \x1b[90m{subtitle}\x1b[0m"
    );
    println!("  \x1b[90m{S_BAR}\x1b[0m");
}

/// Render `@clack/prompts` outro footer (`└  message`)
pub fn clack_outro(message: &str) {
    println!("  \x1b[90m{S_BAR_END}\x1b[0m  {message}");
    println!();
}

/// Render `@clack/prompts` cancel footer (`└  message`)
pub fn clack_cancel(message: &str) {
    println!("  \x1b[90m{S_BAR_END}\x1b[0m  \x1b[31m{message}\x1b[0m");
    println!();
}

/// Interactive `@clack/prompts` select with live type-to-filter, viewport scrolling, and step collapse
pub fn clack_select(
    message: &str,
    options: &[ClackOption],
    default_idx: usize,
    max_visible: usize,
) -> io::Result<Option<usize>> {
    if options.is_empty() {
        return Ok(None);
    }

    // Fallback for non-TTY pipes
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        let plain_opts: Vec<&str> = options.iter().map(|o| o.label.as_str()).collect();
        return prompt_select_fallback(message, &plain_opts, default_idx).map(Some);
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
            // Crucial on Windows: ignore KeyEventKind::Release so keys don't fire twice
            if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                continue;
            }

            let filtered_indices = compute_filtered_indices(options, &filter);
            let count = filtered_indices.len();

            match key.code {
                KeyCode::Esc => {
                    break Ok(None);
                }
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

    match result {
        Ok(Some(idx)) => {
            let chosen = &options[idx];
            println!("  \x1b[32m{S_STEP_SUBMIT}\x1b[0m  \x1b[1m{message}\x1b[0m");
            println!("  \x1b[90m{S_BAR}\x1b[0m  \x1b[36m{}\x1b[0m", chosen.summary);
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

    // 2. Filter / Search bar (`│  🔍 ...`) when list is scrollable or filter is active
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
        // Compute label column alignment across visible slice
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

/// Prompt user for standard text input with an optional default value
pub fn prompt_input(label: &str, default: Option<&str>) -> io::Result<String> {
    if let Some(def) = default {
        print!("  \x1b[1;36m?\x1b[0m {} \x1b[90m[{}]\x1b[0m: ", label, def);
    } else {
        print!("  \x1b[1;36m?\x1b[0m {}: ", label);
    }
    io::stdout().flush()?;

    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    let trimmed = line.trim().to_string();

    if trimmed.is_empty()
        && let Some(def) = default
    {
        Ok(def.to_string())
    } else {
        Ok(trimmed)
    }
}

/// Prompt user for password input with terminal echoing disabled
pub fn prompt_password(label: &str) -> io::Result<String> {
    print!("  \x1b[1;36m?\x1b[0m {}: ", label);
    io::stdout().flush()?;

    if !io::stdin().is_terminal() {
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        return Ok(line.trim().to_string());
    }

    let password = platform_impl::read_hidden()?;
    println!();
    Ok(password.trim().to_string())
}

/// Display an interactive choice list (`@clack/prompts` style on TTY, numbered fallback on pipes)
pub fn prompt_select(label: &str, options: &[&str], default_idx: usize) -> io::Result<usize> {
    if io::stdin().is_terminal() && io::stdout().is_terminal() {
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
    } else {
        prompt_select_fallback(label, options, default_idx)
    }
}

fn prompt_select_fallback(label: &str, options: &[&str], default_idx: usize) -> io::Result<usize> {
    println!("\n  \x1b[1;36m?\x1b[0m {}", label);
    for (i, opt) in options.iter().enumerate() {
        let marker = if i == default_idx {
            "\x1b[32m>\x1b[0m"
        } else {
            " "
        };
        println!("   {} \x1b[1m{})\x1b[0m {}", marker, i + 1, opt);
    }

    loop {
        print!("  Select [1-{}, default {}]: ", options.len(), default_idx + 1);
        io::stdout().flush()?;

        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        let trimmed = line.trim();

        if trimmed.is_empty() {
            return Ok(default_idx);
        }

        if let Ok(choice) = trimmed.parse::<usize>()
            && choice >= 1
            && choice <= options.len()
        {
            return Ok(choice - 1);
        }

        println!(
            "  \x1b[31mInvalid selection, please enter a number between 1 and {}.\x1b[0m",
            options.len()
        );
    }
}

#[cfg(windows)]
#[allow(clippy::upper_case_acronyms)]
mod platform_impl {
    use std::io::{self, BufRead};

    type HANDLE = *mut std::ffi::c_void;
    type BOOL = i32;
    type DWORD = u32;

    const STD_INPUT_HANDLE: DWORD = 0xFFFFFFF6; // ((DWORD)-10)
    const ENABLE_ECHO_INPUT: DWORD = 0x0004;

    unsafe extern "system" {
        fn GetStdHandle(nStdHandle: DWORD) -> HANDLE;
        fn GetConsoleMode(hConsoleHandle: HANDLE, lpMode: *mut DWORD) -> BOOL;
        fn SetConsoleMode(hConsoleHandle: HANDLE, dwMode: DWORD) -> BOOL;
    }

    pub fn read_hidden() -> io::Result<String> {
        unsafe {
            let handle = GetStdHandle(STD_INPUT_HANDLE);
            if handle.is_null() || handle as isize == -1 {
                let mut line = String::new();
                io::stdin().read_line(&mut line)?;
                return Ok(line);
            }

            let mut original_mode: DWORD = 0;
            if GetConsoleMode(handle, &mut original_mode) == 0 {
                let mut line = String::new();
                io::stdin().read_line(&mut line)?;
                return Ok(line);
            }

            // Disable echo input
            let new_mode = original_mode & !ENABLE_ECHO_INPUT;
            SetConsoleMode(handle, new_mode);

            let mut line = String::new();
            let stdin = io::stdin();
            let mut handle_reader = stdin.lock();
            let res = handle_reader.read_line(&mut line);

            // Restore original console mode
            SetConsoleMode(handle, original_mode);

            res.map(|_| line)
        }
    }
}

#[cfg(not(windows))]
mod platform_impl {
    use std::io;

    pub fn read_hidden() -> io::Result<String> {
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        Ok(line)
    }
}
