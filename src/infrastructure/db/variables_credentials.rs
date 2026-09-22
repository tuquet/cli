use rusqlite::{params, Connection, Result};
use crate::core::models::storage::{StorageVariable, StorageCredential};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct SqliteStorageRepository<'a> {
    pub conn: &'a Connection,
}

impl<'a> SqliteStorageRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    fn current_time_millis() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }

    // --- Variables ---

    pub fn get_variables(&self, limit: Option<usize>, offset: Option<usize>, search: Option<&str>) -> Result<Vec<StorageVariable>> {
        let l = limit.map(|v| v as i64).unwrap_or(-1);
        let o = offset.unwrap_or(0) as i64;
        let mut vars = Vec::new();

        if let Some(s) = search {
            let pattern = format!("%{}%", s);
            let mut stmt = self.conn.prepare(
                "SELECT id, name, value FROM storage_variables WHERE (name LIKE ?1 OR id LIKE ?1) ORDER BY created_at ASC LIMIT ?2 OFFSET ?3"
            )?;
            let var_iter = stmt.query_map(params![pattern, l, o], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let raw_value: String = row.get(2)?;
                let value = serde_json::from_str(&raw_value)
                    .unwrap_or(serde_json::Value::String(raw_value));
                Ok(StorageVariable {
                    id: Some(id),
                    name: Some(name.clone()),
                    key: Some(name),
                    value: Some(value),
                })
            })?;
            for v in var_iter {
                vars.push(v?);
            }
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT id, name, value FROM storage_variables ORDER BY created_at ASC LIMIT ?1 OFFSET ?2"
            )?;
            let var_iter = stmt.query_map(params![l, o], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let raw_value: String = row.get(2)?;
                let value = serde_json::from_str(&raw_value)
                    .unwrap_or(serde_json::Value::String(raw_value));
                Ok(StorageVariable {
                    id: Some(id),
                    name: Some(name.clone()),
                    key: Some(name),
                    value: Some(value),
                })
            })?;
            for v in var_iter {
                vars.push(v?);
            }
        }

        Ok(vars)
    }

    pub fn save_variable(&self, var: &StorageVariable) -> Result<StorageVariable> {
        let id = var.id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let name = var.name.clone().or_else(|| var.key.clone()).unwrap_or_default();
        let val_str = var.value.as_ref().map(|v| match v {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        }).unwrap_or_default();
        let now = Self::current_time_millis();

        self.conn.execute(
            "INSERT INTO storage_variables (id, name, value, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(name) DO UPDATE SET
                value = excluded.value,
                updated_at = excluded.updated_at",
            params![id, name, val_str, now, now],
        )?;

        Ok(StorageVariable {
            id: Some(id),
            name: Some(name.clone()),
            key: Some(name),
            value: var.value.clone(),
        })
    }

    pub fn delete_variable(&self, id_or_name: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM storage_variables WHERE id = ?1 OR name = ?1",
            params![id_or_name],
        )?;
        Ok(())
    }

    // --- Credentials ---

    pub fn get_credentials(&self, limit: Option<usize>, offset: Option<usize>, search: Option<&str>) -> Result<Vec<StorageCredential>> {
        let l = limit.map(|v| v as i64).unwrap_or(-1);
        let o = offset.unwrap_or(0) as i64;
        let mut creds = Vec::new();

        if let Some(s) = search {
            let pattern = format!("%{}%", s);
            let mut stmt = self.conn.prepare(
                "SELECT id, name, value FROM storage_credentials WHERE (name LIKE ?1 OR id LIKE ?1) ORDER BY created_at ASC LIMIT ?2 OFFSET ?3"
            )?;
            let cred_iter = stmt.query_map(params![pattern, l, o], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let value: String = row.get(2)?;
                Ok(StorageCredential {
                    id: Some(id),
                    name: Some(name.clone()),
                    key: Some(name),
                    value: Some(value),
                })
            })?;
            for c in cred_iter {
                creds.push(c?);
            }
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT id, name, value FROM storage_credentials ORDER BY created_at ASC LIMIT ?1 OFFSET ?2"
            )?;
            let cred_iter = stmt.query_map(params![l, o], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let value: String = row.get(2)?;
                Ok(StorageCredential {
                    id: Some(id),
                    name: Some(name.clone()),
                    key: Some(name),
                    value: Some(value),
                })
            })?;
            for c in cred_iter {
                creds.push(c?);
            }
        }

        Ok(creds)
    }

    pub fn save_credential(&self, cred: &StorageCredential) -> Result<StorageCredential> {
        let id = cred.id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let name = cred.name.clone().or_else(|| cred.key.clone()).unwrap_or_default();
        let raw_val = cred.value.as_deref().unwrap_or_default();

        // Auto-encrypt with AES if not already encrypted
        let passphrase = std::env::var("AUTOMA_SECRET_KEY")
            .or_else(|_| std::env::var("AUTOMA_PASSPHRASE"))
            .unwrap_or_else(|_| "automa-storage-secret".to_string());

        let encrypted_val = if raw_val.starts_with("Salted__")
            || (raw_val.len() > 64 && raw_val[64..].starts_with("Salted__"))
        {
            raw_val.to_string()
        } else {
            crate::core::crypto::encrypt_secret(raw_val, &passphrase)
                .unwrap_or_else(|_| raw_val.to_string())
        };

        let now = Self::current_time_millis();

        self.conn.execute(
            "INSERT INTO storage_credentials (id, name, value, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(name) DO UPDATE SET
                value = excluded.value,
                updated_at = excluded.updated_at",
            params![id, name, encrypted_val, now, now],
        )?;

        Ok(StorageCredential {
            id: Some(id),
            name: Some(name.clone()),
            key: Some(name),
            value: Some(encrypted_val),
        })
    }

    pub fn delete_credential(&self, id_or_name: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM storage_credentials WHERE id = ?1 OR name = ?1",
            params![id_or_name],
        )?;
        Ok(())
    }
}

impl<'a> crate::infrastructure::db::traits::StorageRepository for SqliteStorageRepository<'a> {
    fn get_variables(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
        search: Option<&str>,
    ) -> Result<Vec<StorageVariable>> {
        self.get_variables(limit, offset, search)
    }

    fn save_variable(&self, var: &StorageVariable) -> Result<StorageVariable> {
        self.save_variable(var)
    }

    fn delete_variable(&self, id_or_name: &str) -> Result<()> {
        self.delete_variable(id_or_name)
    }

    fn get_credentials(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
        search: Option<&str>,
    ) -> Result<Vec<StorageCredential>> {
        self.get_credentials(limit, offset, search)
    }

    fn save_credential(&self, cred: &StorageCredential) -> Result<StorageCredential> {
        self.save_credential(cred)
    }

    fn delete_credential(&self, id_or_name: &str) -> Result<()> {
        self.delete_credential(id_or_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::db::AutomaDb;

    #[test]
    fn test_variables_crud() {
        let db = AutomaDb::new_in_memory().unwrap();
        let storage = db.storage();

        // 1. Add variable
        let var = StorageVariable {
            id: None,
            name: Some("API_ENDPOINT".to_string()),
            key: None,
            value: Some(serde_json::json!("https://api.example.com")),
        };
        let saved = storage.save_variable(&var).unwrap();
        assert!(saved.id.is_some());

        // 2. List variables
        let vars = storage.get_variables(None, None, None).unwrap();
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].name.as_deref(), Some("API_ENDPOINT"));
        assert_eq!(vars[0].value, Some(serde_json::json!("https://api.example.com")));

        // 3. Update variable
        let var_updated = StorageVariable {
            id: None,
            name: Some("API_ENDPOINT".to_string()),
            key: None,
            value: Some(serde_json::json!("https://api.v2.example.com")),
        };
        storage.save_variable(&var_updated).unwrap();
        let vars_after = storage.get_variables(None, None, None).unwrap();
        assert_eq!(vars_after.len(), 1);
        assert_eq!(vars_after[0].value, Some(serde_json::json!("https://api.v2.example.com")));

        // 4. Delete variable
        storage.delete_variable("API_ENDPOINT").unwrap();
        let vars_deleted = storage.get_variables(None, None, None).unwrap();
        assert_eq!(vars_deleted.len(), 0);
    }

    #[test]
    fn test_credentials_auto_aes_encryption() {
        let db = AutomaDb::new_in_memory().unwrap();
        let storage = db.storage();

        // 1. Add plaintext credential
        let cred = StorageCredential {
            id: None,
            name: Some("GITHUB_TOKEN".to_string()),
            key: None,
            value: Some("ghp_secret_token_123456".to_string()),
        };
        let saved = storage.save_credential(&cred).unwrap();
        let encrypted_val = saved.value.unwrap();
        
        // Assert it is encrypted (not plaintext)
        assert_ne!(encrypted_val, "ghp_secret_token_123456");
        assert!(encrypted_val.len() > 64);

        // 2. Query credential
        let creds = storage.get_credentials(None, None, None).unwrap();
        assert_eq!(creds.len(), 1);
        assert_eq!(creds[0].name.as_deref(), Some("GITHUB_TOKEN"));
        assert_eq!(creds[0].value.as_deref(), Some(encrypted_val.as_str()));

        // 3. Delete credential
        storage.delete_credential("GITHUB_TOKEN").unwrap();
        let creds_deleted = storage.get_credentials(None, None, None).unwrap();
        assert_eq!(creds_deleted.len(), 0);
    }
}
