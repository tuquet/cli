use serde_json::Value;

pub async fn execute_specter_faker_generate(args: &Value) -> Result<String, String> {
    let count = args.get("count").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
    let gender = args.get("gender").and_then(|v| v.as_str());
    let nat = args.get("nat").and_then(|v| v.as_str()).unwrap_or("VN");
    let avatar = args.get("avatar").and_then(|v| v.as_str()).unwrap_or("real");
    let domain = args.get("domain").and_then(|v| v.as_str());
    let format_str = args.get("format").and_then(|v| v.as_str()).unwrap_or("json");
    let output_file = args.get("output_file").and_then(|v| v.as_str());

    let gender_opt = gender.filter(|&g| g != "all");
    let nat_opt = if nat == "all" { None } else { Some(nat) };

    let data = specter_faker::generate_users(count, gender_opt, nat_opt, Some(avatar), domain);
    let users = data["results"]
        .as_array()
        .ok_or_else(|| "Failed to generate user list: invalid result structure".to_string())?;

    let output_text = match format_str.to_lowercase().as_str() {
        "csv" => specter_faker::to_csv(users),
        _ => specter_faker::to_json(users, true).map_err(|e| e.to_string())?,
    };

    if let Some(path_str) = output_file {
        std::fs::write(path_str, output_text.as_bytes())
            .map_err(|e| format!("Failed to write output file '{}': {}", path_str, e))?;
        let preview_len = output_text.len().min(500);
        Ok(format!(
            "Successfully generated {} synthetic profile(s) and saved to '{}'.\n\nPreview (first 500 chars):\n{}",
            users.len(),
            path_str,
            &output_text[..preview_len]
        ))
    } else {
        Ok(output_text)
    }
}
