use std::fs;
use std::io::IsTerminal;
use serde_json::{json, Map, Value};
use crate::commands::schema::get_config_schema;
use crate::config::registry::ConfigRegistry;
use crate::ui::{badge_online, colors, Card, Column, Table};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigOptionDef {
    pub key: String,
    pub current_value: String,
    pub type_or_choices: String,
    pub default_value: String,
    pub description: String,
}

pub struct ConfigController;

impl ConfigController {
    /// Render the interactive options table for a given service
    pub fn render_options_table(service: &str) -> Result<(), Box<dyn std::error::Error>> {
        let canonical = ConfigRegistry::canonical_service(service)
            .ok_or_else(|| format!("Unknown service '{}'", service))?;

        let schema_raw = get_config_schema(canonical)
            .ok_or_else(|| format!("No schema found for service '{}'", canonical))?;

        let schema_json: Value = serde_json::from_str(schema_raw)?;
        let path = ConfigRegistry::resolve_service_path(canonical)
            .ok_or_else(|| format!("Cannot resolve config path for service '{}'", canonical))?;

        let current_json = if path.exists() {
            fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str::<Value>(&s).ok())
                .unwrap_or_else(|| json!({}))
        } else {
            json!({})
        };

        let options = Self::extract_options(&schema_json, &current_json);

        // Header Card
        println!();
        let title = format!("{} CONFIGURATION & OPTIONS", canonical.to_uppercase());
        let mut card = Card::new(title);
        card.with_badge(badge_online("SSOT READY"));
        card.with_min_width(74);

        card.add_kv("Config File", path.display().to_string());
        card.add_kv(
            "Schema",
            format!("schema/config/{}.schema.json (Draft 2020-12)", canonical),
        );
        card.with_footer(format!(
            "Tip: 'specter config {} <option> <value>' to modify, or '--edit' to open editor",
            canonical
        ));
        card.print();
        println!();

        // Table
        let cols = vec![
            Column {
                title: "OPTION".to_string(),
                min_width: 18,
                align_right: false,
            },
            Column {
                title: "CURRENT VALUE".to_string(),
                min_width: 16,
                align_right: false,
            },
            Column {
                title: "TYPE / CHOICES".to_string(),
                min_width: 22,
                align_right: false,
            },
            Column {
                title: "DEFAULT".to_string(),
                min_width: 12,
                align_right: false,
            },
            Column {
                title: "DESCRIPTION".to_string(),
                min_width: 36,
                align_right: false,
            },
        ];

        let mut table = Table::new(cols);
        for opt in &options {
            let opt_key = format!(
                "{BOLD_WHITE}{}{RESET}",
                opt.key,
                BOLD_WHITE = colors::BOLD_WHITE,
                RESET = colors::RESET,
            );
            let cur_val = format!(
                "{CYAN}{}{RESET}",
                opt.current_value,
                CYAN = colors::CYAN,
                RESET = colors::RESET,
            );
            let choices = format!(
                "{PURPLE}{}{RESET}",
                opt.type_or_choices,
                PURPLE = colors::PURPLE,
                RESET = colors::RESET,
            );
            let def_val = format!(
                "{MUTED}{}{RESET}",
                opt.default_value,
                MUTED = colors::MUTED,
                RESET = colors::RESET,
            );
            let desc = format!(
                "{}{RESET}",
                opt.description,
                RESET = colors::RESET,
            );

            table.add_row(vec![opt_key, cur_val, choices, def_val, desc]);
        }

        let footer = format!(
            "{} options declared in schema • Pass '--edit' to launch editor with IntelliSense",
            options.len()
        );
        let table = table.with_footer(footer);
        println!("{}", table.render());
        println!();

        Ok(())
    }

    /// Extract options list from schema and current JSON
    pub fn extract_options(schema: &Value, current_json: &Value) -> Vec<ConfigOptionDef> {
        let mut result = Vec::new();
        let properties = match schema.get("properties").and_then(|p| p.as_object()) {
            Some(props) => props,
            None => return result,
        };

        for (key, prop) in properties {
            if key == "$schema" {
                continue;
            }

            let desc = prop
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string();

            // Determine type or enum choices
            let type_or_choices = if let Some(enums) = prop.get("enum").and_then(|e| e.as_array()) {
                let items: Vec<String> = enums
                    .iter()
                    .map(|v| match v {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    })
                    .collect();
                format!("[{}]", items.join(", "))
            } else if let Some(t) = prop.get("type").and_then(|t| t.as_str()) {
                if t == "array" {
                    if let Some(items_type) = prop.get("items").and_then(|i| i.get("type")).and_then(|it| it.as_str()) {
                        format!("array[{}]", items_type)
                    } else {
                        "array".to_string()
                    }
                } else {
                    t.to_string()
                }
            } else {
                "any".to_string()
            };

            // Determine default value
            let default_val = if let Some(def) = prop.get("default") {
                format_json_value(def)
            } else {
                "—".to_string()
            };

            // Determine current value
            let current_val = if let Some(val) = current_json.get(key) {
                format_json_value(val)
            } else if let Some(def) = prop.get("default") {
                format_json_value(def)
            } else {
                "—".to_string()
            };

            result.push(ConfigOptionDef {
                key: key.clone(),
                current_value: current_val,
                type_or_choices,
                default_value: default_val,
                description: desc,
            });
        }

        // Sort by key name for deterministic presentation
        result.sort_by(|a, b| a.key.cmp(&b.key));
        result
    }

    /// Read a specific config key
    pub fn get_key(service: &str, key: &str) -> Result<String, Box<dyn std::error::Error>> {
        let canonical = ConfigRegistry::canonical_service(service)
            .ok_or_else(|| format!("Unknown service '{}'", service))?;

        let schema_raw = get_config_schema(canonical)
            .ok_or_else(|| format!("No schema found for service '{}'", canonical))?;
        let schema_json: Value = serde_json::from_str(schema_raw)?;

        let properties = schema_json
            .get("properties")
            .and_then(|p| p.as_object())
            .ok_or_else(|| format!("No properties defined in schema for '{}'", canonical))?;

        // Find exact or case-insensitive match for key
        let prop_entry = properties.iter().find(|(k, _)| k.eq_ignore_ascii_case(key));
        let (actual_key, prop_def) = match prop_entry {
            Some((k, p)) => (k.as_str(), p),
            None => {
                let available: Vec<&str> = properties.keys().filter(|k| *k != "$schema").map(|s| s.as_str()).collect();
                return Err(format!(
                    "Unknown option '{}' for service '{}'. Available options: [{}]",
                    key, canonical, available.join(", ")
                ).into());
            }
        };

        let path = ConfigRegistry::resolve_service_path(canonical)
            .ok_or_else(|| format!("Cannot resolve config path for '{}'", canonical))?;

        let current_json = if path.exists() {
            fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str::<Value>(&s).ok())
                .unwrap_or_else(|| json!({}))
        } else {
            json!({})
        };

        let val_opt = current_json.get(actual_key).or_else(|| prop_def.get("default"));
        let val_str = match val_opt {
            Some(v) => format_json_value(v),
            None => "—".to_string(),
        };

        if std::io::stdout().is_terminal() {
            println!();
            let mut card = Card::new(format!("{}: {}", canonical.to_uppercase(), actual_key));
            card.with_badge(badge_online("OPTION"));
            card.with_min_width(64);
            card.add_kv("Option", actual_key);
            card.add_kv("Value", &val_str);
            if let Some(desc) = prop_def.get("description").and_then(|d| d.as_str()) {
                card.add_kv("Description", desc);
            }
            card.with_footer(format!("To update: specter config {} {} <new_value>", canonical, actual_key));
            card.print();
            println!();
        } else {
            println!("{}", val_str);
        }

        Ok(val_str)
    }

    /// Set a specific config key with type & enum validation
    pub fn set_key(service: &str, key: &str, raw_value: &str) -> Result<(), Box<dyn std::error::Error>> {
        let canonical = ConfigRegistry::canonical_service(service)
            .ok_or_else(|| format!("Unknown service '{}'", service))?;

        let schema_raw = get_config_schema(canonical)
            .ok_or_else(|| format!("No schema found for service '{}'", canonical))?;
        let schema_json: Value = serde_json::from_str(schema_raw)?;

        let properties = schema_json
            .get("properties")
            .and_then(|p| p.as_object())
            .ok_or_else(|| format!("No properties defined in schema for '{}'", canonical))?;

        // Find exact or case-insensitive match for key
        let prop_entry = properties.iter().find(|(k, _)| k.eq_ignore_ascii_case(key));
        let (actual_key, prop_def) = match prop_entry {
            Some((k, p)) => (k.clone(), p),
            None => {
                let available: Vec<&str> = properties.keys().filter(|k| *k != "$schema").map(|s| s.as_str()).collect();
                return Err(format!(
                    "Unknown option '{}' for service '{}'. Available options: [{}]",
                    key, canonical, available.join(", ")
                ).into());
            }
        };

        // Convert & validate value against schema
        let converted_val = convert_and_validate(prop_def, raw_value, &actual_key)?;

        let path = ConfigRegistry::resolve_service_path(canonical)
            .ok_or_else(|| format!("Cannot resolve config path for '{}'", canonical))?;

        // Load existing or initialize default
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let mut map: Map<String, Value> = if path.exists() {
            fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str::<Value>(&s).ok())
                .and_then(|v| v.as_object().cloned())
                .unwrap_or_default()
        } else {
            ConfigRegistry::get_default_json(canonical)
                .as_object()
                .cloned()
                .unwrap_or_default()
        };

        // Ensure $schema is the first property
        let schema_url = ConfigRegistry::schema_url(canonical);
        map.insert("$schema".to_string(), Value::String(schema_url));

        // Insert new value
        map.insert(actual_key.clone(), converted_val.clone());

        // Reorder map so $schema is first
        let mut ordered = Map::new();
        if let Some(schema_val) = map.remove("$schema") {
            ordered.insert("$schema".to_string(), schema_val);
        }
        for (k, v) in map {
            ordered.insert(k, v);
        }

        let serialized = serde_json::to_string_pretty(&Value::Object(ordered))?;
        fs::write(&path, serialized)?;

        let val_display = format_json_value(&converted_val);
        println!(
            "  {} Updated {}.{} = {}",
            badge_online("SAVED"),
            canonical,
            actual_key,
            val_display
        );
        println!(
            "  {MUTED}Config file: {}{RESET}",
            path.display(),
            MUTED = colors::MUTED,
            RESET = colors::RESET
        );

        Ok(())
    }

    /// Dispatch handler for service config: handles args (get/set/list) and flags (--edit, --show)
    pub fn handle_dispatch(
        service: &str,
        args: &[String],
        edit: bool,
        _show: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let canonical = ConfigRegistry::canonical_service(service)
            .ok_or_else(|| format!("Unknown service '{}'", service))?;
        let path = ConfigRegistry::resolve_service_path(canonical)
            .ok_or_else(|| format!("Cannot resolve config path for '{}'", canonical))?;

        if edit {
            ConfigRegistry::open_in_editor_with_service(&path, canonical)?;
            return Ok(());
        }

        match args {
            [] => {
                Self::render_options_table(canonical)?;
            }
            [single] => {
                if single.eq_ignore_ascii_case("list") || single.eq_ignore_ascii_case("all") {
                    Self::render_options_table(canonical)?;
                } else {
                    Self::get_key(canonical, single)?;
                }
            }
            [action, key] if action.eq_ignore_ascii_case("get") => {
                Self::get_key(canonical, key)?;
            }
            [key, val] if !key.eq_ignore_ascii_case("set") && !key.eq_ignore_ascii_case("get") => {
                Self::set_key(canonical, key, val)?;
            }
            [action, key, val] if action.eq_ignore_ascii_case("set") => {
                Self::set_key(canonical, key, val)?;
            }
            other => {
                return Err(format!(
                    "Invalid arguments for config: {:?}. Usage: 'specter config {} [key] [value]'",
                    other, canonical
                ).into());
            }
        }

        Ok(())
    }
}

pub fn convert_and_validate(
    prop_def: &Value,
    raw_val: &str,
    key: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    let trimmed = raw_val.trim();

    // 1. If enum choices exist, check choices
    if let Some(enums) = prop_def.get("enum").and_then(|e| e.as_array()) {
        let valid_choices: Vec<String> = enums
            .iter()
            .map(|v| match v {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .collect();

        // Case-insensitive match against allowed enum values
        if let Some(matched) = valid_choices.iter().find(|c| c.eq_ignore_ascii_case(trimmed)) {
            return Ok(Value::String(matched.clone()));
        } else {
            return Err(format!(
                "Invalid value '{}' for option '{}'. Allowed choices: [{}]",
                raw_val, key, valid_choices.join(", ")
            ).into());
        }
    }

    // 2. Check property type
    let prop_type = prop_def.get("type").and_then(|t| t.as_str()).unwrap_or("string");
    match prop_type {
        "boolean" => {
            match trimmed.to_ascii_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => Ok(Value::Bool(true)),
                "false" | "0" | "no" | "off" => Ok(Value::Bool(false)),
                _ => Err(format!(
                    "Invalid boolean value '{}' for '{}'. Expected 'true' or 'false'",
                    raw_val, key
                ).into()),
            }
        }
        "integer" => {
            trimmed.parse::<i64>().map(|n| json!(n)).map_err(|_| {
                format!(
                    "Invalid integer value '{}' for '{}'. Expected whole number",
                    raw_val, key
                ).into()
            })
        }
        "number" => {
            trimmed.parse::<f64>().map(|n| json!(n)).map_err(|_| {
                format!(
                    "Invalid numeric value '{}' for '{}'. Expected number",
                    raw_val, key
                ).into()
            })
        }
        "array" => {
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                serde_json::from_str::<Value>(trimmed).map_err(|e| {
                    format!("Invalid JSON array '{}' for '{}': {}", raw_val, key, e).into()
                })
            } else {
                let parts: Vec<Value> = trimmed
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| Value::String(s.to_string()))
                    .collect();
                Ok(Value::Array(parts))
            }
        }
        _ => Ok(Value::String(trimmed.to_string())),
    }
}

pub fn format_json_value(val: &Value) -> String {
    match val {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Array(arr) => {
            if arr.iter().all(|item| item.is_string()) {
                let items: Vec<String> = arr.iter().filter_map(|i| i.as_str()).map(|s| format!("\"{}\"", s)).collect();
                format!("[{}]", items.join(", "))
            } else {
                serde_json::to_string(val).unwrap_or_else(|_| "[]".to_string())
            }
        }
        Value::Object(obj) => {
            format!("({} keys)", obj.len())
        }
        Value::Null => "null".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_options_from_faker_schema() {
        let schema_raw = crate::commands::schema::get_config_schema("faker").expect("Faker schema exists");
        let schema: Value = serde_json::from_str(schema_raw).expect("Valid JSON");
        let current = json!({
            "default_nat": "US",
            "default_domain": "example.com"
        });

        let opts = ConfigController::extract_options(&schema, &current);
        assert!(!opts.is_empty());

        let nat_opt = opts.iter().find(|o| o.key == "default_nat").expect("default_nat option exists");
        assert_eq!(nat_opt.current_value, "US");
        assert_eq!(nat_opt.default_value, "US");
        assert!(nat_opt.type_or_choices.contains("US"));
        assert!(nat_opt.type_or_choices.contains("VN"));
    }

    #[test]
    fn test_convert_and_validate_enum_choices() {
        let prop = json!({
            "type": "string",
            "enum": ["US", "VN", "JP", "all"]
        });

        // Valid exact and case-insensitive
        let val1 = convert_and_validate(&prop, "US", "default_nat").expect("Valid US");
        assert_eq!(val1, Value::String("US".to_string()));

        let val2 = convert_and_validate(&prop, "vn", "default_nat").expect("Valid vn");
        assert_eq!(val2, Value::String("VN".to_string()));

        // Invalid choice
        let err = convert_and_validate(&prop, "FR", "default_nat").unwrap_err();
        assert!(err.to_string().contains("Allowed choices"));
    }

    #[test]
    fn test_convert_and_validate_boolean_and_integer() {
        let bool_prop = json!({ "type": "boolean" });
        assert_eq!(convert_and_validate(&bool_prop, "true", "headless").unwrap(), Value::Bool(true));
        assert_eq!(convert_and_validate(&bool_prop, "0", "headless").unwrap(), Value::Bool(false));
        assert!(convert_and_validate(&bool_prop, "maybe", "headless").is_err());

        let int_prop = json!({ "type": "integer" });
        assert_eq!(convert_and_validate(&int_prop, "1280", "viewport_width").unwrap(), json!(1280));
        assert!(convert_and_validate(&int_prop, "abc", "viewport_width").is_err());
    }

    #[test]
    fn test_convert_and_validate_array() {
        let arr_prop = json!({ "type": "array" });
        let val = convert_and_validate(&arr_prop, "automa, stealth", "extensions").unwrap();
        assert_eq!(val, json!(["automa", "stealth"]));
    }
}
