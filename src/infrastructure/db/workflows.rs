use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DbWorkflow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub data: String,
    pub version: String,
    pub icon: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct SqliteWorkflowRepository<'a> {
    pub conn: &'a Connection,
}

impl<'a> SqliteWorkflowRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn create_workflow(
        &self,
        id: &str,
        name: &str,
        description: Option<&str>,
        data: &str,
        version: Option<&str>,
        icon: Option<&str>,
    ) -> Result<DbWorkflow> {
        let ver = version.unwrap_or("1.0.0");
        self.conn.execute(
            r#"
            INSERT INTO workflows (id, name, description, data, version, icon, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                description = excluded.description,
                data = excluded.data,
                version = excluded.version,
                icon = excluded.icon,
                updated_at = CURRENT_TIMESTAMP
            "#,
            params![id, name, description, data, ver, icon],
        )?;

        self.get_workflow(id)?.ok_or_else(|| {
            rusqlite::Error::QueryReturnedNoRows
        })
    }

    pub fn get_workflows(&self, limit: Option<usize>, offset: Option<usize>, search: Option<&str>) -> Result<Vec<DbWorkflow>> {
        let l = limit.map(|v| v as i64).unwrap_or(-1);
        let o = offset.unwrap_or(0) as i64;
        let mut list = Vec::new();

        if let Some(s) = search {
            let pattern = format!("%{}%", s);
            let mut stmt = self.conn.prepare(
                "SELECT id, name, description, data, version, icon, created_at, updated_at FROM workflows WHERE (name LIKE ?1 OR id LIKE ?1 OR description LIKE ?1) ORDER BY updated_at DESC LIMIT ?2 OFFSET ?3"
            )?;
            let rows = stmt.query_map(params![pattern, l, o], |row| {
                Ok(DbWorkflow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    data: row.get(3)?,
                    version: row.get(4)?,
                    icon: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            })?;
            for r in rows {
                list.push(r?);
            }
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT id, name, description, data, version, icon, created_at, updated_at FROM workflows ORDER BY updated_at DESC LIMIT ?1 OFFSET ?2"
            )?;
            let rows = stmt.query_map(params![l, o], |row| {
                Ok(DbWorkflow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    data: row.get(3)?,
                    version: row.get(4)?,
                    icon: row.get(5)?,
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

    pub fn get_workflow(&self, id: &str) -> Result<Option<DbWorkflow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, data, version, icon, created_at, updated_at FROM workflows WHERE id = ?1"
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(DbWorkflow {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                data: row.get(3)?,
                version: row.get(4)?,
                icon: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;

        if let Some(wf) = rows.next() {
            Ok(Some(wf?))
        } else {
            Ok(None)
        }
    }

    pub fn update_workflow(
        &self,
        id: &str,
        name: Option<&str>,
        description: Option<&str>,
        data: Option<&str>,
        version: Option<&str>,
        icon: Option<&str>,
    ) -> Result<Option<DbWorkflow>> {
        let current = match self.get_workflow(id)? {
            Some(c) => c,
            None => return Ok(None),
        };

        let new_name = name.unwrap_or(&current.name);
        let new_desc = description.or(current.description.as_deref());
        let new_data = data.unwrap_or(&current.data);
        let new_ver = version.unwrap_or(&current.version);
        let new_icon = icon.or(current.icon.as_deref());

        self.conn.execute(
            r#"
            UPDATE workflows 
            SET name = ?1, description = ?2, data = ?3, version = ?4, icon = ?5, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?6
            "#,
            params![new_name, new_desc, new_data, new_ver, new_icon, id],
        )?;

        self.get_workflow(id)
    }

    pub fn upsert_workflow(
        &self,
        id: &str,
        name: Option<&str>,
        description: Option<&str>,
        data: Option<&str>,
        version: Option<&str>,
        icon: Option<&str>,
    ) -> Result<DbWorkflow> {
        if let Some(current) = self.get_workflow(id)? {
            let new_name = name.unwrap_or(&current.name);
            let new_desc = description.or(current.description.as_deref());
            let new_data = data.unwrap_or(&current.data);
            let new_ver = version.unwrap_or(&current.version);
            let new_icon = icon.or(current.icon.as_deref());

            self.conn.execute(
                r#"
                UPDATE workflows 
                SET name = ?1, description = ?2, data = ?3, version = ?4, icon = ?5, updated_at = CURRENT_TIMESTAMP
                WHERE id = ?6
                "#,
                params![new_name, new_desc, new_data, new_ver, new_icon, id],
            )?;
        } else {
            let initial_name = name.unwrap_or(id);
            let initial_data = data.unwrap_or("{}");
            let initial_ver = version.unwrap_or("1.0.0");

            self.conn.execute(
                r#"
                INSERT INTO workflows (id, name, description, data, version, icon, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                "#,
                params![id, initial_name, description, initial_data, initial_ver, icon],
            )?;
        }

        self.get_workflow(id)?
            .ok_or_else(|| rusqlite::Error::QueryReturnedNoRows)
    }

    pub fn delete_workflow(&self, id: &str) -> Result<bool> {
        let affected = self.conn.execute("DELETE FROM workflows WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }
}

impl<'a> crate::infrastructure::db::traits::WorkflowRepository for SqliteWorkflowRepository<'a> {
    fn create_workflow(
        &self,
        id: &str,
        name: &str,
        description: Option<&str>,
        data: &str,
        version: Option<&str>,
        icon: Option<&str>,
    ) -> Result<DbWorkflow> {
        self.create_workflow(id, name, description, data, version, icon)
    }

    fn get_workflows(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
        search: Option<&str>,
    ) -> Result<Vec<DbWorkflow>> {
        self.get_workflows(limit, offset, search)
    }

    fn get_workflow(&self, id: &str) -> Result<Option<DbWorkflow>> {
        self.get_workflow(id)
    }

    fn update_workflow(
        &self,
        id: &str,
        name: Option<&str>,
        description: Option<&str>,
        data: Option<&str>,
        version: Option<&str>,
        icon: Option<&str>,
    ) -> Result<Option<DbWorkflow>> {
        self.update_workflow(id, name, description, data, version, icon)
    }

    fn upsert_workflow(
        &self,
        id: &str,
        name: Option<&str>,
        description: Option<&str>,
        data: Option<&str>,
        version: Option<&str>,
        icon: Option<&str>,
    ) -> Result<DbWorkflow> {
        self.upsert_workflow(id, name, description, data, version, icon)
    }

    fn delete_workflow(&self, id: &str) -> Result<bool> {
        self.delete_workflow(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::db::AutomaDb;

    #[test]
    fn test_sqlite_workflow_repo_crud() {
        let db = AutomaDb::new_in_memory().unwrap();
        let conn = db.raw_conn();
        let repo = SqliteWorkflowRepository::new(conn);

        let initial_data = r#"{"nodes":[{"id":"node_1","type":"trigger"}],"edges":[]}"#;
        let created = repo.create_workflow(
            "wf_search",
            "Search Google",
            Some("Automated search"),
            initial_data,
            Some("1.0.0"),
            Some("search"),
        ).unwrap();

        assert_eq!(created.id, "wf_search");
        assert_eq!(created.name, "Search Google");
        assert_eq!(created.description.as_deref(), Some("Automated search"));

        // Get by ID
        let fetched = repo.get_workflow("wf_search").unwrap().expect("Should exist");
        assert_eq!(fetched.data, initial_data);

        // List
        let list = repo.get_workflows(None, None, None).unwrap();
        assert_eq!(list.len(), 1);

        // Update
        let updated_data = r#"{"nodes":[{"id":"node_1","type":"trigger"},{"id":"node_2","type":"click-element"}],"edges":[]}"#;
        let updated = repo.update_workflow(
            "wf_search",
            Some("Search Google & Bing"),
            None,
            Some(updated_data),
            Some("1.1.0"),
            None,
        ).unwrap().expect("Should update");

        assert_eq!(updated.name, "Search Google & Bing");
        assert_eq!(updated.version, "1.1.0");
        assert_eq!(updated.data, updated_data);

        // Delete
        let deleted = repo.delete_workflow("wf_search").unwrap();
        assert!(deleted);
        assert!(repo.get_workflow("wf_search").unwrap().is_none());
    }
}
