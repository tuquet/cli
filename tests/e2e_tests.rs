use automa_core::AppState;
use automa_core::api;
use automa_core::config::AppConfig;
use automa_core::infrastructure::db::AutomaDb;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;
use reqwest::Client;
use std::time::Duration;
use tokio_stream::StreamExt;

#[tokio::test]
async fn test_worker_sse_json_validity_e2e() {
    let config = AppConfig::load();
    let db_path = std::env::temp_dir().join("test_automa_worker.sqlite");
    let db = Arc::new(Mutex::new(AutomaDb::new(db_path).unwrap()));

    let (tx, mut _rx) = tokio::sync::broadcast::channel(100);
    let (worker_tx, _) = tokio::sync::broadcast::channel(100);

    let state = AppState {
        db,
        config: Arc::new(config.clone()),
        tx: tx.clone(),
        worker_tx: worker_tx.clone(),
        active_jobs: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
    };

    let app = api::routes::create_router(state.clone());
    
    // Use a random port to avoid conflicts with running daemon
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = Client::new();

    // Register Mock Worker in connected_browsers
    automa_core::api::handlers::jobs::connected_browsers().write().await.insert("daemon_worker".to_string());

    // 1. Start a Mock Worker to listen to the SSE stream
    let mut sse_stream = client.get(format!("http://127.0.0.1:{}/api/internal/worker/events?browserId=daemon_worker", port))
        .send()
        .await
        .unwrap()
        .bytes_stream();

    // Give the SSE connection a moment to establish
    tokio::time::sleep(Duration::from_millis(500)).await;

    // 2. Create a Workflow JSON file with CRLF (\r\n) to simulate Windows pretty-printing
    let workflow_path = std::env::temp_dir().join("test_crlf.workflow.json");
    let workflow_json_str = "{\r\n  \"id\": \"wf_crlf_123\",\r\n  \"name\": \"CRLF Test\",\r\n  \"nodes\": []\r\n}";
    tokio::fs::write(&workflow_path, workflow_json_str).await.unwrap();

    // 3. Submit the job
    let payload = serde_json::json!({
        "workflowPath": workflow_path.to_string_lossy().to_string()
    });

    let res = client.post(format!("http://127.0.0.1:{}/api/jobs", port))
        .json(&payload)
        .send()
        .await
        .unwrap();

    assert!(res.status().is_success());
    let res_json: serde_json::Value = res.json().await.unwrap();
    let job_id = res_json["jobId"].as_str().unwrap().to_string();

    // 4. The Mock Worker reads the SSE stream
    let mut parsed_successfully = false;
    let mut raw_data = String::new();
    
    // Read the stream chunks
    if let Some(Ok(bytes)) = sse_stream.next().await {
        let chunk = String::from_utf8_lossy(&bytes);
        println!("Received SSE Chunk: {}", chunk);
        
        // SSE chunks look like "data: {...}\n\n"
        for line in chunk.split("\n") {
            if line.starts_with("data: ") {
                let json_str = &line[6..];
                raw_data = json_str.to_string();
                
                // Assert that the JSON parses perfectly
                match serde_json::from_str::<serde_json::Value>(json_str) {
                    Ok(parsed) => {
                        assert_eq!(parsed["jobId"], job_id);
                        assert_eq!(parsed["workflowData"]["id"], "wf_crlf_123");
                        parsed_successfully = true;
                    },
                    Err(e) => {
                        panic!("Worker Mock failed to parse SSE JSON! Error: {} | Data: {}", e, json_str);
                    }
                }
            }
        }
    }

    assert!(parsed_successfully, "Worker Mock did not receive or parse the SSE event.");
    assert!(!raw_data.contains("\n"), "The JSON payload MUST NOT contain literal newlines to avoid SSE framing issues.");

    // 5. Mock Worker completes the job
    let finish_res = client.patch(format!("http://127.0.0.1:{}/api/jobs/{}/status", port, job_id))
        .send()
        .await
        .unwrap();
    assert!(finish_res.status().is_success());

    // 6. Verify Job is removed from active jobs
    let status_res = client.get(format!("http://127.0.0.1:{}/api/jobs/{}/status", port, job_id))
        .send()
        .await
        .unwrap();
    let status_json: serde_json::Value = status_res.json().await.unwrap();
    assert_eq!(status_json["status"], "completed");

    server.abort();
}

#[tokio::test]
async fn test_submit_job_passes_options() {
    let config = AppConfig::load();
    let db_path = std::env::temp_dir().join("test_log_options.sqlite");
    let db = Arc::new(Mutex::new(AutomaDb::new(db_path).unwrap()));

    let (tx, _rx) = tokio::sync::broadcast::channel(10);
    let (worker_tx, mut worker_rx) = tokio::sync::broadcast::channel(100);

    let state = AppState {
        db,
        config: Arc::new(config.clone()),
        tx: tx.clone(),
        worker_tx: worker_tx.clone(),
        active_jobs: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
    };

    let app = api::routes::create_router(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = Client::new();

    // Register Mock Worker in connected_browsers
    automa_core::api::handlers::jobs::connected_browsers().write().await.insert("daemon_worker".to_string());

    // Start a Mock Worker SSE listener so submit_job doesn't need to launch real Chrome
    let mut _sse_stream = client.get(format!("http://127.0.0.1:{}/api/internal/worker/events?browserId=daemon_worker", port))
        .send()
        .await
        .unwrap()
        .bytes_stream();
    tokio::time::sleep(Duration::from_millis(300)).await;

    let workflow_path = std::env::temp_dir().join("test_options.workflow.json");
    tokio::fs::write(&workflow_path, r#"{"id":"wf_opt_123","nodes":[]}"#).await.unwrap();

    let payload = serde_json::json!({
        "workflowPath": workflow_path.to_string_lossy().to_string(),
        "options": {
            "browserId": "daemon_worker",
            "variables": {
                "keyword": "worldcup messi"
            }
        }
    });

    let res = client.post(format!("http://127.0.0.1:{}/api/jobs", port))
        .json(&payload)
        .send()
        .await
        .unwrap();

    assert!(res.status().is_success());

    // Listen to the broadcast channel to see what was sent to the worker
    let msg = worker_rx.recv().await.unwrap();
    let parsed_msg: serde_json::Value = serde_json::from_str(&msg).unwrap();

    assert_eq!(parsed_msg["options"]["variables"]["keyword"], "worldcup messi");
    assert_eq!(parsed_msg["workflowData"]["id"], "wf_opt_123");

    server.abort();
}
