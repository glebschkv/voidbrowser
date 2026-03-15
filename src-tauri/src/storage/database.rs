use std::path::Path;

use rusqlite::Connection;

use super::crypto;

/// Encrypted SQLCipher database wrapper.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open (or create) the encrypted vault database.
    ///
    /// On first run this generates a new encryption key and creates the schema.
    /// The key is stored in the OS keychain with a file-based fallback.
    pub fn open(app_data_dir: &Path) -> Result<Self, String> {
        let key = crypto::get_or_create_vault_key(app_data_dir)?;
        Self::open_with_key(app_data_dir, &key)
    }

    /// Open the database with an explicit key (used by tests to avoid
    /// keyring interference between parallel test runs).
    fn open_with_key(app_data_dir: &Path, key: &[u8]) -> Result<Self, String> {
        std::fs::create_dir_all(app_data_dir)
            .map_err(|e| format!("Failed to create app data dir: {e}"))?;

        let db_path = app_data_dir.join("vault.db");
        let conn = Connection::open(&db_path)
            .map_err(|e| format!("Failed to open database at {}: {e}", db_path.display()))?;

        let hex_key = hex_encode(key);

        // Set the SQLCipher encryption key via PRAGMA.
        conn.execute_batch(&format!("PRAGMA key = \"x'{hex_key}'\";"))
            .map_err(|e| format!("Failed to set database encryption key: {e}"))?;

        // Verify the key works by reading the schema.
        conn.execute_batch("SELECT count(*) FROM sqlite_master;")
            .map_err(|_| {
                "Database key verification failed — the vault.db file may be corrupted \
                 or the encryption key has changed."
                    .to_string()
            })?;

        let db = Self { conn };
        db.run_migrations()?;
        Ok(db)
    }

    /// Open an in-memory encrypted database (for testing).
    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self, String> {
        let conn =
            Connection::open_in_memory().map_err(|e| format!("Failed to open in-memory db: {e}"))?;
        let db = Self { conn };
        db.run_migrations()?;
        Ok(db)
    }

    /// Borrow the underlying connection.
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    fn run_migrations(&self) -> Result<(), String> {
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS bookmarks (
                    id          TEXT PRIMARY KEY,
                    url         TEXT NOT NULL,
                    title       TEXT NOT NULL,
                    folder      TEXT,
                    favicon_data BLOB,
                    created_at  TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS settings (
                    key   TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );",
            )
            .map_err(|e| format!("Migration failed: {e}"))
    }
}

/// Encode bytes as a lowercase hex string.
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_in_memory_succeeds() {
        let db = Database::open_in_memory().expect("should open");
        // Verify tables exist.
        let count: i64 = db
            .conn()
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name IN ('bookmarks', 'settings')",
                [],
                |row| row.get(0),
            )
            .expect("query");
        assert_eq!(count, 2);
    }

    #[test]
    fn migrations_are_idempotent() {
        let db = Database::open_in_memory().expect("should open");
        // Running migrations again should not error.
        db.run_migrations().expect("re-migration should succeed");
    }

    #[test]
    fn open_on_disk_roundtrip() {
        // Use a fixed key to avoid keyring interference between tests.
        let key = vec![0xAB_u8; 32];
        let dir = tempfile::TempDir::new().expect("tempdir");
        {
            let db = Database::open_with_key(dir.path(), &key).expect("first open");
            db.conn()
                .execute(
                    "INSERT INTO settings (key, value) VALUES ('test_key', 'test_val')",
                    [],
                )
                .expect("insert");
        }
        // Re-open with the same key and verify data persists.
        {
            let db = Database::open_with_key(dir.path(), &key).expect("second open");
            let val: String = db
                .conn()
                .query_row(
                    "SELECT value FROM settings WHERE key = 'test_key'",
                    [],
                    |row| row.get(0),
                )
                .expect("query");
            assert_eq!(val, "test_val");
        }
        // Opening with a WRONG key must fail.
        {
            let wrong_key = vec![0xCD_u8; 32];
            let result = Database::open_with_key(dir.path(), &wrong_key);
            assert!(result.is_err(), "wrong key should fail to open");
        }
    }

    #[test]
    fn hex_encode_works() {
        assert_eq!(hex_encode(&[0x0a, 0xff, 0x00]), "0aff00");
    }
}
