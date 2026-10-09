//! Unified Response Dispatcher & Helpers for Specter CLI
//! Standardizes JSON serialization, TTY presentation, and action envelopes.

use serde::Serialize;
use crate::ui::OutputFormat;

/// Trait for data models that can render themselves to the terminal
pub trait CliRender: Serialize {
    /// Render rich visual presentation (Cards, Tables, Box borders) to terminal
    fn render_terminal(&self);
}

/// Unified response dispatcher for query/inspect models implementing `CliRender`
pub fn respond<T: CliRender>(
    format: OutputFormat,
    data: &T,
) -> Result<(), Box<dyn std::error::Error>> {
    if format.is_json() {
        println!("{}", serde_json::to_string_pretty(data)?);
    } else {
        data.render_terminal();
    }
    Ok(())
}

/// Unified response dispatcher with a custom UI closure for terminal presentation
pub fn respond_with<T: Serialize, F: FnOnce(&T)>(
    format: OutputFormat,
    data: &T,
    render_ui: F,
) -> Result<(), Box<dyn std::error::Error>> {
    if format.is_json() {
        println!("{}", serde_json::to_string_pretty(data)?);
    } else {
        render_ui(data);
    }
    Ok(())
}

/// Standard envelope for action/mutation results (start, stop, switch, provision)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActionEnvelope<T: Serialize = serde_json::Value> {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

/// Unified response helper for action/mutation commands
pub fn respond_action<T: Serialize>(
    format: OutputFormat,
    success: bool,
    msg: impl std::fmt::Display,
    data: Option<T>,
) -> Result<(), Box<dyn std::error::Error>> {
    let message = msg.to_string();
    if format.is_json() {
        let envelope = ActionEnvelope {
            success,
            message,
            data,
        };
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else if success {
        crate::ui::Notify::success(message);
    } else {
        crate::ui::Notify::error(message);
    }
    Ok(())
}

/// Unified error output helper respecting JSON format mode
pub fn respond_error(
    format: OutputFormat,
    err: impl std::fmt::Display,
) {
    if format.is_json() {
        let envelope = serde_json::json!({
            "success": false,
            "error": err.to_string(),
        });
        println!("{}", serde_json::to_string_pretty(&envelope).unwrap_or_default());
    } else {
        crate::ui::Notify::error(err);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct DummyModel {
        id: String,
        count: usize,
    }

    #[test]
    fn test_action_envelope_serialization() {
        let env = ActionEnvelope {
            success: true,
            message: "All good".to_string(),
            data: Some(DummyModel {
                id: "test".to_string(),
                count: 42,
            }),
        };
        let json_str = serde_json::to_string(&env).expect("Serialize ActionEnvelope");
        assert!(json_str.contains("\"success\":true"));
        assert!(json_str.contains("\"message\":\"All good\""));
        assert!(json_str.contains("\"id\":\"test\""));
        assert!(json_str.contains("\"count\":42"));
    }

    #[test]
    fn test_action_envelope_none_data() {
        let env: ActionEnvelope<()> = ActionEnvelope {
            success: false,
            message: "Failed".to_string(),
            data: None,
        };
        let json_str = serde_json::to_string(&env).expect("Serialize ActionEnvelope");
        assert!(json_str.contains("\"success\":false"));
        assert!(!json_str.contains("\"data\""));
    }

    #[test]
    fn test_respond_with_json_and_ui_branches() {
        let model = DummyModel {
            id: "browser-1".to_string(),
            count: 2,
        };

        // In JSON format, UI closure should NOT be called
        let mut ui_called = false;
        let res = respond_with(OutputFormat::Json, &model, |_| {
            ui_called = true;
        });
        assert!(res.is_ok());
        assert!(!ui_called);

        // In Card format, UI closure MUST be called
        let mut ui_called = false;
        let res = respond_with(OutputFormat::Card, &model, |_| {
            ui_called = true;
        });
        assert!(res.is_ok());
        assert!(ui_called);
    }
}
