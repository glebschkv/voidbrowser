use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub url: String,
    pub title: String,
    pub timestamp: u64,
}

/// Session-only browsing history. Purely in-memory — never touches disk.
pub struct SessionHistory {
    entries: Vec<HistoryEntry>,
}

impl SessionHistory {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Add a history entry. If the most recent entry has the same URL,
    /// update its title and timestamp instead of creating a duplicate.
    pub fn add_entry(&mut self, url: &str, title: &str) {
        // Skip internal pages.
        if url.starts_with("void://") || url.is_empty() {
            return;
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        // Deduplicate: if the last entry has the same URL, update it.
        if let Some(last) = self.entries.last_mut() {
            if last.url == url {
                last.title = title.to_string();
                last.timestamp = now;
                return;
            }
        }

        self.entries.push(HistoryEntry {
            url: url.to_string(),
            title: title.to_string(),
            timestamp: now,
        });
    }

    /// Update the title of the most recent entry matching the given URL.
    pub fn update_title(&mut self, url: &str, title: &str) {
        for entry in self.entries.iter_mut().rev() {
            if entry.url == url {
                entry.title = title.to_string();
                return;
            }
        }
    }

    /// Search history entries by title or URL (case-insensitive).
    /// Returns most recent first, limited to 10 results.
    pub fn search(&self, query: &str) -> Vec<HistoryEntry> {
        let query_lower = query.to_lowercase();
        self.entries
            .iter()
            .rev()
            .filter(|e| {
                e.title.to_lowercase().contains(&query_lower)
                    || e.url.to_lowercase().contains(&query_lower)
            })
            .take(10)
            .cloned()
            .collect()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_search() {
        let mut h = SessionHistory::new();
        h.add_entry("https://example.com", "Example");
        h.add_entry("https://rust-lang.org", "Rust");

        let results = h.search("rust");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].url, "https://rust-lang.org");
    }

    #[test]
    fn deduplicates_consecutive_same_url() {
        let mut h = SessionHistory::new();
        h.add_entry("https://example.com", "Page 1");
        h.add_entry("https://example.com", "Page 1 - Updated");

        let results = h.search("example");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Page 1 - Updated");
    }

    #[test]
    fn does_not_deduplicate_nonconsecutive() {
        let mut h = SessionHistory::new();
        h.add_entry("https://a.com", "A");
        h.add_entry("https://b.com", "B");
        h.add_entry("https://a.com", "A again");

        let results = h.search("a.com");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn skips_void_urls() {
        let mut h = SessionHistory::new();
        h.add_entry("void://newtab", "New Tab");
        h.add_entry("", "");

        let results = h.search("");
        assert!(results.is_empty());
    }

    #[test]
    fn search_is_case_insensitive() {
        let mut h = SessionHistory::new();
        h.add_entry("https://GitHub.com", "GitHub");

        assert_eq!(h.search("github").len(), 1);
        assert_eq!(h.search("GITHUB").len(), 1);
    }

    #[test]
    fn search_limits_to_10() {
        let mut h = SessionHistory::new();
        for i in 0..20 {
            h.add_entry(&format!("https://example.com/{i}"), &format!("Page {i}"));
        }
        let results = h.search("example");
        assert_eq!(results.len(), 10);
    }

    #[test]
    fn search_returns_most_recent_first() {
        let mut h = SessionHistory::new();
        h.add_entry("https://a.com", "First");
        h.add_entry("https://b.com", "Second");

        let results = h.search("");
        assert_eq!(results[0].url, "https://b.com");
        assert_eq!(results[1].url, "https://a.com");
    }

    #[test]
    fn update_title_works() {
        let mut h = SessionHistory::new();
        h.add_entry("https://example.com", "Loading...");
        h.update_title("https://example.com", "Example Domain");

        let results = h.search("example");
        assert_eq!(results[0].title, "Example Domain");
    }

    #[test]
    fn clear_empties_history() {
        let mut h = SessionHistory::new();
        h.add_entry("https://example.com", "Ex");
        h.clear();
        assert!(h.search("").is_empty());
    }
}
