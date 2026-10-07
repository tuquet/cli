use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct Browser {
    pub id: String,
    pub name: String,
    pub user_agent: Option<String>,
    pub timezone: Option<String>,
    pub proxy: Option<String>,
    pub extensions: Option<Vec<String>>,
    pub created_at: String,
    pub updated_at: String,
}

fn parse_extensions(raw: Option<String>) -> Vec<String> {
    match raw {
        Some(s) if !s.trim().is_empty() => {
            serde_json::from_str(&s).unwrap_or_else(|_| vec!["automa".to_string()])
        }
        _ => vec!["automa".to_string()],
    }
}

pub struct SqliteBrowserRepository<'a> {
    pub conn: &'a Connection,
}

impl<'a> SqliteBrowserRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn create_browser(
        &self,
        id: &str,
        name: &str,
        user_agent: Option<&str>,
        timezone: Option<&str>,
        proxy: Option<&str>,
        extensions: Option<&[String]>,
    ) -> Result<()> {
        let exts_json = extensions
            .map(|e| serde_json::to_string(e).unwrap_or_else(|_| "[\"automa\"]".to_string()))
            .unwrap_or_else(|| "[\"automa\"]".to_string());

        self.conn.execute(
            "INSERT INTO browsers (id, name, user_agent, timezone, proxy, extensions) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, name, user_agent, timezone, proxy, exts_json],
        )?;
        Ok(())
    }

    pub fn get_browsers(&self, limit: Option<usize>, offset: Option<usize>, search: Option<&str>) -> Result<Vec<Browser>> {
        let l = limit.map(|v| v as i64).unwrap_or(-1);
        let o = offset.unwrap_or(0) as i64;
        let mut browsers = Vec::new();

        if let Some(s) = search {
            let pattern = format!("%{}%", s);
            let mut stmt = self.conn.prepare(
                "SELECT id, name, user_agent, timezone, proxy, extensions, created_at, updated_at FROM browsers WHERE (name LIKE ?1 OR id LIKE ?1) ORDER BY name ASC LIMIT ?2 OFFSET ?3"
            )?;
            let browser_iter = stmt.query_map(params![pattern, l, o], |row| {
                let exts_raw: Option<String> = row.get(5)?;
                Ok(Browser {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    user_agent: row.get(2)?,
                    timezone: row.get(3)?,
                    proxy: row.get(4)?,
                    extensions: Some(parse_extensions(exts_raw)),
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            })?;
            for browser in browser_iter {
                browsers.push(browser?);
            }
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT id, name, user_agent, timezone, proxy, extensions, created_at, updated_at FROM browsers ORDER BY name ASC LIMIT ?1 OFFSET ?2"
            )?;
            let browser_iter = stmt.query_map(params![l, o], |row| {
                let exts_raw: Option<String> = row.get(5)?;
                Ok(Browser {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    user_agent: row.get(2)?,
                    timezone: row.get(3)?,
                    proxy: row.get(4)?,
                    extensions: Some(parse_extensions(exts_raw)),
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            })?;
            for browser in browser_iter {
                browsers.push(browser?);
            }
        }

        Ok(browsers)
    }

    pub fn get_browser(&self, id: &str) -> Result<Option<Browser>> {
        let mut stmt = self.conn.prepare("SELECT id, name, user_agent, timezone, proxy, extensions, created_at, updated_at FROM browsers WHERE id = ?1")?;
        let mut browser_iter = stmt.query_map(params![id], |row| {
            let exts_raw: Option<String> = row.get(5)?;
            Ok(Browser {
                id: row.get(0)?,
                name: row.get(1)?,
                user_agent: row.get(2)?,
                timezone: row.get(3)?,
                proxy: row.get(4)?,
                extensions: Some(parse_extensions(exts_raw)),
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;

        if let Some(browser) = browser_iter.next() {
            Ok(Some(browser?))
        } else {
            Ok(None)
        }
    }

    pub fn update_browser(
        &self,
        id: &str,
        name: &str,
        user_agent: Option<&str>,
        timezone: Option<&str>,
        proxy: Option<&str>,
        extensions: Option<&[String]>,
    ) -> Result<()> {
        if let Some(exts) = extensions {
            let exts_json = serde_json::to_string(exts).unwrap_or_else(|_| "[\"automa\"]".to_string());
            self.conn.execute(
                "UPDATE browsers SET name = ?1, user_agent = ?2, timezone = ?3, proxy = ?4, extensions = ?5, updated_at = CURRENT_TIMESTAMP WHERE id = ?6",
                params![name, user_agent, timezone, proxy, exts_json, id],
            )?;
        } else {
            self.conn.execute(
                "UPDATE browsers SET name = ?1, user_agent = ?2, timezone = ?3, proxy = ?4, updated_at = CURRENT_TIMESTAMP WHERE id = ?5",
                params![name, user_agent, timezone, proxy, id],
            )?;
        }
        Ok(())
    }

    pub fn delete_browser(&self, id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM browsers WHERE id = ?1", params![id])?;
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::db::AutomaDb;

    #[test]
    fn test_sqlite_browser_repo_crud() {
        let db = AutomaDb::new_in_memory().unwrap();
        let conn = db.raw_conn();
        let repo = SqliteBrowserRepository::new(conn);

        let initial_exts = vec!["automa".to_string(), "ublock".to_string()];
        repo.create_browser("b_test_1", "Profile 1", Some("UA/1.0"), Some("UTC"), Some("socks5://127.0.0.1:1080"), Some(&initial_exts)).unwrap();

        let b = repo.get_browser("b_test_1").unwrap().expect("Browser should exist");
        assert_eq!(b.name, "Profile 1");
        assert_eq!(b.user_agent.as_deref(), Some("UA/1.0"));
        assert_eq!(b.timezone.as_deref(), Some("UTC"));
        assert_eq!(b.proxy.as_deref(), Some("socks5://127.0.0.1:1080"));
        assert_eq!(b.extensions.as_ref().unwrap(), &initial_exts);

        let updated_exts = vec!["automa".to_string(), "ublock".to_string(), "cookie-injector".to_string()];
        repo.update_browser("b_test_1", "Profile 1 Renamed", Some("UA/2.0"), None, None, Some(&updated_exts)).unwrap();
        let updated = repo.get_browser("b_test_1").unwrap().unwrap();
        assert_eq!(updated.name, "Profile 1 Renamed");
        assert_eq!(updated.user_agent.as_deref(), Some("UA/2.0"));
        assert_eq!(updated.proxy, None);
        assert_eq!(updated.extensions.as_ref().unwrap(), &updated_exts);

        let list = repo.get_browsers(None, None, None).unwrap();
        assert_eq!(list.len(), 1);

        repo.delete_browser("b_test_1").unwrap();
        assert!(repo.get_browser("b_test_1").unwrap().is_none());
    }
}
