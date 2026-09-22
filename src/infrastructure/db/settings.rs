use rusqlite::{params, Connection, Result};
use crate::core::models::settings::AppSettings;


pub struct SqliteSettingsRepository<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteSettingsRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
}

impl<'a> SqliteSettingsRepository<'a> {
    pub fn get_settings(&self) -> Result<AppSettings> {
        let mut stmt = self.conn.prepare("SELECT value FROM system_settings WHERE key = 'app_settings'")?;
        let mut rows = stmt.query([])?;

        if let Some(row) = rows.next()? {
            let json_str: String = row.get(0)?;
            if let Ok(settings) = serde_json::from_str::<AppSettings>(&json_str) {
                return Ok(settings);
            }
        }

        // Return default if not yet stored
        let default_settings = AppSettings::default();
        let _ = self.save_settings(&default_settings);
        Ok(default_settings)
    }
    pub fn save_settings(&self, settings: &AppSettings) -> Result<()> {
        let json_str = serde_json::to_string_pretty(settings)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

        self.conn.execute(
            r#"
            INSERT INTO system_settings (key, value, updated_at) 
            VALUES ('app_settings', ?1, CURRENT_TIMESTAMP)
            ON CONFLICT(key) DO UPDATE SET 
                value = excluded.value, 
                updated_at = CURRENT_TIMESTAMP
            "#,
            params![json_str],
        )?;

        Ok(())
    }
}

impl<'a> crate::infrastructure::db::traits::SettingsRepository for SqliteSettingsRepository<'a> {
    fn get_settings(&self) -> Result<AppSettings> {
        self.get_settings()
    }

    fn save_settings(&self, settings: &AppSettings) -> Result<()> {
        self.save_settings(settings)
    }
}
