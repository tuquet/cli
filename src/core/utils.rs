use std::path::PathBuf;
use std::env;

pub fn resolve_extension_path() -> String {
    let mut possible_paths = vec![
        PathBuf::from("apps/webe/dist/cli-runner"),
        PathBuf::from("./apps/webe/dist/cli-runner"),
        PathBuf::from("../webe/dist/cli-runner"),
        PathBuf::from("../../apps/webe/dist/cli-runner"),
        PathBuf::from("../../../apps/webe/dist/cli-runner"),
        PathBuf::from("automa-webe/dist/cli-runner"),
        PathBuf::from("./automa-webe/dist/cli-runner"),
        PathBuf::from("../automa-webe/dist/cli-runner"),
        PathBuf::from("../../automa-webe/dist/cli-runner"),
    ];

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            possible_paths.push(exe_dir.join("apps/webe/dist/cli-runner"));
            possible_paths.push(exe_dir.join("../webe/dist/cli-runner"));
            possible_paths.push(exe_dir.join("../../apps/webe/dist/cli-runner"));
            possible_paths.push(exe_dir.join("../../../apps/webe/dist/cli-runner"));
            possible_paths.push(exe_dir.join("automa-webe/dist/cli-runner"));
            possible_paths.push(exe_dir.join("../automa-webe/dist/cli-runner"));
        }
    }

    if let Ok(env_ext) = env::var("AUTOMA_EXTENSION_PATH") {
        possible_paths.insert(0, PathBuf::from(env_ext));
    }

    let mut ext_path = possible_paths.into_iter()
        .find(|p| p.exists())
        .and_then(|p| p.canonicalize().ok())
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "apps/webe/dist/cli-runner".to_string());

    if ext_path.starts_with(r"\\?\") {
        ext_path = ext_path[4..].to_string();
    }
    
    ext_path
}
