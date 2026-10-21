use serde_json::json;

pub async fn execute_runner_probe() -> Result<String, String> {
    let manifest = json!({
        "protocol": "tuquet.automa.v1",
        "name": "worker",
        "version": env!("CARGO_PKG_VERSION"),
        "engine": "chromium-extension-worker",
        "status": "ready",
        "capabilities": [
            "browser:chromium",
            "mv3_extension_worker",
            "isolation:profile_sandbox",
            "headless",
            "automation:workflow_graph"
        ],
        "plugin_type": "worker_driver",
        "hardware": {
            "cpus": std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1),
            "target_arch": std::env::consts::ARCH
        }
    });

    Ok(serde_json::to_string_pretty(&manifest).unwrap_or_default())
}
