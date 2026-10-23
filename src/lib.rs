pub mod api;
pub mod cli;
pub mod commands;
pub mod config;
pub mod constants;
pub mod core;
pub mod infrastructure;
pub mod ui;

use config::AppConfig;
use infrastructure::db::AutomationDb;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<AutomationDb>>,
    pub config: Arc<AppConfig>,
    pub tx: tokio::sync::broadcast::Sender<String>,
    pub worker_tx: tokio::sync::broadcast::Sender<String>,
    pub active_jobs: Arc<tokio::sync::RwLock<HashMap<String, CancellationToken>>>,
}
