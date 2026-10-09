use std::path::Path;
use serde_json::Value;

pub async fn execute_specter_tree(args: &Value) -> Result<String, String> {
    let target_path_str = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let max_depth = args.get("depth").and_then(|v| v.as_u64()).unwrap_or(3).min(10) as usize;
    let show_hidden = args.get("show_hidden").and_then(|v| v.as_bool()).unwrap_or(false);

    let root_path = Path::new(target_path_str);
    if !root_path.exists() {
        return Err(format!("Path '{}' does not exist", target_path_str));
    }

    let canonical = root_path.canonicalize().map_err(|e| e.to_string())?;
    let display_name = canonical.file_name().and_then(|s| s.to_str()).unwrap_or(target_path_str);

    let mut output = String::new();
    output.push_str(&format!("{}/\n", display_name));
    build_dir_tree(&canonical, "", 1, max_depth, show_hidden, &mut output);

    Ok(output)
}

fn build_dir_tree(
    dir: &Path,
    prefix: &str,
    current_depth: usize,
    max_depth: usize,
    show_hidden: bool,
    output: &mut String,
) {
    if current_depth > max_depth {
        return;
    }

    let mut entries: Vec<_> = match std::fs::read_dir(dir) {
        Ok(read) => read.filter_map(|e| e.ok()).collect(),
        Err(_) => return,
    };

    entries.sort_by_key(|e| e.file_name());

    let filtered: Vec<_> = entries
        .into_iter()
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if !show_hidden && name.starts_with('.') {
                return false;
            }
            if matches!(
                name.as_str(),
                "node_modules" | "target" | "dist" | ".git" | ".turbo" | "build" | ".output" | ".next"
            ) {
                return false;
            }
            true
        })
        .collect();

    let count = filtered.len();
    for (i, entry) in filtered.into_iter().enumerate() {
        let is_last = i == count - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();
        let is_dir = path.is_dir();

        if is_dir {
            output.push_str(&format!("{}{}{}/\n", prefix, connector, name));
            let new_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });
            build_dir_tree(&path, &new_prefix, current_depth + 1, max_depth, show_hidden, output);
        } else {
            output.push_str(&format!("{}{}{}\n", prefix, connector, name));
        }
    }
}
