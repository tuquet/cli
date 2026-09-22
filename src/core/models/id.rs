use std::fmt;
use std::ops::Deref;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

macro_rules! define_id {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, Default)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            pub fn new(id: impl Into<String>) -> Self {
                Self(id.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Deref for $name {
            type Target = str;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_string())
            }
        }

        impl rusqlite::types::ToSql for $name {
            fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
                self.0.to_sql()
            }
        }

        impl rusqlite::types::FromSql for $name {
            fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
                String::column_result(value).map(Self)
            }
        }
    };
}

define_id!(JobId, "Strongly-typed Job Identifier");
define_id!(BrowserId, "Strongly-typed Browser Identifier");
define_id!(WorkflowId, "Strongly-typed Workflow Identifier");
define_id!(CampaignId, "Strongly-typed Campaign Identifier");
define_id!(TableId, "Strongly-typed Table Identifier");
define_id!(VariableKey, "Strongly-typed Storage Variable Key");
define_id!(CredentialKey, "Strongly-typed Storage Credential Key");

impl BrowserId {
    /// Checks whether the browser profile identifier is valid (non-empty alphanumeric, hyphens, underscores)
    pub fn is_valid(&self) -> bool {
        !self.0.is_empty() && self.0.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    }

    /// Validates the browser identifier or returns an AutomaError::BadRequest
    pub fn validate(&self) -> Result<(), crate::core::error::AutomaError> {
        if self.is_valid() {
            Ok(())
        } else {
            Err(crate::core::error::AutomaError::BadRequest("Invalid browser ID".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_typed_id_deref_and_as_ref() {
        let wid = WorkflowId::new("wf_123");
        assert_eq!(&*wid, "wf_123");
        assert_eq!(wid.as_ref(), "wf_123");
        assert_eq!(wid.as_str(), "wf_123");
        assert_eq!(format!("{}", wid), "wf_123");
    }

    #[test]
    fn test_typed_id_serde_roundtrip() {
        let jid = JobId::new("job_456");
        let serialized = serde_json::to_string(&jid).unwrap();
        assert_eq!(serialized, "\"job_456\"");
        let deserialized: JobId = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, jid);
    }

    #[test]
    fn test_typed_id_rusqlite_roundtrip() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute("CREATE TABLE test_ids (id TEXT PRIMARY KEY)", []).unwrap();

        let tid = TableId::new("tbl_abc");
        conn.execute("INSERT INTO test_ids (id) VALUES (?1)", rusqlite::params![&tid]).unwrap();

        let fetched: TableId = conn.query_row(
            "SELECT id FROM test_ids WHERE id = ?1",
            rusqlite::params![&tid],
            |row| row.get(0)
        ).unwrap();

        assert_eq!(fetched, tid);
    }

    #[test]
    fn test_browser_id_validation() {
        let valid = BrowserId::new("profile-1_alpha");
        assert!(valid.is_valid());
        assert!(valid.validate().is_ok());

        let invalid_chars = BrowserId::new("invalid/id!");
        assert!(!invalid_chars.is_valid());
        assert!(invalid_chars.validate().is_err());

        let empty = BrowserId::new("");
        assert!(!empty.is_valid());
        assert!(empty.validate().is_err());
    }
}
