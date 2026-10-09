use std::borrow::Cow;
use reedline::{Prompt, PromptEditMode, PromptHistorySearch};

use crate::ui::colors;
use super::scope::ShellScope;

pub struct SpecterPrompt {
    pub scope: ShellScope,
    pub cloud_env: String,
}

impl Prompt for SpecterPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        let border = colors::BORDER;
        let cyan = colors::CYAN;
        let bold = colors::BOLD;
        let reset = colors::RESET;
        let muted = colors::MUTED;
        let green = colors::GREEN;

        let scope_str = self.scope.as_str();
        let scope_color = self.scope.color();

        let env_badge = if self.cloud_env.is_empty() {
            format!("{muted}○ cloud:local{reset}")
        } else {
            format!("{green}● {}{reset}", self.cloud_env)
        };

        let line = format!(
            "{border}╭─{reset} {bold}{cyan}⚡ specter{reset}  {env_badge}  {border}[{reset}{scope_color}{scope_str}{reset}{border}]{reset}\n{border}╰─{reset}"
        );
        Cow::Owned(line)
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, _edit_mode: PromptEditMode) -> Cow<'_, str> {
        Cow::Borrowed("\x1b[1;38;2;56;189;248m❯\x1b[0m ")
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed("\x1b[38;2;71;85;105m::: \x1b[0m")
    }

    fn render_prompt_history_search_indicator(&self, _history_search: PromptHistorySearch) -> Cow<'_, str> {
        Cow::Borrowed("\x1b[38;2;251;191;36m(search)\x1b[0m❯ ")
    }
}
