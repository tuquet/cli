use std::path::{Path, PathBuf};
use std::sync::Mutex;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::config::inbox::InboxConfig;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Storage mutex poisoned")]
    Poisoned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailRecord {
    pub id: String,
    pub message_id: Option<String>,
    pub sender: String,
    pub recipient: String,
    pub domain: String,
    pub subject: Option<String>,
    pub body_text: Option<String>,
    pub body_html: Option<String>,
    pub received_at: String,
    pub raw_headers: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtpRecord {
    pub id: String,
    pub email_id: String,
    pub recipient: String,
    pub otp_code: String,
    pub service_name: Option<String>,
    pub expires_at: Option<String>,
    pub consumed_at: Option<String>,
    pub consumed_by: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkRecord {
    pub id: String,
    pub email_id: String,
    pub recipient: String,
    pub url: String,
    pub link_type: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailSummary {
    pub id: String,
    pub sender: String,
    pub recipient: String,
    pub domain: String,
    pub subject: Option<String>,
    pub received_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InboxStats {
    pub total_emails: usize,
    pub total_otps: usize,
    pub unconsumed_otps: usize,
    pub total_links: usize,
}

pub struct InboxStorage {
    conn: Mutex<Connection>,
    db_path: Option<PathBuf>,
}

impl InboxStorage {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let path_ref = path.as_ref();
        if let Some(parent) = path_ref.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path_ref)?;
        let storage = Self {
            conn: Mutex::new(conn),
            db_path: Some(path_ref.to_path_buf()),
        };
        storage.init_db()?;
        Ok(storage)
    }

    pub fn open_in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
        let storage = Self {
            conn: Mutex::new(conn),
            db_path: None,
        };
        storage.init_db()?;
        Ok(storage)
    }

    pub fn open_default() -> Result<Self, StorageError> {
        let path = InboxConfig::sqlite_path();
        Self::open(path)
    }

    pub fn db_path(&self) -> Option<&Path> {
        self.db_path.as_deref()
    }

    fn init_db(&self) -> Result<(), StorageError> {
        let conn = self.conn.lock().map_err(|_| StorageError::Poisoned)?;
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;
            PRAGMA synchronous = NORMAL;

            CREATE TABLE IF NOT EXISTS emails (
                id TEXT PRIMARY KEY,
                message_id TEXT,
                sender TEXT NOT NULL,
                recipient TEXT NOT NULL,
                domain TEXT NOT NULL,
                subject TEXT,
                body_text TEXT,
                body_html TEXT,
                received_at TEXT NOT NULL,
                raw_headers TEXT,
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_emails_recipient ON emails(recipient);
            CREATE INDEX IF NOT EXISTS idx_emails_domain ON emails(domain);
            CREATE INDEX IF NOT EXISTS idx_emails_received ON emails(received_at DESC);

            CREATE TABLE IF NOT EXISTS otps (
                id TEXT PRIMARY KEY,
                email_id TEXT NOT NULL REFERENCES emails(id) ON DELETE CASCADE,
                recipient TEXT NOT NULL,
                otp_code TEXT NOT NULL,
                service_name TEXT,
                expires_at TEXT,
                consumed_at TEXT,
                consumed_by TEXT,
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_otps_recipient ON otps(recipient);
            CREATE INDEX IF NOT EXISTS idx_otps_created ON otps(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_otps_consumed ON otps(consumed_at);

            CREATE TABLE IF NOT EXISTS links (
                id TEXT PRIMARY KEY,
                email_id TEXT NOT NULL REFERENCES emails(id) ON DELETE CASCADE,
                recipient TEXT NOT NULL,
                url TEXT NOT NULL,
                link_type TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_links_recipient ON links(recipient);
            CREATE INDEX IF NOT EXISTS idx_links_created ON links(created_at DESC);
            ",
        )?;
        Ok(())
    }

    pub fn save_email(&self, email: &EmailRecord) -> Result<(), StorageError> {
        let conn = self.conn.lock().map_err(|_| StorageError::Poisoned)?;
        conn.execute(
            "INSERT OR REPLACE INTO emails (
                id, message_id, sender, recipient, domain, subject, body_text, body_html, received_at, raw_headers, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                email.id,
                email.message_id,
                email.sender,
                email.recipient,
                email.domain,
                email.subject,
                email.body_text,
                email.body_html,
                email.received_at,
                email.raw_headers,
                email.created_at,
            ],
        )?;
        Ok(())
    }

    pub fn get_email(&self, id: &str) -> Result<Option<EmailRecord>, StorageError> {
        let conn = self.conn.lock().map_err(|_| StorageError::Poisoned)?;
        let mut stmt = conn.prepare(
            "SELECT id, message_id, sender, recipient, domain, subject, body_text, body_html, received_at, raw_headers, created_at
             FROM emails WHERE id = ?1",
        )?;

        let email = stmt
            .query_row(params![id], |row| {
                Ok(EmailRecord {
                    id: row.get(0)?,
                    message_id: row.get(1)?,
                    sender: row.get(2)?,
                    recipient: row.get(3)?,
                    domain: row.get(4)?,
                    subject: row.get(5)?,
                    body_text: row.get(6)?,
                    body_html: row.get(7)?,
                    received_at: row.get(8)?,
                    raw_headers: row.get(9)?,
                    created_at: row.get(10)?,
                })
            })
            .optional()?;

        Ok(email)
    }

    pub fn list_emails(&self, limit: usize, offset: usize, recipient: Option<&str>) -> Result<Vec<EmailSummary>, StorageError> {
        let conn = self.conn.lock().map_err(|_| StorageError::Poisoned)?;
        let mut summaries = Vec::new();

        if let Some(r) = recipient {
            let mut stmt = conn.prepare(
                "SELECT id, sender, recipient, domain, subject, received_at
                 FROM emails WHERE recipient = ?1 ORDER BY received_at DESC LIMIT ?2 OFFSET ?3",
            )?;
            let rows = stmt.query_map(params![r, limit as i64, offset as i64], |row| {
                Ok(EmailSummary {
                    id: row.get(0)?,
                    sender: row.get(1)?,
                    recipient: row.get(2)?,
                    domain: row.get(3)?,
                    subject: row.get(4)?,
                    received_at: row.get(5)?,
                })
            })?;
            for row in rows {
                summaries.push(row?);
            }
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, sender, recipient, domain, subject, received_at
                 FROM emails ORDER BY received_at DESC LIMIT ?1 OFFSET ?2",
            )?;
            let rows = stmt.query_map(params![limit as i64, offset as i64], |row| {
                Ok(EmailSummary {
                    id: row.get(0)?,
                    sender: row.get(1)?,
                    recipient: row.get(2)?,
                    domain: row.get(3)?,
                    subject: row.get(4)?,
                    received_at: row.get(5)?,
                })
            })?;
            for row in rows {
                summaries.push(row?);
            }
        }

        Ok(summaries)
    }

    pub fn delete_email(&self, id: &str) -> Result<bool, StorageError> {
        let conn = self.conn.lock().map_err(|_| StorageError::Poisoned)?;
        let rows = conn.execute("DELETE FROM emails WHERE id = ?1", params![id])?;
        Ok(rows > 0)
    }

    pub fn save_otp(&self, otp: &OtpRecord) -> Result<(), StorageError> {
        let conn = self.conn.lock().map_err(|_| StorageError::Poisoned)?;
        conn.execute(
            "INSERT OR REPLACE INTO otps (
                id, email_id, recipient, otp_code, service_name, expires_at, consumed_at, consumed_by, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                otp.id,
                otp.email_id,
                otp.recipient,
                otp.otp_code,
                otp.service_name,
                otp.expires_at,
                otp.consumed_at,
                otp.consumed_by,
                otp.created_at,
            ],
        )?;
        Ok(())
    }

    pub fn get_latest_otp(&self, recipient: &str, unconsumed_only: bool) -> Result<Option<OtpRecord>, StorageError> {
        let conn = self.conn.lock().map_err(|_| StorageError::Poisoned)?;
        let query = if unconsumed_only {
            "SELECT id, email_id, recipient, otp_code, service_name, expires_at, consumed_at, consumed_by, created_at
             FROM otps WHERE recipient = ?1 AND consumed_at IS NULL ORDER BY created_at DESC LIMIT 1"
        } else {
            "SELECT id, email_id, recipient, otp_code, service_name, expires_at, consumed_at, consumed_by, created_at
             FROM otps WHERE recipient = ?1 ORDER BY created_at DESC LIMIT 1"
        };

        let mut stmt = conn.prepare(query)?;
        let otp = stmt
            .query_row(params![recipient], |row| {
                Ok(OtpRecord {
                    id: row.get(0)?,
                    email_id: row.get(1)?,
                    recipient: row.get(2)?,
                    otp_code: row.get(3)?,
                    service_name: row.get(4)?,
                    expires_at: row.get(5)?,
                    consumed_at: row.get(6)?,
                    consumed_by: row.get(7)?,
                    created_at: row.get(8)?,
                })
            })
            .optional()?;

        Ok(otp)
    }

    pub fn consume_otp(&self, otp_id: &str, device_id: &str) -> Result<bool, StorageError> {
        let conn = self.conn.lock().map_err(|_| StorageError::Poisoned)?;
        let now = Utc::now().to_rfc3339();
        let rows = conn.execute(
            "UPDATE otps SET consumed_at = ?1, consumed_by = ?2 WHERE id = ?3 AND consumed_at IS NULL",
            params![now, device_id, otp_id],
        )?;
        Ok(rows > 0)
    }

    pub fn list_otps(&self, recipient: Option<&str>, limit: usize) -> Result<Vec<OtpRecord>, StorageError> {
        let conn = self.conn.lock().map_err(|_| StorageError::Poisoned)?;
        let mut otps = Vec::new();

        if let Some(r) = recipient {
            let mut stmt = conn.prepare(
                "SELECT id, email_id, recipient, otp_code, service_name, expires_at, consumed_at, consumed_by, created_at
                 FROM otps WHERE recipient = ?1 ORDER BY created_at DESC LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![r, limit as i64], |row| {
                Ok(OtpRecord {
                    id: row.get(0)?,
                    email_id: row.get(1)?,
                    recipient: row.get(2)?,
                    otp_code: row.get(3)?,
                    service_name: row.get(4)?,
                    expires_at: row.get(5)?,
                    consumed_at: row.get(6)?,
                    consumed_by: row.get(7)?,
                    created_at: row.get(8)?,
                })
            })?;
            for row in rows {
                otps.push(row?);
            }
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, email_id, recipient, otp_code, service_name, expires_at, consumed_at, consumed_by, created_at
                 FROM otps ORDER BY created_at DESC LIMIT ?1",
            )?;
            let rows = stmt.query_map(params![limit as i64], |row| {
                Ok(OtpRecord {
                    id: row.get(0)?,
                    email_id: row.get(1)?,
                    recipient: row.get(2)?,
                    otp_code: row.get(3)?,
                    service_name: row.get(4)?,
                    expires_at: row.get(5)?,
                    consumed_at: row.get(6)?,
                    consumed_by: row.get(7)?,
                    created_at: row.get(8)?,
                })
            })?;
            for row in rows {
                otps.push(row?);
            }
        }

        Ok(otps)
    }

    pub fn save_link(&self, link: &LinkRecord) -> Result<(), StorageError> {
        let conn = self.conn.lock().map_err(|_| StorageError::Poisoned)?;
        conn.execute(
            "INSERT OR REPLACE INTO links (
                id, email_id, recipient, url, link_type, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                link.id,
                link.email_id,
                link.recipient,
                link.url,
                link.link_type,
                link.created_at,
            ],
        )?;
        Ok(())
    }

    pub fn get_latest_link(&self, recipient: &str, link_type: Option<&str>) -> Result<Option<LinkRecord>, StorageError> {
        let conn = self.conn.lock().map_err(|_| StorageError::Poisoned)?;
        let link = if let Some(lt) = link_type {
            let mut stmt = conn.prepare(
                "SELECT id, email_id, recipient, url, link_type, created_at
                 FROM links WHERE recipient = ?1 AND link_type = ?2 ORDER BY created_at DESC LIMIT 1",
            )?;
            stmt.query_row(params![recipient, lt], |row| {
                Ok(LinkRecord {
                    id: row.get(0)?,
                    email_id: row.get(1)?,
                    recipient: row.get(2)?,
                    url: row.get(3)?,
                    link_type: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })
            .optional()?
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, email_id, recipient, url, link_type, created_at
                 FROM links WHERE recipient = ?1 ORDER BY created_at DESC LIMIT 1",
            )?;
            stmt.query_row(params![recipient], |row| {
                Ok(LinkRecord {
                    id: row.get(0)?,
                    email_id: row.get(1)?,
                    recipient: row.get(2)?,
                    url: row.get(3)?,
                    link_type: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })
            .optional()?
        };

        Ok(link)
    }

    pub fn list_links(&self, recipient: Option<&str>, limit: usize) -> Result<Vec<LinkRecord>, StorageError> {
        let conn = self.conn.lock().map_err(|_| StorageError::Poisoned)?;
        let mut links = Vec::new();

        if let Some(r) = recipient {
            let mut stmt = conn.prepare(
                "SELECT id, email_id, recipient, url, link_type, created_at
                 FROM links WHERE recipient = ?1 ORDER BY created_at DESC LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![r, limit as i64], |row| {
                Ok(LinkRecord {
                    id: row.get(0)?,
                    email_id: row.get(1)?,
                    recipient: row.get(2)?,
                    url: row.get(3)?,
                    link_type: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })?;
            for row in rows {
                links.push(row?);
            }
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, email_id, recipient, url, link_type, created_at
                 FROM links ORDER BY created_at DESC LIMIT ?1",
            )?;
            let rows = stmt.query_map(params![limit as i64], |row| {
                Ok(LinkRecord {
                    id: row.get(0)?,
                    email_id: row.get(1)?,
                    recipient: row.get(2)?,
                    url: row.get(3)?,
                    link_type: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })?;
            for row in rows {
                links.push(row?);
            }
        }

        Ok(links)
    }

    pub fn purge_older_than_days(&self, days: u32) -> Result<usize, StorageError> {
        let conn = self.conn.lock().map_err(|_| StorageError::Poisoned)?;
        let cutoff = Utc::now() - chrono::Duration::days(days as i64);
        let cutoff_str = cutoff.to_rfc3339();

        let rows = conn.execute(
            "DELETE FROM emails WHERE received_at < ?1",
            params![cutoff_str],
        )?;
        Ok(rows)
    }

    pub fn stats(&self) -> Result<InboxStats, StorageError> {
        let conn = self.conn.lock().map_err(|_| StorageError::Poisoned)?;

        let total_emails: i64 = conn.query_row("SELECT COUNT(*) FROM emails", [], |row| row.get(0))?;
        let total_otps: i64 = conn.query_row("SELECT COUNT(*) FROM otps", [], |row| row.get(0))?;
        let unconsumed_otps: i64 = conn.query_row(
            "SELECT COUNT(*) FROM otps WHERE consumed_at IS NULL",
            [],
            |row| row.get(0),
        )?;
        let total_links: i64 = conn.query_row("SELECT COUNT(*) FROM links", [], |row| row.get(0))?;

        Ok(InboxStats {
            total_emails: total_emails as usize,
            total_otps: total_otps as usize,
            unconsumed_otps: unconsumed_otps as usize,
            total_links: total_links as usize,
        })
    }
}

pub fn generate_uuid() -> String {
    Uuid::new_v4().to_string()
}

pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_lifecycle_in_memory() {
        let storage = InboxStorage::open_in_memory().expect("Open memory db");

        let email = EmailRecord {
            id: generate_uuid(),
            message_id: Some("msg-12345".to_string()),
            sender: "noreply@facebook.com".to_string(),
            recipient: "acc01@mydomain.com".to_string(),
            domain: "mydomain.com".to_string(),
            subject: Some("Your Facebook confirmation code is 849201".to_string()),
            body_text: Some("Here is your code: 849201".to_string()),
            body_html: None,
            received_at: now_rfc3339(),
            raw_headers: None,
            created_at: now_rfc3339(),
        };

        storage.save_email(&email).expect("Save email");

        let fetched_email = storage.get_email(&email.id).expect("Get email");
        assert!(fetched_email.is_some());
        assert_eq!(fetched_email.unwrap().recipient, "acc01@mydomain.com");

        let otp = OtpRecord {
            id: generate_uuid(),
            email_id: email.id.clone(),
            recipient: "acc01@mydomain.com".to_string(),
            otp_code: "849201".to_string(),
            service_name: Some("Facebook".to_string()),
            expires_at: None,
            consumed_at: None,
            consumed_by: None,
            created_at: now_rfc3339(),
        };

        storage.save_otp(&otp).expect("Save otp");

        let latest_otp = storage
            .get_latest_otp("acc01@mydomain.com", true)
            .expect("Get latest OTP");
        assert!(latest_otp.is_some());
        let otp_rec = latest_otp.unwrap();
        assert_eq!(otp_rec.otp_code, "849201");
        assert_eq!(otp_rec.service_name.as_deref(), Some("Facebook"));

        // Consume OTP
        let consumed = storage
            .consume_otp(&otp_rec.id, "device-alpha-1")
            .expect("Consume OTP");
        assert!(consumed);

        // Subsequent call for unconsumed OTP should return None
        let unconsumed = storage
            .get_latest_otp("acc01@mydomain.com", true)
            .expect("Get unconsumed OTP");
        assert!(unconsumed.is_none());

        // But unconsumed_only = false should return it
        let any_otp = storage
            .get_latest_otp("acc01@mydomain.com", false)
            .expect("Get any OTP");
        assert!(any_otp.is_some());

        // Test Links
        let link = LinkRecord {
            id: generate_uuid(),
            email_id: email.id.clone(),
            recipient: "acc01@mydomain.com".to_string(),
            url: "https://facebook.com/confirm?token=xyz987".to_string(),
            link_type: "verification".to_string(),
            created_at: now_rfc3339(),
        };

        storage.save_link(&link).expect("Save link");
        let fetched_link = storage
            .get_latest_link("acc01@mydomain.com", Some("verification"))
            .expect("Get link");
        assert!(fetched_link.is_some());
        assert_eq!(
            fetched_link.unwrap().url,
            "https://facebook.com/confirm?token=xyz987"
        );

        // Stats
        let stats = storage.stats().expect("Stats");
        assert_eq!(stats.total_emails, 1);
        assert_eq!(stats.total_otps, 1);
        assert_eq!(stats.unconsumed_otps, 0);
        assert_eq!(stats.total_links, 1);

        // Cascade delete
        let deleted = storage.delete_email(&email.id).expect("Delete email");
        assert!(deleted);
        let after_delete_stats = storage.stats().expect("Stats after delete");
        assert_eq!(after_delete_stats.total_emails, 0);
        assert_eq!(after_delete_stats.total_otps, 0);
        assert_eq!(after_delete_stats.total_links, 0);
    }
}
