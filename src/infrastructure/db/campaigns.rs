use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DbCampaign {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub data: String,
    pub cron: Option<String>,
    pub version: String,
    pub created_at: String,
    pub updated_at: String,
}

pub struct SqliteCampaignRepository<'a> {
    pub conn: &'a Connection,
}

impl<'a> SqliteCampaignRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn create_campaign(
        &self,
        id: &str,
        name: &str,
        description: Option<&str>,
        data: &str,
        cron: Option<&str>,
        version: Option<&str>,
    ) -> Result<DbCampaign> {
        let ver = version.unwrap_or("1.0.0");
        self.conn.execute(
            r#"
            INSERT INTO campaigns (id, name, description, data, cron, version, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                description = excluded.description,
                data = excluded.data,
                cron = excluded.cron,
                version = excluded.version,
                updated_at = CURRENT_TIMESTAMP
            "#,
            params![id, name, description, data, cron, ver],
        )?;

        self.get_campaign(id)?.ok_or_else(|| {
            rusqlite::Error::QueryReturnedNoRows
        })
    }

    pub fn get_campaigns(&self, limit: Option<usize>, offset: Option<usize>, search: Option<&str>) -> Result<Vec<DbCampaign>> {
        let l = limit.map(|v| v as i64).unwrap_or(-1);
        let o = offset.unwrap_or(0) as i64;
        let mut list = Vec::new();

        if let Some(s) = search {
            let pattern = format!("%{}%", s);
            let mut stmt = self.conn.prepare(
                "SELECT id, name, description, data, cron, version, created_at, updated_at FROM campaigns WHERE (name LIKE ?1 OR id LIKE ?1 OR description LIKE ?1) ORDER BY updated_at DESC LIMIT ?2 OFFSET ?3"
            )?;
            let rows = stmt.query_map(params![pattern, l, o], |row| {
                Ok(DbCampaign {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    data: row.get(3)?,
                    cron: row.get(4)?,
                    version: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            })?;
            for r in rows {
                list.push(r?);
            }
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT id, name, description, data, cron, version, created_at, updated_at FROM campaigns ORDER BY updated_at DESC LIMIT ?1 OFFSET ?2"
            )?;
            let rows = stmt.query_map(params![l, o], |row| {
                Ok(DbCampaign {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    data: row.get(3)?,
                    cron: row.get(4)?,
                    version: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            })?;
            for r in rows {
                list.push(r?);
            }
        }

        Ok(list)
    }

    pub fn get_campaign(&self, id: &str) -> Result<Option<DbCampaign>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, data, cron, version, created_at, updated_at FROM campaigns WHERE id = ?1"
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(DbCampaign {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                data: row.get(3)?,
                cron: row.get(4)?,
                version: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;

        if let Some(c) = rows.next() {
            Ok(Some(c?))
        } else {
            Ok(None)
        }
    }

    pub fn update_campaign(
        &self,
        id: &str,
        name: Option<&str>,
        description: Option<&str>,
        data: Option<&str>,
        cron: Option<&str>,
        version: Option<&str>,
    ) -> Result<Option<DbCampaign>> {
        let current = match self.get_campaign(id)? {
            Some(c) => c,
            None => return Ok(None),
        };

        let new_name = name.unwrap_or(&current.name);
        let new_desc = description.or(current.description.as_deref());
        let new_data = data.unwrap_or(&current.data);
        let new_cron = cron.or(current.cron.as_deref());
        let new_ver = version.unwrap_or(&current.version);

        self.conn.execute(
            r#"
            UPDATE campaigns 
            SET name = ?1, description = ?2, data = ?3, cron = ?4, version = ?5, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?6
            "#,
            params![new_name, new_desc, new_data, new_cron, new_ver, id],
        )?;

        self.get_campaign(id)
    }

    pub fn upsert_campaign(
        &self,
        id: &str,
        name: Option<&str>,
        description: Option<&str>,
        data: Option<&str>,
        cron: Option<&str>,
        version: Option<&str>,
    ) -> Result<DbCampaign> {
        if let Some(current) = self.get_campaign(id)? {
            let new_name = name.unwrap_or(&current.name);
            let new_desc = description.or(current.description.as_deref());
            let new_data = data.unwrap_or(&current.data);
            let new_cron = cron.or(current.cron.as_deref());
            let new_ver = version.unwrap_or(&current.version);

            self.conn.execute(
                r#"
                UPDATE campaigns 
                SET name = ?1, description = ?2, data = ?3, cron = ?4, version = ?5, updated_at = CURRENT_TIMESTAMP
                WHERE id = ?6
                "#,
                params![new_name, new_desc, new_data, new_cron, new_ver, id],
            )?;
        } else {
            let initial_name = name.unwrap_or(id);
            let initial_data = data.unwrap_or("{}");
            let initial_ver = version.unwrap_or("1.0.0");

            self.conn.execute(
                r#"
                INSERT INTO campaigns (id, name, description, data, cron, version, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                "#,
                params![id, initial_name, description, initial_data, cron, initial_ver],
            )?;
        }

        self.get_campaign(id)?
            .ok_or_else(|| rusqlite::Error::QueryReturnedNoRows)
    }

    pub fn delete_campaign(&self, id: &str) -> Result<bool> {
        let affected = self.conn.execute("DELETE FROM campaigns WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }
}

impl<'a> crate::infrastructure::db::traits::CampaignRepository for SqliteCampaignRepository<'a> {
    fn create_campaign(
        &self,
        id: &str,
        name: &str,
        description: Option<&str>,
        data: &str,
        cron: Option<&str>,
        version: Option<&str>,
    ) -> Result<DbCampaign> {
        self.create_campaign(id, name, description, data, cron, version)
    }

    fn get_campaigns(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
        search: Option<&str>,
    ) -> Result<Vec<DbCampaign>> {
        self.get_campaigns(limit, offset, search)
    }

    fn get_campaign(&self, id: &str) -> Result<Option<DbCampaign>> {
        self.get_campaign(id)
    }

    fn update_campaign(
        &self,
        id: &str,
        name: Option<&str>,
        description: Option<&str>,
        data: Option<&str>,
        cron: Option<&str>,
        version: Option<&str>,
    ) -> Result<Option<DbCampaign>> {
        self.update_campaign(id, name, description, data, cron, version)
    }

    fn upsert_campaign(
        &self,
        id: &str,
        name: Option<&str>,
        description: Option<&str>,
        data: Option<&str>,
        cron: Option<&str>,
        version: Option<&str>,
    ) -> Result<DbCampaign> {
        self.upsert_campaign(id, name, description, data, cron, version)
    }

    fn delete_campaign(&self, id: &str) -> Result<bool> {
        self.delete_campaign(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::db::AutomaDb;

    #[test]
    fn test_sqlite_campaign_repo_crud() {
        let db = AutomaDb::new_in_memory().unwrap();
        let conn = db.raw_conn();
        let repo = SqliteCampaignRepository::new(conn);

        let initial_data = r#"{"members":[{"browserId":"b1","workflowId":"w1"}],"settings":{"concurrency_mode":"parallel"}}"#;
        let created = repo.create_campaign(
            "cp_daily_sync",
            "Daily Sync Campaign",
            Some("Syncs marketing profiles"),
            initial_data,
            Some("0 8 * * *"),
            Some("1.0.0"),
        ).unwrap();

        assert_eq!(created.id, "cp_daily_sync");
        assert_eq!(created.name, "Daily Sync Campaign");
        assert_eq!(created.cron.as_deref(), Some("0 8 * * *"));

        // Get by ID
        let fetched = repo.get_campaign("cp_daily_sync").unwrap().expect("Should exist");
        assert_eq!(fetched.data, initial_data);

        // List
        let list = repo.get_campaigns(None, None, None).unwrap();
        assert_eq!(list.len(), 1);

        // Update
        let updated = repo.update_campaign(
            "cp_daily_sync",
            Some("Daily Sync Campaign v2"),
            None,
            None,
            Some("0 9 * * *"),
            Some("1.1.0"),
        ).unwrap().expect("Should update");

        assert_eq!(updated.name, "Daily Sync Campaign v2");
        assert_eq!(updated.cron.as_deref(), Some("0 9 * * *"));
        assert_eq!(updated.version, "1.1.0");

        // Delete
        let deleted = repo.delete_campaign("cp_daily_sync").unwrap();
        assert!(deleted);
        assert!(repo.get_campaign("cp_daily_sync").unwrap().is_none());
    }
}
