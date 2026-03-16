use std::collections::HashMap;

use rusqlite::params;

use super::database::Database;

/// Default settings matching CLAUDE.md specification.
pub const DEFAULTS: &[(&str, &str)] = &[
    ("search_engine", "duckduckgo"),
    ("theme", "dark"),
    ("accent_color", "#6366f1"),
    ("font_size", "medium"),
    ("sidebar_position", "left"),
    ("shield_enabled", "true"),
    ("third_party_cookies", "block"),
    ("first_party_cookies", "session_only"),
    ("fingerprint_resistance", "true"),
    ("https_only", "true"),
    ("restore_tabs_on_start", "false"),
    ("history_mode", "session_only"),
    ("download_location", "~/Downloads"),
];

/// Get a single setting value from the database.
pub fn get_setting(db: &Database, key: &str) -> Result<Option<String>, String> {
    let mut stmt = db
        .conn()
        .prepare("SELECT value FROM settings WHERE key = ?1")
        .map_err(|e| format!("Failed to prepare setting query: {e}"))?;

    let result = stmt
        .query_row(params![key], |row| row.get::<_, String>(0))
        .ok();

    Ok(result)
}

/// Get a setting, falling back to the compiled default if not in the database.
pub fn get_setting_or_default(db: &Database, key: &str) -> String {
    get_setting(db, key)
        .ok()
        .flatten()
        .unwrap_or_else(|| {
            DEFAULTS
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v.to_string())
                .unwrap_or_default()
        })
}

/// Set a setting value (insert or replace).
pub fn set_setting(db: &Database, key: &str, value: &str) -> Result<(), String> {
    db.conn()
        .execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )
        .map_err(|e| format!("Failed to set setting: {e}"))?;
    Ok(())
}

/// Retrieve all settings from the database, merged with defaults.
/// Database values override defaults.
pub fn get_all_settings(db: &Database) -> Result<HashMap<String, String>, String> {
    // Start with defaults.
    let mut settings: HashMap<String, String> = DEFAULTS
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    // Override with database values.
    let mut stmt = db
        .conn()
        .prepare("SELECT key, value FROM settings")
        .map_err(|e| format!("Failed to prepare settings query: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| format!("Failed to query settings: {e}"))?;

    for row in rows {
        let (key, value) = row.map_err(|e| format!("Failed to read setting row: {e}"))?;
        settings.insert(key, value);
    }

    Ok(settings)
}

/// Map a search engine name to its URL template.
/// The `%s` placeholder is replaced with the encoded query at resolve time.
pub fn search_engine_url(engine_name: &str) -> &str {
    match engine_name {
        "duckduckgo" => "https://duckduckgo.com/?q=%s",
        "brave" => "https://search.brave.com/search?q=%s",
        "startpage" => "https://www.startpage.com/do/dsearch?query=%s",
        "google" => "https://www.google.com/search?q=%s",
        _ => "https://duckduckgo.com/?q=%s",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::database::Database;

    fn test_db() -> Database {
        Database::open_in_memory().expect("open in-memory db")
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let db = test_db();
        let val = get_setting(&db, "nonexistent").expect("query");
        assert!(val.is_none());
    }

    #[test]
    fn set_and_get_setting() {
        let db = test_db();
        set_setting(&db, "theme", "light").expect("set");
        let val = get_setting(&db, "theme").expect("get");
        assert_eq!(val, Some("light".to_string()));
    }

    #[test]
    fn upsert_overwrites_existing() {
        let db = test_db();
        set_setting(&db, "theme", "dark").expect("set1");
        set_setting(&db, "theme", "light").expect("set2");
        let val = get_setting(&db, "theme").expect("get");
        assert_eq!(val, Some("light".to_string()));
    }

    #[test]
    fn get_setting_or_default_uses_db_value() {
        let db = test_db();
        set_setting(&db, "search_engine", "google").expect("set");
        assert_eq!(get_setting_or_default(&db, "search_engine"), "google");
    }

    #[test]
    fn get_setting_or_default_falls_back() {
        let db = test_db();
        assert_eq!(get_setting_or_default(&db, "search_engine"), "duckduckgo");
    }

    #[test]
    fn get_all_settings_merges_defaults() {
        let db = test_db();
        set_setting(&db, "theme", "light").expect("set");
        set_setting(&db, "custom_key", "custom_val").expect("set");

        let all = get_all_settings(&db).expect("get all");
        assert_eq!(all.get("theme"), Some(&"light".to_string()));
        assert_eq!(
            all.get("search_engine"),
            Some(&"duckduckgo".to_string())
        );
        assert_eq!(all.get("custom_key"), Some(&"custom_val".to_string()));
    }

    #[test]
    fn search_engine_url_mapping() {
        assert!(search_engine_url("duckduckgo").contains("duckduckgo.com"));
        assert!(search_engine_url("brave").contains("search.brave.com"));
        assert!(search_engine_url("startpage").contains("startpage.com"));
        assert!(search_engine_url("google").contains("google.com"));
        // Unknown engine falls back to DuckDuckGo.
        assert!(search_engine_url("unknown").contains("duckduckgo.com"));
    }
}
