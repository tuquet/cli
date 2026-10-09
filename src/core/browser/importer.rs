use crate::infrastructure::db::AutomaDb;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvBrowserProfile {
    pub id: String,
    pub name: String,
    pub user_agent: Option<String>,
    pub timezone: Option<String>,
    pub proxy: Option<String>,
}

/// Parse CSV text containing browser profiles
pub fn parse_csv_profiles(csv_string: &str) -> Vec<CsvBrowserProfile> {
    let mut profiles = Vec::new();
    for line in csv_string.lines().skip(1) {
        let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if parts.is_empty() || parts[0].is_empty() {
            continue;
        }
        let id = parts[0].to_string();
        let name = parts.get(1).unwrap_or(&"").to_string();
        let user_agent = parts.get(2).and_then(|s| if s.is_empty() { None } else { Some(s.to_string()) });
        let timezone = parts.get(3).and_then(|s| if s.is_empty() { None } else { Some(s.to_string()) });
        let proxy = parts.get(4).and_then(|s| if s.is_empty() { None } else { Some(s.to_string()) });
        profiles.push(CsvBrowserProfile {
            id,
            name,
            user_agent,
            timezone,
            proxy,
        });
    }
    profiles
}

/// Batch import parsed browser profiles into SQLite database
pub fn import_csv_into_db(db: &AutomaDb, csv_string: &str) -> usize {
    let profiles = parse_csv_profiles(csv_string);
    let mut imported = 0;
    for p in profiles {
        let _ = db.browsers().create_browser(
            &p.id,
            &p.name,
            p.user_agent.as_deref(),
            p.timezone.as_deref(),
            p.proxy.as_deref(),
            None,
        );
        imported += 1;
    }
    imported
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_csv_profiles() {
        let csv = "id,name,userAgent,timezone,proxy\np1,Profile 1,Agent1,UTC,socks5://127.0.0.1:1080\np2,Profile 2,,,";
        let parsed = parse_csv_profiles(csv);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].id, "p1");
        assert_eq!(parsed[0].name, "Profile 1");
        assert_eq!(parsed[0].user_agent.as_deref(), Some("Agent1"));
        assert_eq!(parsed[0].timezone.as_deref(), Some("UTC"));
        assert_eq!(parsed[0].proxy.as_deref(), Some("socks5://127.0.0.1:1080"));

        assert_eq!(parsed[1].id, "p2");
        assert_eq!(parsed[1].name, "Profile 2");
        assert_eq!(parsed[1].user_agent, None);
        assert_eq!(parsed[1].timezone, None);
        assert_eq!(parsed[1].proxy, None);
    }
}
