use rusqlite::{params, Connection, Result};
use crate::core::models::storage::{StorageTable, TableRow};


pub struct SqliteTableRepository<'a> {
    pub conn: &'a Connection,
}

impl<'a> SqliteTableRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn get_tables(&self, limit: Option<usize>, offset: Option<usize>, search: Option<&str>) -> Result<Vec<StorageTable>> {
        let l = limit.map(|v| v as i64).unwrap_or(-1);
        let o = offset.unwrap_or(0) as i64;
        let mut tables = Vec::new();

        if let Some(s) = search {
            let pattern = format!("%{}%", s);
            let mut stmt = self.conn.prepare(
                "SELECT id, name, columns, columns_index, created_at, modified_at FROM storage_tables WHERE (name LIKE ?1 OR id LIKE ?1) ORDER BY created_at DESC LIMIT ?2 OFFSET ?3"
            )?;
            let table_iter = stmt.query_map(params![pattern, l, o], |row| {
                Ok(StorageTable {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    columns: row.get::<_, Option<String>>(2)?.and_then(|s| serde_json::from_str(&s).ok()),
                    items: None,
                    columns_index: row.get::<_, Option<String>>(3)?.and_then(|s| serde_json::from_str(&s).ok()),
                    created_at: row.get(4)?,
                    modified_at: row.get(5)?,
                })
            })?;
            for t in table_iter {
                tables.push(t?);
            }
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT id, name, columns, columns_index, created_at, modified_at FROM storage_tables ORDER BY created_at DESC LIMIT ?1 OFFSET ?2"
            )?;
            let table_iter = stmt.query_map(params![l, o], |row| {
                Ok(StorageTable {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    columns: row.get::<_, Option<String>>(2)?.and_then(|s| serde_json::from_str(&s).ok()),
                    items: None,
                    columns_index: row.get::<_, Option<String>>(3)?.and_then(|s| serde_json::from_str(&s).ok()),
                    created_at: row.get(4)?,
                    modified_at: row.get(5)?,
                })
            })?;
            for t in table_iter {
                tables.push(t?);
            }
        }

        Ok(tables)
    }

    pub fn save_table(&self, table: &StorageTable) -> Result<()> {
        let cols = table.columns.as_ref().map(|v| v.to_string());
        let cols_idx = table.columns_index.as_ref().map(|v| v.to_string());
        let id = table.id.as_deref().unwrap_or("");
        let name = table.name.as_deref().unwrap_or("");

        self.conn.execute(
            "INSERT INTO storage_tables (id, name, columns, columns_index, created_at, modified_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                columns = excluded.columns,
                columns_index = excluded.columns_index,
                modified_at = excluded.modified_at",
            params![id, name, cols, cols_idx, table.created_at, table.modified_at],
        )?;
        Ok(())
    }

    pub fn delete_table(&self, id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM storage_tables WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_table_rows(&self, table_id: &str, limit: Option<usize>, offset: Option<usize>, search: Option<&str>) -> Result<Vec<TableRow>> {
        let l = limit.map(|v| v as i64).unwrap_or(50);
        let o = offset.unwrap_or(0) as i64;
        let mut rows = Vec::new();

        if let Some(s) = search {
            let pattern = format!("%{}%", s);
            let mut stmt = self.conn.prepare(
                "SELECT id, data FROM storage_table_rows WHERE table_id = ?1 AND data LIKE ?2 LIMIT ?3 OFFSET ?4"
            )?;
            let row_iter = stmt.query_map(params![table_id, pattern, l, o], |row| {
                let id: String = row.get(0)?;
                let raw_data: String = row.get(1)?;
                let data: serde_json::Value = serde_json::from_str(&raw_data).unwrap_or_default();
                Ok(TableRow {
                    id,
                    table_id: table_id.to_string(),
                    data,
                })
            })?;
            for r in row_iter {
                rows.push(r?);
            }
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT id, data FROM storage_table_rows WHERE table_id = ?1 LIMIT ?2 OFFSET ?3"
            )?;
            let row_iter = stmt.query_map(params![table_id, l, o], |row| {
                let id: String = row.get(0)?;
                let raw_data: String = row.get(1)?;
                let data: serde_json::Value = serde_json::from_str(&raw_data).unwrap_or_default();
                Ok(TableRow {
                    id,
                    table_id: table_id.to_string(),
                    data,
                })
            })?;
            for r in row_iter {
                rows.push(r?);
            }
        }

        Ok(rows)
    }

    pub fn add_table_row(&self, id: &str, table_id: &str, data: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO storage_table_rows (id, table_id, data) VALUES (?1, ?2, ?3)",
            params![id, table_id, data],
        )?;
        Ok(())
    }
}

impl<'a> crate::infrastructure::db::traits::TableRepository for SqliteTableRepository<'a> {
    fn get_tables(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
        search: Option<&str>,
    ) -> Result<Vec<StorageTable>> {
        self.get_tables(limit, offset, search)
    }

    fn save_table(&self, table: &StorageTable) -> Result<()> {
        self.save_table(table)
    }

    fn delete_table(&self, id: &str) -> Result<()> {
        self.delete_table(id)
    }

    fn get_table_rows(
        &self,
        table_id: &str,
        limit: Option<usize>,
        offset: Option<usize>,
        search: Option<&str>,
    ) -> Result<Vec<TableRow>> {
        self.get_table_rows(table_id, limit, offset, search)
    }

    fn add_table_row(&self, id: &str, table_id: &str, data: &str) -> Result<()> {
        self.add_table_row(id, table_id, data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::db::AutomaDb;

    #[test]
    fn test_sqlite_tables_crud_and_rows() {
        let db = AutomaDb::new_in_memory().unwrap();
        let conn = db.raw_conn();
        let repo = SqliteTableRepository::new(conn);

        let table = StorageTable {
            id: Some("tbl_users".to_string()),
            name: Some("Users Table".to_string()),
            columns: Some(serde_json::json!([{"name": "username", "type": "string"}])),
            items: None,
            columns_index: None,
            created_at: Some(1700000000),
            modified_at: Some(1700000000),
        };

        repo.save_table(&table).unwrap();
        let tables = repo.get_tables(None, None, None).unwrap();
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].name.as_deref().unwrap(), "Users Table");

        repo.add_table_row("row_1", "tbl_users", r#"{"username":"alice"}"#).unwrap();
        repo.add_table_row("row_2", "tbl_users", r#"{"username":"bob"}"#).unwrap();

        let rows = repo.get_table_rows("tbl_users", Some(10), Some(0), None).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].data["username"], "alice");

        // Test pagination limit
        let paginated = repo.get_table_rows("tbl_users", Some(1), Some(1), None).unwrap();
        assert_eq!(paginated.len(), 1);
        assert_eq!(paginated[0].data["username"], "bob");

        // Delete table
        repo.delete_table("tbl_users").unwrap();
        assert!(repo.get_tables(None, None, None).unwrap().is_empty());
    }
}
