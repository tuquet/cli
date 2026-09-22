use crate::core::error::AutomaError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;
use uuid::Uuid;
use crate::api::handlers::storage::{StorageVariable, StorageCredential}; // For now, borrow models

#[derive(Serialize, Deserialize, Default)]
pub struct StorageSettings {
    #[serde(default)]
    pub variables: Vec<StorageVariable>,
    #[serde(default)]
    pub credentials: Vec<StorageCredential>,
}

pub struct FsStorageManager;

impl FsStorageManager {
    async fn get_globals_file_path(filename: &str) -> PathBuf {
        let config = crate::config::AppConfig::load();
        let globals_dir = PathBuf::from(&config.data_dir).join("globals");
        if !globals_dir.exists() {
            let _ = fs::create_dir_all(&globals_dir).await;
        }
        globals_dir.join(filename)
    }

    pub async fn load_storage_settings() -> StorageSettings {
        let path = Self::get_globals_file_path("storage.setting.json").await;
        if let Ok(content) = fs::read_to_string(&path).await {
            if let Ok(settings) = serde_json::from_str::<StorageSettings>(&content) {
                return settings;
            }
        }
        StorageSettings::default()
    }

    pub async fn save_storage_settings(settings: &StorageSettings) -> Result<(), AutomaError> {
        let path = Self::get_globals_file_path("storage.setting.json").await;
        let content = serde_json::to_string_pretty(settings)
            .map_err(|e| AutomaError::JsonError(e))?;
        fs::write(&path, content)
            .await
            .map_err(|e| AutomaError::IoError(e))?;
        Ok(())
    }

    pub fn generate_id() -> String {
        Uuid::new_v4().to_string()
    }

    pub async fn get_variables() -> Result<Vec<StorageVariable>, AutomaError> {
        let settings = Self::load_storage_settings().await;
        Ok(settings.variables)
    }

    pub async fn add_variable(mut payload: StorageVariable) -> Result<StorageVariable, AutomaError> {
        let mut settings = Self::load_storage_settings().await;
        let key = payload.name.clone().or_else(|| payload.key.clone()).unwrap_or_default();
        
        if payload.id.is_none() { payload.id = Some(Self::generate_id()); }
        if payload.name.is_none() { payload.name = Some(key.clone()); }

        if let Some(existing) = settings.variables.iter_mut().find(|v| v.name.as_deref() == Some(&key) || v.key.as_deref() == Some(&key)) {
            existing.value = payload.value.clone();
        } else {
            settings.variables.push(payload.clone());
        }

        Self::save_storage_settings(&settings).await?;
        Ok(payload)
    }

    pub async fn delete_variable(id: &str) -> Result<(), AutomaError> {
        let mut settings = Self::load_storage_settings().await;
        settings.variables.retain(|v| v.id.as_deref() != Some(id) && v.name.as_deref() != Some(id) && v.key.as_deref() != Some(id));
        Self::save_storage_settings(&settings).await?;
        Ok(())
    }

    pub async fn get_credentials() -> Result<Vec<StorageCredential>, AutomaError> {
        let settings = Self::load_storage_settings().await;
        Ok(settings.credentials)
    }

    pub async fn add_credential(mut payload: StorageCredential) -> Result<StorageCredential, AutomaError> {
        let mut settings = Self::load_storage_settings().await;
        let key = payload.name.clone().or_else(|| payload.key.clone()).unwrap_or_default();
        
        if payload.id.is_none() { payload.id = Some(Self::generate_id()); }
        if payload.name.is_none() { payload.name = Some(key.clone()); }

        if let Some(existing) = settings.credentials.iter_mut().find(|v| v.name.as_deref() == Some(&key) || v.key.as_deref() == Some(&key)) {
            existing.value = payload.value.clone();
        } else {
            settings.credentials.push(payload.clone());
        }

        Self::save_storage_settings(&settings).await?;
        Ok(payload)
    }

    pub async fn delete_credential(id: &str) -> Result<(), AutomaError> {
        let mut settings = Self::load_storage_settings().await;
        settings.credentials.retain(|v| v.id.as_deref() != Some(id) && v.name.as_deref() != Some(id) && v.key.as_deref() != Some(id));
        Self::save_storage_settings(&settings).await?;
        Ok(())
    }
}
