use crate::ui::OutputFormat;
use serde::Serialize;

/// Trait for data models that can render themselves to the terminal
pub trait CliRender {
    /// Render clean presentation to terminal
    fn render_terminal(&self);
}

/// Helper to render data based on requested format
pub fn respond<T: Serialize + CliRender>(
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
    message: impl Into<String>,
    data: Option<T>,
) -> Result<(), Box<dyn std::error::Error>> {
    let msg = message.into();
    if format.is_json() {
        let envelope = ActionEnvelope {
            success,
            message: msg,
            data,
        };
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        if success {
            println!("{}", msg);
        } else {
            eprintln!("ERROR: {}", msg);
        }
    }
    Ok(())
}

/// Standard error responder that respects machine formats
pub fn respond_error(format: OutputFormat, err: impl std::fmt::Display) {
    if format.is_json() {
        let err_json = serde_json::json!({
            "success": false,
            "error": err.to_string(),
        });
        println!("{}", err_json);
    } else {
        eprintln!("ERROR: {}", err);
    }
}
