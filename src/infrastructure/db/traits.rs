use rusqlite::Result;
use serde_json::Value;
use crate::core::models::settings::AppSettings;
use crate::core::models::storage::{StorageCredential, StorageTable, StorageVariable, TableRow};
use crate::infrastructure::db::browsers::Browser;
use crate::infrastructure::db::campaigns::DbCampaign;
use crate::infrastructure::db::jobs::{JobDetails, JobHistoryItem, LogItem};
use crate::infrastructure::db::workflows::DbWorkflow;

pub trait WorkflowRepository {
    fn create_workflow(
        &self,
        id: &str,
        name: &str,
        description: Option<&str>,
        data: &str,
        version: Option<&str>,
        icon: Option<&str>,
    ) -> Result<DbWorkflow>;

    fn get_workflows(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
        search: Option<&str>,
    ) -> Result<Vec<DbWorkflow>>;

    fn get_workflow(&self, id: &str) -> Result<Option<DbWorkflow>>;

    fn update_workflow(
        &self,
        id: &str,
        name: Option<&str>,
        description: Option<&str>,
        data: Option<&str>,
        version: Option<&str>,
        icon: Option<&str>,
    ) -> Result<Option<DbWorkflow>>;

    fn upsert_workflow(
        &self,
        id: &str,
        name: Option<&str>,
        description: Option<&str>,
        data: Option<&str>,
        version: Option<&str>,
        icon: Option<&str>,
    ) -> Result<DbWorkflow>;

    fn delete_workflow(&self, id: &str) -> Result<bool>;
}

pub trait CampaignRepository {
    fn create_campaign(
        &self,
        id: &str,
        name: &str,
        description: Option<&str>,
        data: &str,
        cron: Option<&str>,
        version: Option<&str>,
    ) -> Result<DbCampaign>;

    fn get_campaigns(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
        search: Option<&str>,
    ) -> Result<Vec<DbCampaign>>;

    fn get_campaign(&self, id: &str) -> Result<Option<DbCampaign>>;

    fn update_campaign(
        &self,
        id: &str,
        name: Option<&str>,
        description: Option<&str>,
        data: Option<&str>,
        cron: Option<&str>,
        version: Option<&str>,
    ) -> Result<Option<DbCampaign>>;

    fn upsert_campaign(
        &self,
        id: &str,
        name: Option<&str>,
        description: Option<&str>,
        data: Option<&str>,
        cron: Option<&str>,
        version: Option<&str>,
    ) -> Result<DbCampaign>;

    fn delete_campaign(&self, id: &str) -> Result<bool>;
}

pub trait BrowserRepository {
    fn create_browser(
        &self,
        id: &str,
        name: &str,
        user_agent: Option<&str>,
        timezone: Option<&str>,
    ) -> Result<()>;

    fn get_browsers(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
        search: Option<&str>,
    ) -> Result<Vec<Browser>>;

    fn get_browser(&self, id: &str) -> Result<Option<Browser>>;

    fn update_browser(
        &self,
        id: &str,
        name: &str,
        user_agent: Option<&str>,
        timezone: Option<&str>,
    ) -> Result<()>;

    fn delete_browser(&self, id: &str) -> Result<()>;
}

pub trait JobRepository {
    fn create_job(
        &self,
        job_id: &str,
        name: &str,
        data: &Value,
        options: &Value,
        status: &str,
    ) -> bool;

    fn update_job_status(&self, job_id: &str, status: &str) -> Result<()>;

    fn finish_job(&self, job_id: &str, status: &str, results: &Value, duration: i64) -> Result<()>;

    fn cleanup_old_jobs(&self) -> Result<()>;

    fn delete_job(&self, job_id: &str) -> Result<()>;

    fn clear_all_jobs(&self) -> bool;

    fn get_history(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
        search: Option<&str>,
        status: Option<&str>,
    ) -> Result<Vec<JobHistoryItem>>;

    fn get_job_details(&self, job_id: &str) -> Result<JobDetails>;

    fn insert_log(&self, job_id: &str, log_type: &str, message: &str) -> Result<()>;

    fn add_logs(&mut self, logs: &[(&str, &str, &str)]) -> Result<()>;

    fn get_job_logs(&self, job_id: &str) -> Result<Vec<LogItem>>;

    fn get_parsed_job_logs(&self, job_id: &str) -> Result<Vec<Value>>;
}

pub trait TableRepository {
    fn get_tables(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
        search: Option<&str>,
    ) -> Result<Vec<StorageTable>>;

    fn save_table(&self, table: &StorageTable) -> Result<()>;

    fn delete_table(&self, id: &str) -> Result<()>;

    fn get_table_rows(
        &self,
        table_id: &str,
        limit: Option<usize>,
        offset: Option<usize>,
        search: Option<&str>,
    ) -> Result<Vec<TableRow>>;

    fn add_table_row(&self, id: &str, table_id: &str, data: &str) -> Result<()>;
}

pub trait StorageRepository {
    fn get_variables(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
        search: Option<&str>,
    ) -> Result<Vec<StorageVariable>>;

    fn save_variable(&self, var: &StorageVariable) -> Result<StorageVariable>;

    fn delete_variable(&self, id_or_name: &str) -> Result<()>;

    fn get_credentials(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
        search: Option<&str>,
    ) -> Result<Vec<StorageCredential>>;

    fn save_credential(&self, cred: &StorageCredential) -> Result<StorageCredential>;

    fn delete_credential(&self, id_or_name: &str) -> Result<()>;
}

pub trait SettingsRepository {
    fn get_settings(&self) -> Result<AppSettings>;
    fn save_settings(&self, settings: &AppSettings) -> Result<()>;
}
