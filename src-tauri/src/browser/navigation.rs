use url::Url;

const DUCKDUCKGO_SEARCH: &str = "https://duckduckgo.com/?q=%s";

/// Determine if the input looks like a URL or a search query, and return
/// the final URL to navigate to.  Uses DuckDuckGo as the default engine.
pub fn resolve_input(input: &str) -> String {
    resolve_input_with_engine(input, DUCKDUCKGO_SEARCH)
}

/// Like [`resolve_input`] but accepts a custom search URL template.
/// The template must contain `%s` which is replaced with the encoded query.
pub fn resolve_input_with_engine(input: &str, search_url_template: &str) -> String {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return "about:blank".to_string();
    }

    // If it already has a recognized scheme, use it directly
    if trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("about:")
        || trimmed.starts_with("data:")
        || trimmed.starts_with("void://")
    {
        return trimmed.to_string();
    }

    // If it looks like a URL (contains a dot and no spaces), prepend https://
    if looks_like_url(trimmed) {
        let with_scheme = format!("https://{trimmed}");
        if Url::parse(&with_scheme).is_ok() {
            return with_scheme;
        }
    }

    // Otherwise treat as a search query
    let encoded = urlencoding_encode(trimmed);
    search_url_template.replace("%s", &encoded)
}

/// Heuristic: input looks like a URL if it contains a dot,
/// has no spaces, and the part before the first dot is non-empty.
fn looks_like_url(input: &str) -> bool {
    if input.contains(' ') {
        return false;
    }

    if let Some(dot_pos) = input.find('.') {
        // Must have something before and after the dot
        dot_pos > 0 && dot_pos < input.len() - 1
    } else {
        // Also accept localhost, localhost:port, etc.
        input.starts_with("localhost")
    }
}

/// Simple percent-encoding for search queries.
fn urlencoding_encode(input: &str) -> String {
    let mut result = String::with_capacity(input.len() * 3);
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            b' ' => {
                result.push('+');
            }
            _ => {
                result.push('%');
                result.push_str(&format!("{byte:02X}"));
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_url_passthrough() {
        assert_eq!(
            resolve_input("https://example.com"),
            "https://example.com"
        );
        assert_eq!(
            resolve_input("http://example.com"),
            "http://example.com"
        );
    }

    #[test]
    fn test_bare_domain_gets_https() {
        assert_eq!(
            resolve_input("example.com"),
            "https://example.com"
        );
        assert_eq!(
            resolve_input("github.com/tauri-apps"),
            "https://github.com/tauri-apps"
        );
    }

    #[test]
    fn test_search_query() {
        let result = resolve_input("rust programming");
        assert!(result.starts_with("https://duckduckgo.com/?q="));
        assert!(result.contains("rust"));
        assert!(result.contains("programming"));
    }

    #[test]
    fn test_empty_input() {
        assert_eq!(resolve_input(""), "about:blank");
        assert_eq!(resolve_input("  "), "about:blank");
    }

    #[test]
    fn test_localhost() {
        assert_eq!(
            resolve_input("localhost:3000"),
            "https://localhost:3000"
        );
    }

    #[test]
    fn test_single_word_is_search() {
        let result = resolve_input("tauri");
        assert!(result.starts_with("https://duckduckgo.com/?q="));
    }

    #[test]
    fn test_custom_search_engine() {
        let result =
            resolve_input_with_engine("rust lang", "https://www.google.com/search?q=%s");
        assert!(result.starts_with("https://www.google.com/search?q="));
        assert!(result.contains("rust"));
    }

    #[test]
    fn test_void_scheme_passthrough() {
        assert_eq!(resolve_input("void://newtab"), "void://newtab");
        assert_eq!(resolve_input("void://privacy"), "void://privacy");
        assert_eq!(resolve_input("void://about"), "void://about");
    }

    #[test]
    fn test_custom_engine_url_passthrough() {
        let result =
            resolve_input_with_engine("https://example.com", "https://www.google.com/search?q=%s");
        assert_eq!(result, "https://example.com");
    }
}
