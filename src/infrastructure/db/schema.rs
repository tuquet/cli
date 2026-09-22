use rusqlite::{Connection, Result};

pub fn init_db(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS jobs (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            data TEXT NOT NULL, -- JSON
            options TEXT, -- JSON
            status TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            job_id TEXT NOT NULL,
            type TEXT NOT NULL,
            message TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(job_id) REFERENCES jobs(id) ON DELETE CASCADE
        );
        
        CREATE INDEX IF NOT EXISTS idx_jobs_status ON jobs(status);
        CREATE INDEX IF NOT EXISTS idx_logs_job_id ON logs(job_id);

        CREATE TABLE IF NOT EXISTS browsers (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            user_agent TEXT,
            timezone TEXT,
            proxy TEXT,
            browser_type TEXT DEFAULT 'Chromium',
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS workflows (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            data TEXT NOT NULL, -- JSON AST (nodes, edges, settings)
            version TEXT DEFAULT '1.0.0',
            icon TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE INDEX IF NOT EXISTS idx_workflows_name ON workflows(name);

        CREATE TABLE IF NOT EXISTS campaigns (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            data TEXT NOT NULL, -- JSON (members, tasks, matrix, concurrency, etc.)
            cron TEXT,
            version TEXT DEFAULT '1.0.0',
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE INDEX IF NOT EXISTS idx_campaigns_name ON campaigns(name);

        CREATE TABLE IF NOT EXISTS storage_tables (
            id TEXT PRIMARY KEY,
            name TEXT,
            columns TEXT,
            columns_index TEXT,
            created_at INTEGER,
            modified_at INTEGER
        );

        CREATE TABLE IF NOT EXISTS storage_table_rows (
            id TEXT PRIMARY KEY,
            table_id TEXT NOT NULL,
            data TEXT NOT NULL,
            FOREIGN KEY(table_id) REFERENCES storage_tables(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_table_rows_table_id ON storage_table_rows(table_id);

        CREATE TABLE IF NOT EXISTS system_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS storage_variables (
            id TEXT PRIMARY KEY,
            name TEXT UNIQUE NOT NULL,
            value TEXT NOT NULL,
            created_at INTEGER,
            updated_at INTEGER
        );

        CREATE TABLE IF NOT EXISTS storage_credentials (
            id TEXT PRIMARY KEY,
            name TEXT UNIQUE NOT NULL,
            value TEXT NOT NULL,
            created_at INTEGER,
            updated_at INTEGER
        );
        "#
    )?;
    Ok(())
}
