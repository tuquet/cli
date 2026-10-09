use nu_ansi_term::{Color, Style};
use reedline::{Highlighter, StyledText};

use super::scope::ShellScope;

pub struct SpecterHighlighter {
    pub scope: ShellScope,
}

impl Highlighter for SpecterHighlighter {
    fn highlight(&self, line: &str, _cursor: usize) -> StyledText {
        let mut styled = StyledText::new();
        if line.is_empty() {
            return styled;
        }

        let leading_spaces = line.len() - line.trim_start().len();
        if leading_spaces > 0 {
            styled.push((Style::new(), line[..leading_spaces].to_string()));
        }

        let mut remaining = &line[leading_spaces..];
        let mut first_word: Option<&str> = None;
        let mut word_index = 0;

        while !remaining.is_empty() {
            let next_word_end = remaining.find(char::is_whitespace).unwrap_or(remaining.len());
            let word = &remaining[..next_word_end];

            if word_index == 0 {
                first_word = Some(word);
                if self.scope.is_valid_first_word(word) {
                    if word.eq_ignore_ascii_case("use") {
                        styled.push((Color::Cyan.bold(), word.to_string()));
                    } else if ShellScope::from_name(word).is_some() {
                        styled.push((Color::Magenta.bold(), word.to_string()));
                    } else {
                        styled.push((Color::Cyan.bold(), word.to_string()));
                    }
                } else {
                    styled.push((Color::Red.normal(), word.to_string()));
                }
            } else if word_index == 1 && first_word.map(|w| w.eq_ignore_ascii_case("use")).unwrap_or(false) {
                // Second word after `use <scope>`
                if ShellScope::from_name(word).is_some() {
                    styled.push((Color::Magenta.bold(), word.to_string()));
                } else {
                    styled.push((Color::Red.normal(), word.to_string()));
                }
            } else if word_index == 1 && self.scope.is_scope_prefix(first_word.unwrap_or("")) {
                // Second word after scope prefix redundancy: e.g. "bridge status", "automa run"
                if self.scope.is_valid_subcommand(word) {
                    styled.push((Color::Cyan.bold(), word.to_string()));
                } else {
                    styled.push((Color::Red.normal(), word.to_string()));
                }
            } else if word_index == 1 && self.scope == ShellScope::Global && ShellScope::from_name(first_word.unwrap_or("")).is_some() {
                // Second word after scope name in Global: e.g. "bridge status", "automa run"
                if let Some(target_scope) = ShellScope::from_name(first_word.unwrap_or("")) {
                    if target_scope.is_valid_subcommand(word) {
                        styled.push((Color::Cyan.bold(), word.to_string()));
                    } else {
                        styled.push((Color::Red.normal(), word.to_string()));
                    }
                } else {
                    styled.push((Color::White.normal(), word.to_string()));
                }
            } else if word.starts_with('-') {
                styled.push((Color::Yellow.normal(), word.to_string()));
            } else if word.starts_with('@') {
                styled.push((Color::Magenta.normal(), word.to_string()));
            } else if word.starts_with('"') || word.starts_with('\'') {
                styled.push((Color::Green.normal(), word.to_string()));
            } else if word.chars().all(|c| c.is_ascii_digit()) {
                styled.push((Color::Cyan.normal(), word.to_string()));
            } else if word.eq_ignore_ascii_case("all") {
                styled.push((Color::Green.bold(), word.to_string()));
            } else {
                styled.push((Color::White.normal(), word.to_string()));
            }

            word_index += 1;
            remaining = &remaining[next_word_end..];
            let ws_len = remaining.len() - remaining.trim_start().len();
            if ws_len > 0 {
                styled.push((Style::new(), remaining[..ws_len].to_string()));
                remaining = &remaining[ws_len..];
            }
        }

        styled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlighter_valid_and_invalid_first_words() {
        let hl = SpecterHighlighter { scope: ShellScope::Bridge };

        // Valid subcommand "status"
        let styled = hl.highlight("status", 0);
        assert_eq!(styled.buffer.len(), 1);
        assert_eq!(styled.buffer[0].1, "status");
        assert_eq!(styled.buffer[0].0.foreground, Some(Color::Cyan));

        // Invalid command "badcmd"
        let styled = hl.highlight("badcmd", 0);
        assert_eq!(styled.buffer.len(), 1);
        assert_eq!(styled.buffer[0].1, "badcmd");
        assert_eq!(styled.buffer[0].0.foreground, Some(Color::Red));
    }

    #[test]
    fn test_highlighter_use_scope_arguments() {
        let hl = SpecterHighlighter { scope: ShellScope::Bridge };

        // "use automa" -> 'use' (cyan), ' ' (default), 'automa' (magenta)
        let styled = hl.highlight("use automa", 0);
        assert_eq!(styled.buffer.len(), 3);
        assert_eq!(styled.buffer[0].1, "use");
        assert_eq!(styled.buffer[0].0.foreground, Some(Color::Cyan));
        assert_eq!(styled.buffer[2].1, "automa");
        assert_eq!(styled.buffer[2].0.foreground, Some(Color::Magenta));

        // "use invalid_scope" -> 'use' (cyan), ' ' (default), 'invalid_scope' (red)
        let styled_err = hl.highlight("use invalid_scope", 0);
        assert_eq!(styled_err.buffer.len(), 3);
        assert_eq!(styled_err.buffer[0].1, "use");
        assert_eq!(styled_err.buffer[2].1, "invalid_scope");
        assert_eq!(styled_err.buffer[2].0.foreground, Some(Color::Red));
    }

    #[test]
    fn test_highlighter_prefix_redundancy() {
        let hl = SpecterHighlighter { scope: ShellScope::Bridge };

        // "bridge status" in Bridge scope -> 'bridge' (magenta), ' ' (default), 'status' (cyan)
        let styled = hl.highlight("bridge status", 0);
        assert_eq!(styled.buffer.len(), 3);
        assert_eq!(styled.buffer[0].1, "bridge");
        assert_eq!(styled.buffer[0].0.foreground, Some(Color::Magenta));
        assert_eq!(styled.buffer[2].1, "status");
        assert_eq!(styled.buffer[2].0.foreground, Some(Color::Cyan));

        // "bridge badsubcmd" in Bridge scope -> 'bridge' (magenta), ' ' (default), 'badsubcmd' (red)
        let styled_err = hl.highlight("bridge badsubcmd", 0);
        assert_eq!(styled_err.buffer.len(), 3);
        assert_eq!(styled_err.buffer[2].1, "badsubcmd");
        assert_eq!(styled_err.buffer[2].0.foreground, Some(Color::Red));
    }
}
