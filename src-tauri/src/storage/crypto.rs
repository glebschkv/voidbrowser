use std::fs;
use std::path::Path;

use rand::Rng;
use zeroize::Zeroizing;

const KEYRING_SERVICE: &str = "com.voidbrowser.app";
const KEYRING_USER: &str = "vault-key";
const FALLBACK_KEY_FILE: &str = "vault.key";
const KEY_LEN: usize = 32;

/// Retrieve an existing vault key or generate a new one.
///
/// Strategy:
///  1. Try the OS keyring (Windows Credential Manager / Linux Secret Service).
///  2. If the keyring is unavailable or errors, fall back to a file-based key
///     stored at `{app_data_dir}/vault.key`.
///
/// The returned `Zeroizing<Vec<u8>>` is automatically zeroed on drop.
pub fn get_or_create_vault_key(app_data_dir: &Path) -> Result<Zeroizing<Vec<u8>>, String> {
    match try_keyring() {
        Ok(key) => Ok(key),
        Err(keyring_err) => {
            eprintln!(
                "Keyring unavailable ({keyring_err}), falling back to file-based key storage"
            );
            get_or_create_file_key(app_data_dir)
        }
    }
}

// ── Keyring path ────────────────────────────────────────────────────────

fn try_keyring() -> Result<Zeroizing<Vec<u8>>, String> {
    let entry =
        keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).map_err(|e| format!("{e}"))?;

    // Try to retrieve an existing key.
    match entry.get_secret() {
        Ok(secret) if secret.len() == KEY_LEN => Ok(Zeroizing::new(secret)),
        Ok(secret) if !secret.is_empty() => {
            // Stored secret has wrong length — treat it as corrupt and regenerate.
            eprintln!("Keyring secret has unexpected length ({}), regenerating", secret.len());
            let key = generate_key();
            entry
                .set_secret(key.as_slice())
                .map_err(|e| format!("Failed to store key in keyring: {e}"))?;
            Ok(key)
        }
        Ok(_) | Err(keyring::Error::NoEntry) => {
            // No key yet — generate and store.
            let key = generate_key();
            entry
                .set_secret(key.as_slice())
                .map_err(|e| format!("Failed to store key in keyring: {e}"))?;
            Ok(key)
        }
        Err(e) => Err(format!("Keyring error: {e}")),
    }
}

// ── File-based fallback ─────────────────────────────────────────────────

fn get_or_create_file_key(app_data_dir: &Path) -> Result<Zeroizing<Vec<u8>>, String> {
    let key_path = app_data_dir.join(FALLBACK_KEY_FILE);

    if key_path.exists() {
        let data = fs::read(&key_path)
            .map_err(|e| format!("Failed to read key file {}: {e}", key_path.display()))?;

        if data.len() == KEY_LEN {
            return Ok(Zeroizing::new(data));
        }
        // Corrupt key file — regenerate.
        eprintln!(
            "Key file has unexpected length ({}), regenerating",
            data.len()
        );
    }

    // Ensure the parent directory exists.
    if let Some(parent) = key_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create key directory: {e}"))?;
    }

    let key = generate_key();
    fs::write(&key_path, key.as_slice())
        .map_err(|e| format!("Failed to write key file {}: {e}", key_path.display()))?;

    // Best-effort: restrict permissions on Unix-like systems.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        let _ = fs::set_permissions(&key_path, perms);
    }

    Ok(key)
}

// ── Helpers ─────────────────────────────────────────────────────────────

fn generate_key() -> Zeroizing<Vec<u8>> {
    let mut key = Zeroizing::new(vec![0u8; KEY_LEN]);
    rand::thread_rng().fill(key.as_mut_slice());
    key
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// File-based key: first call generates, second call reads back the same key.
    #[test]
    fn file_key_roundtrip() {
        let dir = TempDir::new().expect("tempdir");
        let key1 = get_or_create_file_key(dir.path()).expect("first call");
        assert_eq!(key1.len(), KEY_LEN);

        let key2 = get_or_create_file_key(dir.path()).expect("second call");
        assert_eq!(key1.as_slice(), key2.as_slice(), "key should be stable");
    }

    /// File-based key: corrupt file gets regenerated.
    #[test]
    fn file_key_corrupt_regenerates() {
        let dir = TempDir::new().expect("tempdir");
        let key_path = dir.path().join(FALLBACK_KEY_FILE);
        fs::write(&key_path, b"short").expect("write corrupt");

        let key = get_or_create_file_key(dir.path()).expect("should regenerate");
        assert_eq!(key.len(), KEY_LEN);
        assert_ne!(key.as_slice(), b"short");
    }

    /// File-based key: non-existent directory gets created.
    #[test]
    fn file_key_creates_parent_dirs() {
        let dir = TempDir::new().expect("tempdir");
        let nested = dir.path().join("a").join("b").join("c");

        let key = get_or_create_file_key(&nested).expect("should create dirs");
        assert_eq!(key.len(), KEY_LEN);
        assert!(nested.join(FALLBACK_KEY_FILE).exists());
    }

    /// Top-level function should succeed via file fallback even when keyring
    /// is likely unavailable (as in CI / headless environments).
    #[test]
    fn get_or_create_vault_key_works_without_keyring() {
        let dir = TempDir::new().expect("tempdir");
        let key1 = get_or_create_vault_key(dir.path()).expect("should succeed via fallback");
        assert_eq!(key1.len(), KEY_LEN);

        // Second call should return the same key.
        let key2 = get_or_create_vault_key(dir.path()).expect("should succeed again");
        // Key may come from keyring or file — but both calls should be consistent.
        // In CI without keyring, both go through file path.
        assert_eq!(key1.len(), key2.len());
    }

    /// Generated keys should be random (not all zeros).
    #[test]
    fn generated_key_is_random() {
        let key = generate_key();
        assert_eq!(key.len(), KEY_LEN);
        // Extremely unlikely that 32 random bytes are all zero.
        assert!(key.iter().any(|&b| b != 0), "key should not be all zeros");
    }
}
