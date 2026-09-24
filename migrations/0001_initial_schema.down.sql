-- ============================================================================
-- AUTOMA CORE: LOCAL SQLITE ENGINE (DOWN MIGRATION - ROLLBACK)
-- Version: 0001_initial_schema
-- Target: SQLite 3
-- Scope: Revert initial local schema
-- ============================================================================

-- migrate:down

DROP INDEX IF EXISTS idx_table_rows_table_id;
DROP INDEX IF EXISTS idx_campaigns_name;
DROP INDEX IF EXISTS idx_workflows_name;
DROP INDEX IF EXISTS idx_logs_job_id;
DROP INDEX IF EXISTS idx_jobs_status;

DROP TABLE IF EXISTS storage_credentials;
DROP TABLE IF EXISTS storage_variables;
DROP TABLE IF EXISTS system_settings;
DROP TABLE IF EXISTS storage_table_rows;
DROP TABLE IF EXISTS storage_tables;
DROP TABLE IF EXISTS campaigns;
DROP TABLE IF EXISTS workflows;
DROP TABLE IF EXISTS browsers;
DROP TABLE IF EXISTS logs;
DROP TABLE IF EXISTS jobs;
