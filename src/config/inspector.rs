use crate::commands::schema::get_config_schema;
use crate::config::registry::ConfigRegistry;
use crate::ui::{
    Card, ClackOption, Column, Table, badge_online, clack_cancel, clack_intro, clack_outro,
    clack_password, clack_select, clack_step_ok, clack_step_warn, clack_text, colors,
    is_interactive_tty,
};
use serde_json::{Map, Value, json};
use std::fs;
use std::io::IsTerminal;

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
    /// Render the options table for a given service on the Clack timeline
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
            "Tip: 'tq config {} <option> <value>' to modify, or '--edit' to open editor",
            canonical
        ));
        card.print();

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
            let desc = format!("{}{RESET}", opt.description, RESET = colors::RESET,);

            table.add_row(vec![opt_key, cur_val, choices, def_val, desc]);
        }

        let footer = format!(
            "{} options declared in schema • Pass '--edit' to launch editor with IntelliSense",
            options.len()
        );
        table.with_footer(footer).print();

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
                    if let Some(items_type) = prop
                        .get("items")
                        .and_then(|i| i.get("type"))
                        .and_then(|it| it.as_str())
                    {
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
            let raw_current_val = if let Some(val) = current_json.get(key) {
                format_json_value(val)
            } else if let Some(def) = prop.get("default") {
                format_json_value(def)
            } else {
                "—".to_string()
            };
            let current_val = if is_sensitive_config_key(key)
                && raw_current_val != "—"
                && !raw_current_val.is_empty()
            {
                mask_secret_value(&raw_current_val)
            } else {
                raw_current_val
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
                let available: Vec<&str> = properties
                    .keys()
                    .filter(|k| *k != "$schema")
                    .map(|s| s.as_str())
                    .collect();
                return Err(format!(
                    "Unknown option '{}' for service '{}'. Available options: [{}]",
                    key,
                    canonical,
                    available.join(", ")
                )
                .into());
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

        let val_opt = current_json
            .get(actual_key)
            .or_else(|| prop_def.get("default"));
        let val_str = match val_opt {
            Some(v) => format_json_value(v),
            None => "—".to_string(),
        };

        if std::io::stdout().is_terminal() {
            let display_val =
                if is_sensitive_config_key(actual_key) && val_str != "—" && !val_str.is_empty() {
                    mask_secret_value(&val_str)
                } else {
                    val_str.clone()
                };
            clack_intro(
                &format!("tq config {}", canonical),
                &format!("Inspect Option: {}", actual_key),
            );
            let mut card = Card::new(format!("{}: {}", canonical.to_uppercase(), actual_key));
            card.with_badge(badge_online("OPTION"));
            card.with_min_width(64);
            card.add_kv("Option", actual_key);
            card.add_kv("Value", &display_val);
            if let Some(desc) = prop_def.get("description").and_then(|d| d.as_str()) {
                card.add_kv("Description", desc);
            }
            card.with_footer(format!(
                "To update: tq config {} {} <new_value>",
                canonical, actual_key
            ));
            card.print();
            clack_outro("Option inspected.");
        } else {
            println!("{}", val_str);
        }

        Ok(val_str)
    }

    /// Set a specific config key with type & enum validation
    pub fn set_key(
        service: &str,
        key: &str,
        raw_value: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
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
                let available: Vec<&str> = properties
                    .keys()
                    .filter(|k| *k != "$schema")
                    .map(|s| s.as_str())
                    .collect();
                return Err(format!(
                    "Unknown option '{}' for service '{}'. Available options: [{}]",
                    key,
                    canonical,
                    available.join(", ")
                )
                .into());
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

        let raw_display = format_json_value(&converted_val);
        let val_display = if is_sensitive_config_key(&actual_key) && !raw_display.is_empty() {
            mask_secret_value(&raw_display)
        } else {
            raw_display
        };
        clack_step_ok(format!(
            "Updated {}.{} = {} ({})",
            canonical,
            actual_key,
            val_display,
            path.display()
        ));
        clack_outro("Configuration saved.");

        Ok(())
    }

    /// Interactive global configuration picker (`tq config` on TTY)
    pub fn interactive_global_config() -> Result<(), Box<dyn std::error::Error>> {
        if !is_interactive_tty() {
            ConfigRegistry::render_overview_card();
            return Ok(());
        }

        clack_intro(
            "tq config",
            "Unified Microservice Pillar Configuration Controller",
        );

        let entries = ConfigRegistry::list_all();
        let mut opts: Vec<ClackOption> = entries
            .iter()
            .map(|e| {
                let state = if e.exists { "ready" } else { "auto-init" };
                ClackOption::new(
                    format!("{} — {}", e.pillar, e.title),
                    format!("{} ({})", e.path.display(), state),
                    format!("{} ({})", e.pillar, e.title),
                )
            })
            .collect();
        opts.push(ClackOption::new(
            "View All Pillar Paths Overview (--show)",
            "Display SSOT configuration registry card",
            "All Pillar Paths Overview",
        ));

        match clack_select(
            "Select a microservice pillar to inspect or configure",
            &opts,
            0,
            10,
        )? {
            Some(idx) if idx < entries.len() => {
                Self::interactive_service_config_inner(entries[idx].pillar, true)?;
            }
            Some(_) => {
                ConfigRegistry::render_overview_card();
                clack_outro("All 7 microservice configuration paths listed.");
            }
            None => {
                clack_cancel("Operation cancelled.");
            }
        }
        Ok(())
    }

    /// Interactive service configuration editor (`tq config <service>` or `tq <service> config` on TTY)
    pub fn interactive_service_config(service: &str) -> Result<(), Box<dyn std::error::Error>> {
        Self::interactive_service_config_inner(service, false)
    }

    fn interactive_service_config_inner(
        service: &str,
        intro_already_shown: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let canonical = ConfigRegistry::canonical_service(service)
            .ok_or_else(|| format!("Unknown service '{}'", service))?;
        let path = ConfigRegistry::resolve_service_path(canonical)
            .ok_or_else(|| format!("Cannot resolve config path for '{}'", canonical))?;

        if !is_interactive_tty() {
            return Self::render_options_table(canonical);
        }

        if !intro_already_shown {
            clack_intro(
                &format!("tq config {}", canonical),
                &format!(
                    "{} Configuration • {}",
                    canonical.to_uppercase(),
                    path.display()
                ),
            );
        }

        let schema_raw = get_config_schema(canonical)
            .ok_or_else(|| format!("No schema found for service '{}'", canonical))?;
        let schema_json: Value = serde_json::from_str(schema_raw)?;
        let current_json = if path.exists() {
            fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str::<Value>(&s).ok())
                .unwrap_or_else(|| json!({}))
        } else {
            json!({})
        };

        let options = Self::extract_options(&schema_json, &current_json);
        let mut clack_opts: Vec<ClackOption> = options
            .iter()
            .map(|o| {
                ClackOption::new(
                    format!("{} = {}", o.key, o.current_value),
                    format!("{} — {}", o.type_or_choices, o.description),
                    format!("{} (current: {})", o.key, o.current_value),
                )
            })
            .collect();

        clack_opts.push(ClackOption::new(
            "Show Full Options Table (--show)",
            "Render complete schema options table in terminal",
            "Show Full Options Table",
        ));
        clack_opts.push(ClackOption::new(
            "Open JSON Config in Default Editor (--edit)",
            format!("Launch editor for {}", path.display()),
            "Open in Default Editor",
        ));

        let Some(choice_idx) = clack_select(
            &format!("Select a '{}' option to modify (or Esc to exit)", canonical),
            &clack_opts,
            0,
            10,
        )?
        else {
            clack_cancel("No changes made.");
            return Ok(());
        };

        if choice_idx == options.len() {
            Self::render_options_table(canonical)?;
            clack_outro(&format!(
                "Inspected {} options for '{}'.",
                options.len(),
                canonical
            ));
            return Ok(());
        }
        if choice_idx == options.len() + 1 {
            ConfigRegistry::open_in_editor_with_service(&path, canonical)?;
            clack_outro(&format!("Opened {} in editor.", path.display()));
            return Ok(());
        }

        let target_opt = &options[choice_idx];
        let prop_def = schema_json
            .get("properties")
            .and_then(|p| p.get(&target_opt.key));

        let new_val_opt: Option<String> = if let Some(enums) = prop_def
            .and_then(|p| p.get("enum"))
            .and_then(|e| e.as_array())
        {
            let choices: Vec<String> = enums
                .iter()
                .map(|v| match v {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                })
                .collect();
            let default_idx = choices
                .iter()
                .position(|c| c.eq_ignore_ascii_case(&target_opt.current_value))
                .unwrap_or(0);
            let enum_opts: Vec<ClackOption> = choices
                .iter()
                .map(|c| {
                    let hint = if c.eq_ignore_ascii_case(&target_opt.current_value) {
                        "current value"
                    } else if c.eq_ignore_ascii_case(&target_opt.default_value) {
                        "default"
                    } else {
                        ""
                    };
                    ClackOption::new(c, hint, c)
                })
                .collect();
            clack_select(
                &format!("Select new value for {}.{}", canonical, target_opt.key),
                &enum_opts,
                default_idx,
                8,
            )?
            .map(|i| choices[i].clone())
        } else if prop_def
            .and_then(|p| p.get("type"))
            .and_then(|t| t.as_str())
            == Some("boolean")
        {
            let bool_opts = vec![
                ClackOption::new("true", "Enable option", "true"),
                ClackOption::new("false", "Disable option", "false"),
            ];
            let default_idx = if target_opt.current_value.eq_ignore_ascii_case("true") {
                0
            } else {
                1
            };
            match clack_select(
                &format!("Set boolean value for {}.{}", canonical, target_opt.key),
                &bool_opts,
                default_idx,
                4,
            )? {
                Some(0) => Some("true".to_string()),
                Some(1) => Some("false".to_string()),
                _ => None,
            }
        } else if is_sensitive_config_key(&target_opt.key) {
            clack_password(&format!(
                "Enter secret value for {}.{} (masked):",
                canonical, target_opt.key
            ))?
        } else {
            let initial = if target_opt.current_value == "—" {
                None
            } else {
                Some(target_opt.current_value.as_str())
            };
            loop {
                let candidate = clack_text(
                    &format!(
                        "Enter new value for {}.{} ({})",
                        canonical, target_opt.key, target_opt.type_or_choices
                    ),
                    Some(&target_opt.default_value),
                    initial,
                )?;
                let Some(ref raw) = candidate else {
                    break None;
                };
                if let Some(p_def) = prop_def {
                    if let Err(e) = convert_and_validate(p_def, raw, &target_opt.key) {
                        clack_step_warn(e.to_string());
                        continue;
                    }
                }
                break candidate;
            }
        };

        match new_val_opt {
            Some(new_val) => {
                Self::set_key(canonical, &target_opt.key, &new_val)?;
            }
            None => {
                clack_cancel("No changes saved.");
            }
        }

        Ok(())
    }

    /// Dispatch handler for service config: handles args (get/set/list) and flags (--edit, --show)
    pub fn handle_dispatch(
        service: &str,
        args: &[String],
        edit: bool,
        show: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let canonical = ConfigRegistry::canonical_service(service)
            .ok_or_else(|| format!("Unknown service '{}'", service))?;
        let path = ConfigRegistry::resolve_service_path(canonical)
            .ok_or_else(|| format!("Cannot resolve config path for '{}'", canonical))?;

        if edit {
            clack_intro(
                &format!("tq config {}", canonical),
                "Launch Configuration Editor",
            );
            ConfigRegistry::open_in_editor_with_service(&path, canonical)?;
            clack_outro(&format!("Opened {} in default editor.", path.display()));
            return Ok(());
        }

        match args {
            [] => {
                if !show && is_interactive_tty() {
                    Self::interactive_service_config(canonical)?;
                } else {
                    clack_intro(
                        &format!("tq config {}", canonical),
                        &format!("{} Schema Options", canonical.to_uppercase()),
                    );
                    Self::render_options_table(canonical)?;
                    clack_outro(&format!("Configuration file: {}", path.display()));
                }
            }
            [single] => {
                if single.eq_ignore_ascii_case("list") || single.eq_ignore_ascii_case("all") {
                    clack_intro(
                        &format!("tq config {}", canonical),
                        &format!("{} Schema Options", canonical.to_uppercase()),
                    );
                    Self::render_options_table(canonical)?;
                    clack_outro(&format!("Configuration file: {}", path.display()));
                } else {
                    Self::get_key(canonical, single)?;
                }
            }
            [action, key] if action.eq_ignore_ascii_case("get") => {
                Self::get_key(canonical, key)?;
            }
            [key, val] if !key.eq_ignore_ascii_case("set") && !key.eq_ignore_ascii_case("get") => {
                clack_intro(
                    &format!("tq config {}", canonical),
                    "Update Configuration Key",
                );
                Self::set_key(canonical, key, val)?;
            }
            [action, key, val] if action.eq_ignore_ascii_case("set") => {
                clack_intro(
                    &format!("tq config {}", canonical),
                    "Update Configuration Key",
                );
                Self::set_key(canonical, key, val)?;
            }
            other => {
                return Err(format!(
                    "Invalid arguments for config: {:?}. Usage: 'tq config {} [key] [value]'",
                    other, canonical
                )
                .into());
            }
        }

        Ok(())
    }
}

pub fn is_sensitive_config_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    lower.contains("secret")
        || lower.contains("token")
        || lower.contains("password")
        || lower.ends_with("_key")
        || lower == "api_key"
}

pub fn mask_secret_value(val: &str) -> String {
    let chars: Vec<char> = val.chars().collect();
    if chars.len() <= 6 {
        "••••••••".to_string()
    } else {
        let prefix: String = chars[..3].iter().collect();
        let suffix: String = chars[chars.len() - 2..].iter().collect();
        format!("{}••••{}", prefix, suffix)
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
        if let Some(matched) = valid_choices
            .iter()
            .find(|c| c.eq_ignore_ascii_case(trimmed))
        {
            return Ok(Value::String(matched.clone()));
        } else {
            return Err(format!(
                "Invalid value '{}' for option '{}'. Allowed choices: [{}]",
                raw_val,
                key,
                valid_choices.join(", ")
            )
            .into());
        }
    }

    // 2. Check property type
    let prop_type = prop_def
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("string");
    match prop_type {
        "boolean" => match trimmed.to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => Ok(Value::Bool(true)),
            "false" | "0" | "no" | "off" => Ok(Value::Bool(false)),
            _ => Err(format!(
                "Invalid boolean value '{}' for '{}'. Expected 'true' or 'false'",
                raw_val, key
            )
            .into()),
        },
        "integer" => {
            let n = trimmed.parse::<i64>().map_err(|_| {
                format!(
                    "Invalid integer value '{}' for '{}'. Expected whole number",
                    raw_val, key
                )
            })?;
            if let Some(min) = prop_def.get("minimum").and_then(|m| m.as_i64())
                && n < min
            {
                return Err(format!(
                    "Value {} for '{}' is below minimum allowed ({})",
                    n, key, min
                )
                .into());
            }
            if let Some(max) = prop_def.get("maximum").and_then(|m| m.as_i64())
                && n > max
            {
                return Err(format!(
                    "Value {} for '{}' exceeds maximum allowed ({})",
                    n, key, max
                )
                .into());
            }
            if key.ends_with("port") && !(1..=65535).contains(&n) {
                return Err(format!(
                    "Port number {} for '{}' must be between 1 and 65535",
                    n, key
                )
                .into());
            }
            Ok(json!(n))
        }
        "number" => {
            let n = trimmed.parse::<f64>().map_err(|_| {
                format!(
                    "Invalid numeric value '{}' for '{}'. Expected number",
                    raw_val, key
                )
            })?;
            if let Some(min) = prop_def.get("minimum").and_then(|m| m.as_f64())
                && n < min
            {
                return Err(format!(
                    "Value {} for '{}' is below minimum allowed ({})",
                    n, key, min
                )
                .into());
            }
            if let Some(max) = prop_def.get("maximum").and_then(|m| m.as_f64())
                && n > max
            {
                return Err(format!(
                    "Value {} for '{}' exceeds maximum allowed ({})",
                    n, key, max
                )
                .into());
            }
            Ok(json!(n))
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
                let items: Vec<String> = arr
                    .iter()
                    .filter_map(|i| i.as_str())
                    .map(|s| format!("\"{}\"", s))
                    .collect();
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
        let schema_raw =
            crate::commands::schema::get_config_schema("faker").expect("Faker schema exists");
        let schema: Value = serde_json::from_str(schema_raw).expect("Valid JSON");
        let current = json!({
            "default_nat": "US",
            "default_domain": "example.com"
        });

        let opts = ConfigController::extract_options(&schema, &current);
        assert!(!opts.is_empty());

        let nat_opt = opts
            .iter()
            .find(|o| o.key == "default_nat")
            .expect("default_nat option exists");
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
        assert_eq!(
            convert_and_validate(&bool_prop, "true", "headless").unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            convert_and_validate(&bool_prop, "0", "headless").unwrap(),
            Value::Bool(false)
        );
        assert!(convert_and_validate(&bool_prop, "maybe", "headless").is_err());

        let int_prop = json!({ "type": "integer" });
        assert_eq!(
            convert_and_validate(&int_prop, "1280", "viewport_width").unwrap(),
            json!(1280)
        );
        assert!(convert_and_validate(&int_prop, "abc", "viewport_width").is_err());
        assert!(convert_and_validate(&int_prop, "70000", "default_port").is_err());
        assert_eq!(
            convert_and_validate(&int_prop, "8765", "default_port").unwrap(),
            json!(8765)
        );
    }

    #[test]
    fn test_secret_masking() {
        assert!(is_sensitive_config_key("webhook_secret"));
        assert!(is_sensitive_config_key("api_key"));
        assert!(is_sensitive_config_key("auth_token"));
        assert!(!is_sensitive_config_key("default_port"));
        assert_eq!(mask_secret_value("short"), "••••••••");
        assert_eq!(mask_secret_value("super_secret_token_99"), "sup••••99");
    }

    #[test]
    fn test_convert_and_validate_array() {
        let arr_prop = json!({ "type": "array" });
        let val = convert_and_validate(&arr_prop, "automa, stealth", "extensions").unwrap();
        assert_eq!(val, json!(["automa", "stealth"]));
    }
}
