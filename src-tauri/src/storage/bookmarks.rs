use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::database::Database;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bookmark {
    pub id: String,
    pub url: String,
    pub title: String,
    pub folder: Option<String>,
    pub favicon_data: Option<Vec<u8>>,
    pub created_at: String,
}

pub fn add_bookmark(
    db: &Database,
    url: &str,
    title: &str,
    folder: Option<&str>,
) -> Result<Bookmark, String> {
    let id = Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().to_rfc3339();

    db.conn()
        .execute(
            "INSERT INTO bookmarks (id, url, title, folder, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, url, title, folder, created_at],
        )
        .map_err(|e| format!("Failed to add bookmark: {e}"))?;

    Ok(Bookmark {
        id,
        url: url.to_string(),
        title: title.to_string(),
        folder: folder.map(String::from),
        favicon_data: None,
        created_at,
    })
}

pub fn remove_bookmark(db: &Database, id: &str) -> Result<(), String> {
    let rows = db
        .conn()
        .execute("DELETE FROM bookmarks WHERE id = ?1", params![id])
        .map_err(|e| format!("Failed to remove bookmark: {e}"))?;

    if rows == 0 {
        return Err(format!("Bookmark not found: {id}"));
    }
    Ok(())
}

pub fn update_bookmark(
    db: &Database,
    id: &str,
    url: Option<&str>,
    title: Option<&str>,
    folder: Option<Option<&str>>,
) -> Result<(), String> {
    let mut sets = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(u) = url {
        sets.push("url = ?");
        values.push(Box::new(u.to_string()));
    }
    if let Some(t) = title {
        sets.push("title = ?");
        values.push(Box::new(t.to_string()));
    }
    if let Some(f) = folder {
        sets.push("folder = ?");
        values.push(Box::new(f.map(String::from)));
    }

    if sets.is_empty() {
        return Ok(());
    }

    // Build numbered placeholders: url = ?1, title = ?2, ...
    let set_clause: String = sets
        .iter()
        .enumerate()
        .map(|(i, s)| s.replace('?', &format!("?{}", i + 1)))
        .collect::<Vec<_>>()
        .join(", ");

    let id_param = values.len() + 1;
    let sql = format!("UPDATE bookmarks SET {set_clause} WHERE id = ?{id_param}");
    values.push(Box::new(id.to_string()));

    let params_refs: Vec<&dyn rusqlite::types::ToSql> = values.iter().map(|v| v.as_ref()).collect();

    let rows = db
        .conn()
        .execute(&sql, params_refs.as_slice())
        .map_err(|e| format!("Failed to update bookmark: {e}"))?;

    if rows == 0 {
        return Err(format!("Bookmark not found: {id}"));
    }
    Ok(())
}

pub fn get_bookmarks(db: &Database, folder: Option<&str>) -> Result<Vec<Bookmark>, String> {
    let mut stmt = match folder {
        Some(f) => {
            let mut s = db
                .conn()
                .prepare("SELECT id, url, title, folder, favicon_data, created_at FROM bookmarks WHERE folder = ?1 ORDER BY created_at DESC")
                .map_err(|e| format!("Failed to prepare query: {e}"))?;
            let rows = s
                .query_map(params![f], row_to_bookmark)
                .map_err(|e| format!("Failed to query bookmarks: {e}"))?;
            return rows
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("Failed to read bookmark row: {e}"));
        }
        None => db
            .conn()
            .prepare("SELECT id, url, title, folder, favicon_data, created_at FROM bookmarks WHERE folder IS NULL ORDER BY created_at DESC")
            .map_err(|e| format!("Failed to prepare query: {e}"))?,
    };

    let rows = stmt
        .query_map([], row_to_bookmark)
        .map_err(|e| format!("Failed to query bookmarks: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to read bookmark row: {e}"))
}

pub fn search_bookmarks(db: &Database, query: &str) -> Result<Vec<Bookmark>, String> {
    let pattern = format!("%{query}%");
    let mut stmt = db
        .conn()
        .prepare(
            "SELECT id, url, title, folder, favicon_data, created_at
             FROM bookmarks
             WHERE title LIKE ?1 OR url LIKE ?2
             ORDER BY created_at DESC
             LIMIT 20",
        )
        .map_err(|e| format!("Failed to prepare search: {e}"))?;

    let rows = stmt
        .query_map(params![pattern, pattern], row_to_bookmark)
        .map_err(|e| format!("Failed to search bookmarks: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to read bookmark row: {e}"))
}

fn row_to_bookmark(row: &rusqlite::Row) -> rusqlite::Result<Bookmark> {
    Ok(Bookmark {
        id: row.get(0)?,
        url: row.get(1)?,
        title: row.get(2)?,
        folder: row.get(3)?,
        favicon_data: row.get(4)?,
        created_at: row.get(5)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        Database::open_in_memory().expect("open in-memory db")
    }

    #[test]
    fn add_and_get_bookmark() {
        let db = test_db();
        let bm = add_bookmark(&db, "https://example.com", "Example", None).expect("add");
        assert!(!bm.id.is_empty());
        assert_eq!(bm.url, "https://example.com");

        let bookmarks = get_bookmarks(&db, None).expect("get");
        assert_eq!(bookmarks.len(), 1);
        assert_eq!(bookmarks[0].id, bm.id);
    }

    #[test]
    fn add_bookmark_with_folder() {
        let db = test_db();
        add_bookmark(&db, "https://a.com", "A", Some("news")).expect("add");
        add_bookmark(&db, "https://b.com", "B", None).expect("add");

        let root = get_bookmarks(&db, None).expect("root");
        assert_eq!(root.len(), 1);
        assert_eq!(root[0].url, "https://b.com");

        let news = get_bookmarks(&db, Some("news")).expect("news");
        assert_eq!(news.len(), 1);
        assert_eq!(news[0].url, "https://a.com");
    }

    #[test]
    fn remove_bookmark_works() {
        let db = test_db();
        let bm = add_bookmark(&db, "https://example.com", "Ex", None).expect("add");
        remove_bookmark(&db, &bm.id).expect("remove");

        let bookmarks = get_bookmarks(&db, None).expect("get");
        assert!(bookmarks.is_empty());
    }

    #[test]
    fn remove_nonexistent_bookmark_errors() {
        let db = test_db();
        let result = remove_bookmark(&db, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn update_bookmark_works() {
        let db = test_db();
        let bm = add_bookmark(&db, "https://old.com", "Old", None).expect("add");
        update_bookmark(&db, &bm.id, Some("https://new.com"), Some("New"), None).expect("update");

        let bookmarks = get_bookmarks(&db, None).expect("get");
        assert_eq!(bookmarks[0].url, "https://new.com");
        assert_eq!(bookmarks[0].title, "New");
    }

    #[test]
    fn search_bookmarks_works() {
        let db = test_db();
        add_bookmark(&db, "https://rust-lang.org", "Rust Language", None).expect("add");
        add_bookmark(&db, "https://solidjs.com", "SolidJS", None).expect("add");
        add_bookmark(&db, "https://docs.rs", "Docs.rs - Rust Docs", None).expect("add");

        let results = search_bookmarks(&db, "rust").expect("search");
        assert_eq!(results.len(), 2);

        let results = search_bookmarks(&db, "solidjs").expect("search");
        assert_eq!(results.len(), 1);

        let results = search_bookmarks(&db, "zzzzz").expect("search");
        assert!(results.is_empty());
    }
}
