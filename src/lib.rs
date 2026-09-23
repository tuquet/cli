pub mod api;
pub mod cli;
pub mod config;
pub mod core;
pub mod infrastructure;

use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;
use tokio_util::sync::CancellationToken;
use infrastructure::db::AutomaDb;
use config::AppConfig;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<AutomaDb>>,
    pub config: Arc<AppConfig>,
    pub tx: tokio::sync::broadcast::Sender<String>,
    pub worker_tx: tokio::sync::broadcast::Sender<String>,
    pub active_jobs: Arc<tokio::sync::RwLock<HashMap<String, CancellationToken>>>,
}
