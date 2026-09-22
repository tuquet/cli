use rusqlite::{params, Connection, OptionalExtension, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobHistoryItem {
    pub id: String,
    pub name: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobInfo {
    pub id: String,
    pub name: String,
    pub workflow_id: String,
    pub status: String,
    pub created_at: String,
    pub ended_at: String,
    pub duration: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct JobDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job: Option<JobInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub results: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logs: Option<Vec<Value>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LogItem {
    pub id: i64,
    pub job_id: String,
    #[serde(rename = "type")]
    pub log_type: String,
    pub message: String,
    pub created_at: String,
}


pub struct SqliteJobRepository<'a> {
    pub conn: &'a Connection,
}

impl<'a> SqliteJobRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn create_job(&self, job_id: &str, name: &str, data: &Value, options: &Value, status: &str) -> bool {
        let data_str = serde_json::to_string(data).unwrap_or_else(|_| "{}".to_string());
        let options_str = serde_json::to_string(options).unwrap_or_else(|_| "{}".to_string());
        
        self.conn.execute(
            "INSERT INTO jobs (id, name, data, options, status) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![job_id, name, data_str, options_str, status],
        ).is_ok()
    }

    pub fn update_job_status(&self, job_id: &str, status: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE jobs SET status = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            params![status, job_id],
        )?;
        Ok(())
    }

    pub fn finish_job(&self, job_id: &str, status: &str, results: &Value, duration: i64) -> Result<()> {
        let row_res: Result<String> = self.conn.query_row(
            "SELECT data FROM jobs WHERE id = ?1",
            params![job_id],
            |row| row.get(0),
        );

        if let Ok(data_str) = row_res {
            if let Ok(mut data) = serde_json::from_str::<Value>(&data_str) {
                if let Some(obj) = data.as_object_mut() {
                    obj.insert("results".to_string(), results.clone());
                    obj.insert("duration".to_string(), json!(duration));
                }
                let new_data_str = serde_json::to_string(&data).unwrap_or_default();
                self.conn.execute(
                    "UPDATE jobs SET status = ?1, data = ?2, updated_at = CURRENT_TIMESTAMP WHERE id = ?3",
                    params![status, new_data_str, job_id],
                )?;
            } else {
                self.conn.execute(
                    "UPDATE jobs SET status = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                    params![status, job_id],
                )?;
            }
        } else {
            self.conn.execute(
                "UPDATE jobs SET status = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                params![status, job_id],
            )?;
        }
        Ok(())
    }

    pub fn cleanup_old_jobs(&self) -> Result<()> {
        self.conn.execute(
            r#"
            DELETE FROM logs WHERE job_id NOT IN (
                SELECT id FROM jobs ORDER BY rowid DESC LIMIT 100
            )
            "#,
            [],
        )?;

        self.conn.execute(
            r#"
            DELETE FROM jobs WHERE rowid NOT IN (
                SELECT rowid FROM jobs ORDER BY rowid DESC LIMIT 100
            )
            "#,
            [],
        )?;
        Ok(())
    }

    pub fn delete_job(&self, job_id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM logs WHERE job_id = ?1", params![job_id])?;
        self.conn.execute("DELETE FROM jobs WHERE id = ?1", params![job_id])?;
        Ok(())
    }

    pub fn clear_all_jobs(&self) -> bool {
        let res1 = self.conn.execute("DELETE FROM logs", []);
        let res2 = self.conn.execute("DELETE FROM jobs", []);
        res1.is_ok() && res2.is_ok()
    }

    pub fn get_history(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
        search: Option<&str>,
        status: Option<&str>,
    ) -> Result<Vec<JobHistoryItem>> {
        let l = limit.map(|v| v as i64).unwrap_or(50);
        let o = offset.unwrap_or(0) as i64;
        let mut history = Vec::new();

        let mut conditions = Vec::new();
        let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(s) = search {
            conditions.push("(id LIKE ? OR name LIKE ?)");
            let pattern = format!("%{}%", s);
            param_values.push(Box::new(pattern.clone()));
            param_values.push(Box::new(pattern));
        }

        if let Some(st) = status {
            conditions.push("status = ?");
            param_values.push(Box::new(st.to_string()));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let sql = format!(
            "SELECT id, name, status, created_at, updated_at FROM jobs {} ORDER BY rowid DESC LIMIT ? OFFSET ?",
            where_clause
        );

        param_values.push(Box::new(l));
        param_values.push(Box::new(o));

        let params_slice: Vec<&dyn rusqlite::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = self.conn.prepare(&sql)?;
        let iter = stmt.query_map(params_slice.as_slice(), |row| {
            Ok(JobHistoryItem {
                id: row.get(0)?,
                name: row.get(1)?,
                status: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;
        for item in iter {
            history.push(item?);
        }
        Ok(history)
    }

    pub fn get_job_details(&self, job_id: &str) -> Result<JobDetails> {
        let job_res = self.conn.query_row(
            "SELECT id, name, status, data, created_at, updated_at FROM jobs WHERE id = ?1",
            params![job_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        ).optional();

        match job_res {
            Ok(Some((id, name, status, data_str, created_at, updated_at))) => {
                let data_val = serde_json::from_str::<Value>(&data_str).unwrap_or_else(|_| json!({}));
                let workflow_id = data_val
                    .get("workflowData")
                    .and_then(|w| w.get("id"))
                    .and_then(|id| id.as_str())
                    .unwrap_or(&id)
                    .to_string();
                let duration = data_val.get("duration").and_then(|d| d.as_i64());
                let results = data_val.get("results").cloned().unwrap_or_else(|| json!({"table": [], "variables": {}}));
                
                let job_info = JobInfo {
                    id,
                    name,
                    workflow_id,
                    status,
                    created_at,
                    ended_at: updated_at,
                    duration,
                };

                let logs = self.get_parsed_job_logs(job_id)?;

                Ok(JobDetails {
                    job: Some(job_info),
                    results: Some(results),
                    logs: Some(logs),
                    error: None,
                })
            }
            Ok(None) => Ok(JobDetails {
                job: None,
                results: None,
                logs: None,
                error: Some("Job not found".to_string()),
            }),
            Err(e) => Err(e)
        }
    }

    pub fn insert_log(&self, job_id: &str, log_type: &str, message: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO logs (job_id, type, message) VALUES (?1, ?2, ?3)",
            params![job_id, log_type, message],
        )?;
        Ok(())
    }

    pub fn add_logs(&mut self, logs: &[(&str, &str, &str)]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        for (job_id, log_type, message) in logs {
            tx.execute(
                "INSERT INTO logs (job_id, type, message) VALUES (?1, ?2, ?3)",
                params![job_id, log_type, message],
            )?;
        }
        tx.commit()?; 
        Ok(())
    }

    pub fn get_job_logs(&self, job_id: &str) -> Result<Vec<LogItem>> {
        let mut logs = Vec::new();
        let mut stmt = self.conn.prepare("SELECT id, job_id, type, message, created_at FROM logs WHERE job_id = ?1 ORDER BY id ASC")?;
        let iter = stmt.query_map(params![job_id], |row| {
            Ok(LogItem {
                id: row.get(0)?,
                job_id: row.get(1)?,
                log_type: row.get(2)?,
                message: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;
        for item in iter {
            logs.push(item?);
        }
        Ok(logs)
    }

    pub fn get_parsed_job_logs(&self, job_id: &str) -> Result<Vec<Value>> {
        let mut logs = Vec::new();
        let mut stmt = self.conn.prepare("SELECT id, type, message, created_at FROM logs WHERE job_id = ?1 ORDER BY id ASC")?;
        let iter = stmt.query_map(params![job_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        for item in iter {
            let item = item?;
            let mut log_obj = json!({
                "id": item.0,
                "type": item.1,
                "message": item.2,
                "created_at": item.3
            });

            if let Ok(parsed_msg) = serde_json::from_str::<Value>(&item.2) {
                if let (Some(obj), Some(parsed_obj)) = (log_obj.as_object_mut(), parsed_msg.as_object()) {
                    for (k, v) in parsed_obj {
                        obj.insert(k.clone(), v.clone());
                    }
                }
            }
            logs.push(log_obj);
        }
        Ok(logs)
    }
}

impl<'a> crate::infrastructure::db::traits::JobRepository for SqliteJobRepository<'a> {
    fn create_job(
        &self,
        job_id: &str,
        name: &str,
        data: &Value,
        options: &Value,
        status: &str,
    ) -> bool {
        self.create_job(job_id, name, data, options, status)
    }

    fn update_job_status(&self, job_id: &str, status: &str) -> Result<()> {
        self.update_job_status(job_id, status)
    }

    fn finish_job(&self, job_id: &str, status: &str, results: &Value, duration: i64) -> Result<()> {
        self.finish_job(job_id, status, results, duration)
    }

    fn cleanup_old_jobs(&self) -> Result<()> {
        self.cleanup_old_jobs()
    }

    fn delete_job(&self, job_id: &str) -> Result<()> {
        self.delete_job(job_id)
    }

    fn clear_all_jobs(&self) -> bool {
        self.clear_all_jobs()
    }

    fn get_history(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
        search: Option<&str>,
        status: Option<&str>,
    ) -> Result<Vec<JobHistoryItem>> {
        self.get_history(limit, offset, search, status)
    }

    fn get_job_details(&self, job_id: &str) -> Result<JobDetails> {
        self.get_job_details(job_id)
    }

    fn insert_log(&self, job_id: &str, log_type: &str, message: &str) -> Result<()> {
        self.insert_log(job_id, log_type, message)
    }

    fn add_logs(&mut self, logs: &[(&str, &str, &str)]) -> Result<()> {
        self.add_logs(logs)
    }

    fn get_job_logs(&self, job_id: &str) -> Result<Vec<LogItem>> {
        self.get_job_logs(job_id)
    }

    fn get_parsed_job_logs(&self, job_id: &str) -> Result<Vec<Value>> {
        self.get_parsed_job_logs(job_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::db::AutomaDb;

    #[test]
    fn test_sqlite_jobs_lifecycle_and_logs() {
        let db = AutomaDb::new_in_memory().unwrap();
        let conn = db.raw_conn();
        let repo = SqliteJobRepository::new(conn);

        // 1. Create job
        let created = repo.create_job("job_abc", "My Workflow Job", &json!({"step": 1}), &json!({"headless": true}), "pending");
        assert!(created);

        // 2. Add logs
        repo.insert_log("job_abc", "info", "Starting execution").unwrap();
        repo.insert_log("job_abc", "success", r#"{"duration": 150, "status": "ok"}"#).unwrap();

        // 3. Update status & finish job
        repo.update_job_status("job_abc", "running").unwrap();
        repo.finish_job("job_abc", "completed", &json!({"items_extracted": 5}), 1500).unwrap();

        // 4. Retrieve job details & logs
        let details = repo.get_job_details("job_abc").unwrap();
        assert_eq!(details.job.as_ref().unwrap().status, "completed");

        let logs = repo.get_job_logs("job_abc").unwrap();
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].log_type, "info");

        let parsed_logs = repo.get_parsed_job_logs("job_abc").unwrap();
        assert_eq!(parsed_logs.len(), 2);
        assert_eq!(parsed_logs[1]["status"], "ok");

        // 5. Query history & clear
        let history = repo.get_history(Some(10), None, None, None).unwrap();
        assert_eq!(history.len(), 1);

        assert!(repo.clear_all_jobs());
        let history_after = repo.get_history(Some(10), None, None, None).unwrap();
        assert!(history_after.is_empty());
    }
}
