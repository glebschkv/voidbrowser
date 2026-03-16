use std::collections::{HashMap, HashSet};

/// State for HTTPS-Only mode.
///
/// Tracks domains the user has explicitly allowed over HTTP ("proceed anyway")
/// and per-tab HTTPS upgrade counts for the privacy dashboard.
/// All state is session-only and resets on browser close.
#[allow(dead_code)]
pub struct HttpsOnlyState {
    allowed_http_domains: HashSet<String>,
    upgrade_counts: HashMap<String, u64>,
    /// Tracks tab IDs that have a pending HTTPS warning page displayed.
    /// Maps tab_id -> original HTTP URL.
    pending_warnings: HashMap<String, String>,
    /// Total HTTPS upgrades across all tabs this session.
    total_upgrades: u64,
}

#[allow(dead_code)]
impl HttpsOnlyState {
    pub fn new() -> Self {
        Self {
            allowed_http_domains: HashSet::new(),
            upgrade_counts: HashMap::new(),
            pending_warnings: HashMap::new(),
            total_upgrades: 0,
        }
    }

    /// Check if the user has allowed HTTP for this domain during this session.
    pub fn is_http_allowed(&self, domain: &str) -> bool {
        self.allowed_http_domains.contains(domain)
    }

    /// Add a domain to the allowed-HTTP set for this session.
    pub fn allow_http(&mut self, domain: &str) {
        self.allowed_http_domains.insert(domain.to_string());
    }

    /// Record an HTTPS upgrade for a tab. Returns the new count.
    pub fn record_upgrade(&mut self, tab_id: &str) -> u64 {
        let count = self.upgrade_counts.entry(tab_id.to_string()).or_insert(0);
        *count += 1;
        self.total_upgrades += 1;
        *count
    }

    /// Get the number of HTTPS upgrades for a tab.
    pub fn get_upgrade_count(&self, tab_id: &str) -> u64 {
        self.upgrade_counts.get(tab_id).copied().unwrap_or(0)
    }

    /// Get total HTTPS upgrades across all tabs this session.
    pub fn get_total_upgrades(&self) -> u64 {
        self.total_upgrades
    }

    /// Reset upgrade count for a tab (on new navigation).
    pub fn reset_tab(&mut self, tab_id: &str) {
        self.upgrade_counts.remove(tab_id);
        self.pending_warnings.remove(tab_id);
    }

    /// Remove all state for a closed tab.
    pub fn remove_tab(&mut self, tab_id: &str) {
        self.upgrade_counts.remove(tab_id);
        self.pending_warnings.remove(tab_id);
    }

    /// Set a pending warning for a tab (about:blank is loading, will inject warning HTML).
    pub fn set_pending_warning(&mut self, tab_id: &str, original_http_url: &str) {
        self.pending_warnings
            .insert(tab_id.to_string(), original_http_url.to_string());
    }

    /// Take and clear the pending warning for a tab, returning the original HTTP URL if present.
    pub fn take_pending_warning(&mut self, tab_id: &str) -> Option<String> {
        self.pending_warnings.remove(tab_id)
    }

    /// Check if a tab has a pending warning.
    pub fn has_pending_warning(&self, tab_id: &str) -> bool {
        self.pending_warnings.contains_key(tab_id)
    }
}

/// Check if a URL should be upgraded from HTTP to HTTPS.
/// Returns `Some(https_url)` if the URL uses `http://`, `None` otherwise.
#[allow(dead_code)]
pub fn should_upgrade(url: &str) -> Option<String> {
    url.strip_prefix("http://")
        .map(|rest| format!("https://{rest}"))
}

/// Extract the host/domain from a URL string.
pub fn extract_domain(url: &str) -> Option<String> {
    let parsed: url::Url = url.parse().ok()?;
    parsed.host_str().map(|h| h.to_string())
}

/// Generate the HTTPS warning page HTML for injection into a webview.
///
/// The page warns the user that the site is not available over HTTPS and
/// provides a "proceed anyway" button that invokes a Tauri command.
pub fn generate_https_warning_page(original_http_url: &str, tab_id: &str) -> String {
    let domain = extract_domain(original_http_url).unwrap_or_else(|| original_http_url.to_string());
    // Escape for safe embedding in JS strings
    let escaped_url = original_http_url
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('"', "&quot;");
    let escaped_tab_id = tab_id
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('"', "&quot;");
    let escaped_domain = domain
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;");

    // Build the script using .replace() instead of format!() to avoid issues
    // with `#` in HTML color codes being interpreted as Rust raw string prefixes.
    let template = r##"
        document.open();
        document.write(`<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Connection Not Secure</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            background: #171717;
            color: #f5f5f5;
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            height: 100vh;
            user-select: none;
            padding: 2rem;
        }
        .warning-icon {
            width: 64px;
            height: 64px;
            margin-bottom: 1.5rem;
        }
        h1 {
            font-size: 1.5rem;
            font-weight: 600;
            margin-bottom: 1rem;
            color: #fbbf24;
        }
        .domain {
            font-family: monospace;
            font-size: 1.1rem;
            color: #a3a3a3;
            background: #262626;
            padding: 0.5rem 1rem;
            border-radius: 6px;
            margin-bottom: 1.5rem;
        }
        p {
            color: #a3a3a3;
            font-size: 0.9rem;
            max-width: 480px;
            text-align: center;
            line-height: 1.5;
            margin-bottom: 2rem;
        }
        .buttons {
            display: flex;
            gap: 1rem;
        }
        button {
            padding: 10px 24px;
            border-radius: 6px;
            border: none;
            font-size: 0.9rem;
            cursor: pointer;
            font-weight: 500;
            transition: background 0.2s;
        }
        .btn-back {
            background: #6366f1;
            color: white;
        }
        .btn-back:hover {
            background: #818cf8;
        }
        .btn-proceed {
            background: #262626;
            color: #a3a3a3;
            border: 1px solid #404040;
        }
        .btn-proceed:hover {
            background: #404040;
            color: #f5f5f5;
        }
    </style>
</head>
<body>
    <svg class="warning-icon" viewBox="0 0 24 24" fill="none" stroke="#fbbf24" stroke-width="1.5">
        <path d="M12 2L2 22h20L12 2z" stroke-linejoin="round"/>
        <line x1="12" y1="9" x2="12" y2="15" stroke-linecap="round"/>
        <circle cx="12" cy="18" r="0.5" fill="#fbbf24"/>
    </svg>
    <h1>Connection Not Secure</h1>
    <div class="domain">__VOID_DOMAIN__</div>
    <p>
        This site is not available over a secure (HTTPS) connection.
        Your connection to this site is not encrypted, which means information
        you send could be read by others on the network.
    </p>
    <div class="buttons">
        <button class="btn-back" onclick="window.history.back()">Go Back</button>
        <button class="btn-proceed" onclick="proceedHttp()">Proceed to HTTP Site</button>
    </div>
    <script>
        function proceedHttp() {
            if (window.__TAURI__ && window.__TAURI__.core) {
                window.__TAURI__.core.invoke('allow_http_and_navigate', {
                    tabId: '__VOID_TAB_ID__',
                    url: '__VOID_URL__'
                });
            }
        }
    </script>
</body>
</html>`);
        document.close();
        "##;

    template
        .replace("__VOID_DOMAIN__", &escaped_domain)
        .replace("__VOID_TAB_ID__", &escaped_tab_id)
        .replace("__VOID_URL__", &escaped_url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_upgrade_http() {
        assert_eq!(
            should_upgrade("http://example.com"),
            Some("https://example.com".to_string())
        );
        assert_eq!(
            should_upgrade("http://example.com/path?q=1"),
            Some("https://example.com/path?q=1".to_string())
        );
    }

    #[test]
    fn test_should_upgrade_https_returns_none() {
        assert_eq!(should_upgrade("https://example.com"), None);
    }

    #[test]
    fn test_should_upgrade_non_http_returns_none() {
        assert_eq!(should_upgrade("about:blank"), None);
        assert_eq!(should_upgrade("data:text/html,hello"), None);
        assert_eq!(should_upgrade("ftp://example.com"), None);
    }

    #[test]
    fn test_extract_domain() {
        assert_eq!(
            extract_domain("https://example.com/path"),
            Some("example.com".to_string())
        );
        assert_eq!(
            extract_domain("http://sub.example.com:8080"),
            Some("sub.example.com".to_string())
        );
        assert_eq!(extract_domain("not-a-url"), None);
    }

    #[test]
    fn test_allowed_http_domains() {
        let mut state = HttpsOnlyState::new();
        assert!(!state.is_http_allowed("example.com"));
        state.allow_http("example.com");
        assert!(state.is_http_allowed("example.com"));
        assert!(!state.is_http_allowed("other.com"));
    }

    #[test]
    fn test_upgrade_counts() {
        let mut state = HttpsOnlyState::new();
        assert_eq!(state.get_upgrade_count("tab1"), 0);
        assert_eq!(state.record_upgrade("tab1"), 1);
        assert_eq!(state.record_upgrade("tab1"), 2);
        assert_eq!(state.get_upgrade_count("tab1"), 2);
        state.reset_tab("tab1");
        assert_eq!(state.get_upgrade_count("tab1"), 0);
    }

    #[test]
    fn test_pending_warnings() {
        let mut state = HttpsOnlyState::new();
        assert!(!state.has_pending_warning("tab1"));
        state.set_pending_warning("tab1", "http://example.com");
        assert!(state.has_pending_warning("tab1"));
        let url = state.take_pending_warning("tab1");
        assert_eq!(url, Some("http://example.com".to_string()));
        assert!(!state.has_pending_warning("tab1"));
    }
}
