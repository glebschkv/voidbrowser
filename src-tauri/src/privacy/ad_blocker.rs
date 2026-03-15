use std::collections::{HashMap, HashSet};

use adblock::lists::FilterSet;
use adblock::request::Request;
use adblock::Engine;

/// Ad and tracker blocker powered by Brave's adblock-rust engine.
///
/// The adblock `Engine` uses `Rc` internally and is not `Send`/`Sync`.
/// We wrap it to allow storage in Tauri managed state, since all access
/// happens through a `Mutex` which guarantees single-threaded access.
#[allow(dead_code)]
pub struct AdBlocker {
    engine: Engine,
}

// SAFETY: Engine uses Rc internally but we protect all access behind a Mutex,
// ensuring single-threaded access. The Engine is created on one thread and
// accessed sequentially via the Mutex lock.
unsafe impl Send for AdBlocker {}
// SAFETY: All access is serialized through Mutex<AdBlocker>.
unsafe impl Sync for AdBlocker {}

#[allow(dead_code)]
impl AdBlocker {
    /// Create a new AdBlocker by loading the bundled EasyList and EasyPrivacy filter lists.
    pub fn new() -> Self {
        let mut filter_set = FilterSet::new(true);

        let easylist = include_str!("../../resources/filter_lists/easylist.txt");
        let easyprivacy = include_str!("../../resources/filter_lists/easyprivacy.txt");

        let rules: Vec<String> = easylist
            .lines()
            .chain(easyprivacy.lines())
            .map(String::from)
            .collect();
        eprintln!("Ad blocker: loaded {} filter rules", rules.len());
        filter_set.add_filters(&rules, Default::default());

        let engine = Engine::from_filter_set(filter_set, true);
        Self { engine }
    }

    /// Check whether a request URL should be blocked.
    ///
    /// - `url`: the URL being requested
    /// - `source_url`: the page URL that initiated the request (for domain-specific rules)
    /// - `resource_type`: the type of resource ("script", "image", "stylesheet", etc.)
    pub fn should_block(&self, url: &str, source_url: &str, resource_type: &str) -> bool {
        let request = match Request::new(url, source_url, resource_type) {
            Ok(r) => r,
            Err(_) => return false,
        };
        let result = self.engine.check_network_request(&request);
        result.matched
    }
}

/// Per-tab shield state tracking blocked request counts and disabled tabs.
#[allow(dead_code)]
pub struct ShieldState {
    blocked_counts: HashMap<String, u64>,
    disabled_tabs: HashSet<String>,
}

#[allow(dead_code)]
impl ShieldState {
    pub fn new() -> Self {
        Self {
            blocked_counts: HashMap::new(),
            disabled_tabs: HashSet::new(),
        }
    }

    /// Increment the blocked count for a tab, returning the new count.
    pub fn increment(&mut self, tab_id: &str) -> u64 {
        let count = self.blocked_counts.entry(tab_id.to_string()).or_insert(0);
        *count += 1;
        *count
    }

    /// Get the current blocked count for a tab.
    pub fn get_count(&self, tab_id: &str) -> u64 {
        self.blocked_counts.get(tab_id).copied().unwrap_or(0)
    }

    /// Reset the blocked count for a tab (e.g., on navigation).
    pub fn reset(&mut self, tab_id: &str) {
        self.blocked_counts.remove(tab_id);
    }

    /// Remove all state for a tab (when it's closed).
    pub fn remove_tab(&mut self, tab_id: &str) {
        self.blocked_counts.remove(tab_id);
        self.disabled_tabs.remove(tab_id);
    }

    /// Check whether the shield is disabled for a tab.
    pub fn is_disabled(&self, tab_id: &str) -> bool {
        self.disabled_tabs.contains(tab_id)
    }

    /// Toggle the shield for a tab. Returns `true` if shield is now enabled.
    pub fn toggle(&mut self, tab_id: &str) -> bool {
        if self.disabled_tabs.contains(tab_id) {
            self.disabled_tabs.remove(tab_id);
            true
        } else {
            self.disabled_tabs.insert(tab_id.to_string());
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shield_state_increment() {
        let mut state = ShieldState::new();
        assert_eq!(state.get_count("tab1"), 0);
        assert_eq!(state.increment("tab1"), 1);
        assert_eq!(state.increment("tab1"), 2);
        assert_eq!(state.get_count("tab1"), 2);
    }

    #[test]
    fn test_shield_state_toggle() {
        let mut state = ShieldState::new();
        assert!(!state.is_disabled("tab1"));
        let enabled = state.toggle("tab1");
        assert!(!enabled);
        assert!(state.is_disabled("tab1"));
        let enabled = state.toggle("tab1");
        assert!(enabled);
        assert!(!state.is_disabled("tab1"));
    }

    #[test]
    fn test_shield_state_remove_tab() {
        let mut state = ShieldState::new();
        state.increment("tab1");
        state.toggle("tab1");
        state.remove_tab("tab1");
        assert_eq!(state.get_count("tab1"), 0);
        assert!(!state.is_disabled("tab1"));
    }

    #[test]
    fn test_ad_blocker_blocks_known_ad_domain() {
        let blocker = AdBlocker::new();
        let blocked = blocker.should_block(
            "https://pagead2.googlesyndication.com/pagead/js/adsbygoogle.js",
            "https://example.com",
            "script",
        );
        assert!(blocked, "Expected googlesyndication.com ad script to be blocked");
    }

    #[test]
    fn test_ad_blocker_allows_normal_content() {
        let blocker = AdBlocker::new();
        let blocked = blocker.should_block(
            "https://example.com/index.html",
            "https://example.com",
            "document",
        );
        assert!(!blocked, "Expected normal page to not be blocked");
    }
}
