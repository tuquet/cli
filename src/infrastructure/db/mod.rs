use rusqlite::{Connection, Result};
use std::path::Path;

pub mod browsers;
pub mod jobs;
pub mod schema;
pub mod settings;
pub mod tables;
pub mod variables_credentials;
pub mod workflows;

pub use browsers::{Browser, SqliteBrowserRepository};
pub use jobs::{JobDetails, JobHistoryItem, JobInfo, LogItem, SqliteJobRepository};
pub use settings::SqliteSettingsRepository;
pub use tables::SqliteTableRepository;
pub use variables_credentials::SqliteStorageRepository;
pub use workflows::{DbWorkflow, SqliteWorkflowRepository};

pub struct AutomationDb {
    conn: Connection,
}

impl AutomationDb {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        schema::init_db(&conn)?;
        Ok(Self { conn })
    }

    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        schema::init_db(&conn)?;
        Ok(Self { conn })
    }

    pub fn raw_conn(&self) -> &Connection {
        &self.conn
    }

    // --- Sub-Repositories ---

    pub fn jobs(&self) -> SqliteJobRepository<'_> {
        SqliteJobRepository::new(&self.conn)
    }

    pub fn browsers(&self) -> SqliteBrowserRepository<'_> {
        SqliteBrowserRepository::new(&self.conn)
    }

    pub fn workflows(&self) -> SqliteWorkflowRepository<'_> {
        SqliteWorkflowRepository::new(&self.conn)
    }

    pub fn tables(&self) -> SqliteTableRepository<'_> {
        SqliteTableRepository::new(&self.conn)
    }

    pub fn settings(&self) -> SqliteSettingsRepository<'_> {
        SqliteSettingsRepository::new(&self.conn)
    }

    pub fn storage(&self) -> SqliteStorageRepository<'_> {
        SqliteStorageRepository::new(&self.conn)
    }
}
