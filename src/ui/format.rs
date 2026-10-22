use clap::Args;
use serde::{Deserialize, Serialize};

/// Output presentation mode.
/// Internal logic can distinguish Table vs Card presentation if needed,
/// but CLI flags are strictly streamlined to standard: Text (default) or JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Table,
    Card,
    Json,
}

impl OutputFormat {
    pub fn is_json(&self) -> bool {
        matches!(self, OutputFormat::Json)
    }

    pub fn is_table(&self) -> bool {
        matches!(self, OutputFormat::Table)
    }

    pub fn is_card(&self) -> bool {
        matches!(self, OutputFormat::Card)
    }
}

/// Standard, clean CLI arguments for output formatting
/// Eliminates legacy `-f card` and `-f table`.
/// By default on terminal, commands output clean formatted Text (table/describe).
/// With `--json` or `-j`, commands output structured JSON.
#[derive(Args, Debug, Clone)]
pub struct FormatArgs {
    /// Output format: text, json (default: text on TTY, json if piped)
    #[arg(long, default_value = "auto")]
    pub format: String,

    /// Output results in raw JSON format (alias for --format=json)
    #[arg(short = 'j', long)]
    pub json: bool,
}

impl Default for FormatArgs {
    fn default() -> Self {
        Self {
            format: "auto".to_string(),
            json: false,
        }
    }
}

impl FormatArgs {
    /// Resolve the effective output format based on flags and TTY detection
    pub fn resolve(&self) -> OutputFormat {
        if self.json {
            return OutputFormat::Json;
        }

        match self.format.trim().to_lowercase().as_str() {
            "json" => OutputFormat::Json,
            "text" | "plain" | "table" | "card" => OutputFormat::Table,
            _ => {
                use std::io::IsTerminal;
                if std::io::stdout().is_terminal() {
                    OutputFormat::Table
                } else {
                    OutputFormat::Json
                }
            }
        }
    }
}

impl From<OutputFormat> for FormatArgs {
    fn from(fmt: OutputFormat) -> Self {
        match fmt {
            OutputFormat::Json => FormatArgs {
                format: "json".to_string(),
                json: true,
            },
            _ => FormatArgs {
                format: "text".to_string(),
                json: false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_args_json_flag() {
        let args = FormatArgs {
            format: "auto".to_string(),
            json: true,
        };
        assert_eq!(args.resolve(), OutputFormat::Json);
    }

    #[test]
    fn test_format_args_string_overrides() {
        let args_json = FormatArgs {
            format: "json".to_string(),
            json: false,
        };
        assert_eq!(args_json.resolve(), OutputFormat::Json);

        let args_text = FormatArgs {
            format: "text".to_string(),
            json: false,
        };
        assert_eq!(args_text.resolve(), OutputFormat::Table);
    }
}
