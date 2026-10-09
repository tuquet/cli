use clap::Args;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Json,
    Table,
    Card,
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

/// Common CLI arguments for output formatting across Specter commands
#[derive(Args, Debug, Clone)]
pub struct FormatArgs {
    /// Output format: auto, json, table, or card (default: auto)
    #[arg(short = 'f', long, default_value = "auto")]
    pub format: String,

    /// Display formatted terminal table view (shortcut for -f table)
    #[arg(long)]
    pub table: bool,

    /// Display rich visual card view (shortcut for -f card)
    #[arg(short = 'c', long)]
    pub card: bool,

    /// Output results in raw JSON format (shortcut for -f json)
    #[arg(short = 'j', long)]
    pub json: bool,
}

impl Default for FormatArgs {
    fn default() -> Self {
        Self {
            format: "auto".to_string(),
            table: false,
            card: false,
            json: false,
        }
    }
}

impl FormatArgs {
    /// Resolve the effective output format based on flags, format string, and TTY detection
    pub fn resolve(&self) -> OutputFormat {
        if self.json {
            OutputFormat::Json
        } else if self.card {
            OutputFormat::Card
        } else if self.table {
            OutputFormat::Table
        } else {
            match self.format.trim().to_lowercase().as_str() {
                "card" => OutputFormat::Card,
                "table" => OutputFormat::Table,
                "json" => OutputFormat::Json,
                _ => {
                    use std::io::IsTerminal;
                    if std::io::stdout().is_terminal() {
                        OutputFormat::Card
                    } else {
                        OutputFormat::Json
                    }
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
                table: false,
                card: false,
                json: true,
            },
            OutputFormat::Table => FormatArgs {
                format: "table".to_string(),
                table: true,
                card: false,
                json: false,
            },
            OutputFormat::Card => FormatArgs {
                format: "card".to_string(),
                table: false,
                card: true,
                json: false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_args_default_is_json() {
        let args = FormatArgs::default();
        assert_eq!(args.resolve(), OutputFormat::Json);
        assert!(args.resolve().is_json());
    }

    #[test]
    fn test_format_args_string_resolution() {
        let args_card = FormatArgs {
            format: "card".to_string(),
            ..Default::default()
        };
        assert_eq!(args_card.resolve(), OutputFormat::Card);

        let args_table = FormatArgs {
            format: "TABLE".to_string(),
            ..Default::default()
        };
        assert_eq!(args_table.resolve(), OutputFormat::Table);

        let args_unknown = FormatArgs {
            format: "unknown".to_string(),
            ..Default::default()
        };
        assert_eq!(args_unknown.resolve(), OutputFormat::Json);
    }

    #[test]
    fn test_format_args_flag_precedence() {
        // --card overrides default format "json"
        let args_card = FormatArgs {
            format: "json".to_string(),
            card: true,
            ..Default::default()
        };
        assert_eq!(args_card.resolve(), OutputFormat::Card);

        // --table overrides default format "json"
        let args_table = FormatArgs {
            format: "json".to_string(),
            table: true,
            ..Default::default()
        };
        assert_eq!(args_table.resolve(), OutputFormat::Table);

        // --card has precedence over --table
        let args_both = FormatArgs {
            format: "json".to_string(),
            card: true,
            table: true,
            ..Default::default()
        };
        assert_eq!(args_both.resolve(), OutputFormat::Card);
    }
}
