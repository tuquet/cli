/// Pure function: Expands ${VAR} and ${VAR:-default} from environment variables
pub struct EnvExpander;

impl EnvExpander {
    pub fn expand(input: &str) -> String {
        let mut output = String::with_capacity(input.len());
        let mut chars = input.char_indices().peekable();

        while let Some((_, c)) = chars.next() {
            if c == '$'
                && let Some(&(_, '{')) = chars.peek() {
                    chars.next(); // consume '{'
                    let mut var_expr = String::new();
                    let mut closed = false;
                    for (_, inner) in chars.by_ref() {
                        if inner == '}' {
                            closed = true;
                            break;
                        }
                        var_expr.push(inner);
                    }
                    if closed {
                        if let Some((var_name, default_val)) = var_expr.split_once(":-") {
                            let val = std::env::var(var_name.trim())
                                .unwrap_or_else(|_| default_val.to_string());
                            output.push_str(&val);
                        } else {
                            let val = std::env::var(var_expr.trim()).unwrap_or_default();
                            output.push_str(&val);
                        }
                        continue;
                    } else {
                        output.push('$');
                        output.push('{');
                        output.push_str(&var_expr);
                        continue;
                    }
                }
            output.push(c);
        }
        output
    }
}

pub const DEFAULT_SSOT_DIR_NAME: &str = ".specter";

/// Resolves the canonical SSOT root directory (~/.specter/ or $SPECTER_HOME)
pub fn canonical_ssot_dir() -> std::path::PathBuf {
    if let Ok(dir) = std::env::var("SPECTER_HOME") {
        if !dir.trim().is_empty() {
            return std::path::PathBuf::from(dir);
        }
    }

    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home).join(DEFAULT_SSOT_DIR_NAME)
}

/// Canonical root directory for Specter (~/.specter/)
pub fn canonical_specter_dir() -> std::path::PathBuf {
    canonical_ssot_dir()
}

/// Backward-compatible alias for existing call-sites
pub fn canonical_tuquet_dir() -> std::path::PathBuf {
    canonical_ssot_dir()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_with_default() {
        let input = "Server_${NON_EXISTENT_VAR_123:-fallback_val}";
        let output = EnvExpander::expand(input);
        assert_eq!(output, "Server_fallback_val");
    }

    #[test]
    fn test_expand_with_existing_env() {
        unsafe {
            std::env::set_var("TEST_TUQUET_ENV_VAR", "injected_val");
        }
        let input = "Prefix_${TEST_TUQUET_ENV_VAR}_Suffix";
        let output = EnvExpander::expand(input);
        assert_eq!(output, "Prefix_injected_val_Suffix");
    }

    #[test]
    fn test_canonical_ssot_dir() {
        let dir = canonical_ssot_dir();
        assert!(dir.to_string_lossy().contains(".specter") || dir.to_string_lossy().contains(".tuquet"));
    }
}

