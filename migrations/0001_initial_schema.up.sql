-- ============================================================================
-- AUTOMA CORE: LOCAL SQLITE ENGINE (UP MIGRATION)
-- Version: 0001_initial_schema
-- Target: SQLite 3 (WAL Mode, Foreign Keys Enabled)
-- Scope: Client-side local daemon runtime (offline-first execution)
-- ============================================================================

-- migrate:up

-- 1. Jobs Table
CREATE TABLE IF NOT EXISTS jobs (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    data TEXT NOT NULL, -- JSON
    options TEXT, -- JSON
    status TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_jobs_status ON jobs(status);

-- 2. Logs Table
CREATE TABLE IF NOT EXISTS logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id TEXT NOT NULL,
    type TEXT NOT NULL,
    message TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(job_id) REFERENCES jobs(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_logs_job_id ON logs(job_id);

-- 3. Browsers Table
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

-- 4. Workflows Table (Local AST)
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

-- 5. Campaigns Table
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

-- 6. Storage Tables & Rows
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

-- 7. System Settings
CREATE TABLE IF NOT EXISTS system_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 8. Storage Variables & Encrypted Credentials
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
