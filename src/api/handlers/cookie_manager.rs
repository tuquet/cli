use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Cookie {
    pub domain: String,
    pub expiration_date: Option<f64>,
    pub host_only: Option<bool>,
    pub http_only: Option<bool>,
    pub name: String,
    pub path: String,
    pub same_site: Option<String>,
    pub secure: Option<bool>,
    pub session: Option<bool>,
    pub store_id: Option<String>,
    pub value: String,
    pub id: Option<i64>,
}

pub fn export_cookies(db_path: &std::path::Path) -> Result<Vec<Cookie>> {
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare("SELECT creation_utc, host_key, top_frame_site_key, name, value, encrypted_value, path, expires_utc, is_secure, is_httponly, samesite FROM cookies")?;
    
    let cookie_iter = stmt.query_map([], |row| {
        let host_key: String = row.get(1)?;
        let name: String = row.get(3)?;
        let value: String = row.get(4)?;
        let path: String = row.get(6)?;
        let expires_utc: i64 = row.get(7)?;
        let is_secure: i32 = row.get(8)?;
        let is_httponly: i32 = row.get(9)?;
        let samesite: i32 = row.get(10)?;

        let expiration_date = if expires_utc > 0 {
            Some((expires_utc as f64 / 1_000_000.0) - 11_644_473_600.0)
        } else {
            None
        };

        let same_site_str = match samesite {
            0 => "unspecified",
            1 => "no_restriction",
            2 => "lax",
            3 => "strict",
            _ => "unspecified",
        };

        Ok(Cookie {
            domain: host_key,
            expiration_date,
            host_only: Some(false),
            http_only: Some(is_httponly != 0),
            name,
            path,
            same_site: Some(same_site_str.to_string()),
            secure: Some(is_secure != 0),
            session: Some(expires_utc == 0),
            store_id: Some("0".to_string()),
            value,
            id: None,
        })
    })?;

    let mut cookies = Vec::new();
    for cookie in cookie_iter {
        cookies.push(cookie?);
    }

    Ok(cookies)
}

pub fn import_cookies(db_path: &std::path::Path, cookies: Vec<Cookie>) -> Result<()> {
    let conn = Connection::open(db_path)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS cookies (
            creation_utc INTEGER NOT NULL,
            host_key TEXT NOT NULL,
            top_frame_site_key TEXT NOT NULL DEFAULT '',
            name TEXT NOT NULL,
            value TEXT NOT NULL,
            encrypted_value BLOB NOT NULL DEFAULT '',
            path TEXT NOT NULL,
            expires_utc INTEGER NOT NULL,
            is_secure INTEGER NOT NULL,
            is_httponly INTEGER NOT NULL,
            last_access_utc INTEGER NOT NULL,
            has_expires INTEGER NOT NULL,
            is_persistent INTEGER NOT NULL,
            priority INTEGER NOT NULL DEFAULT 1,
            samesite INTEGER NOT NULL DEFAULT 0,
            source_scheme INTEGER NOT NULL DEFAULT 0,
            source_port INTEGER NOT NULL DEFAULT -1,
            is_same_party INTEGER NOT NULL DEFAULT 0,
            last_update_utc INTEGER NOT NULL,
            PRIMARY KEY (host_key, top_frame_site_key, name, path, source_scheme, source_port)
        )",
        [],
    )?;
    
    let mut stmt = conn.prepare(
        "INSERT OR REPLACE INTO cookies 
        (creation_utc, host_key, top_frame_site_key, name, value, encrypted_value, path, expires_utc, 
         is_secure, is_httponly, last_access_utc, has_expires, is_persistent, priority, samesite, 
         source_scheme, source_port, is_same_party, last_update_utc) 
        VALUES 
        (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)"
    )?;

    for cookie in cookies {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
        let creation_utc = (now + 11_644_473_600) * 1_000_000;
        
        let expires_utc = if let Some(exp) = cookie.expiration_date {
            ((exp + 11_644_473_600.0) * 1_000_000.0) as i64
        } else {
            0
        };

        let samesite = match cookie.same_site.as_deref() {
            Some("no_restriction") => 1,
            Some("lax") => 2,
            Some("strict") => 3,
            _ => 0,
        };

        let has_expires = if cookie.expiration_date.is_some() { 1 } else { 0 };
        let is_persistent = has_expires;

        stmt.execute(rusqlite::params![
            creation_utc as i64,
            cookie.domain,
            "",
            cookie.name,
            cookie.value,
            b"", 
            cookie.path,
            expires_utc,
            cookie.secure.unwrap_or(false) as i32,
            cookie.http_only.unwrap_or(false) as i32,
            creation_utc as i64, 
            has_expires,
            is_persistent,
            1, 
            samesite,
            0, 
            -1, 
            0, 
            creation_utc as i64 
        ])?;
    }

    Ok(())
}
