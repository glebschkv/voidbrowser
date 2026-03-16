/// Determine if a request is third-party relative to the page.
///
/// Compares the registrable domain (last two segments of the host) of both URLs.
/// For example, `cdn.example.com` and `www.example.com` are same-party,
/// but `tracker.com` and `news.com` are third-party.
pub fn is_third_party(request_url: &str, page_url: &str) -> bool {
    let request_domain = registrable_domain(request_url);
    let page_domain = registrable_domain(page_url);

    match (request_domain, page_domain) {
        (Some(req), Some(page)) => req != page,
        // If we can't parse either URL, treat as same-party (don't block)
        _ => false,
    }
}

/// Extract the registrable domain (eTLD+1 approximation) from a URL.
///
/// For MVP, this compares the last two dot-separated segments of the host.
/// E.g., `sub.example.com` → `example.com`, `example.co.uk` → `co.uk` (imperfect
/// but sufficient for the common case).
fn registrable_domain(url: &str) -> Option<String> {
    let parsed: url::Url = url.parse().ok()?;
    let host = parsed.host_str()?;

    // IP addresses are their own registrable domain
    if host.parse::<std::net::IpAddr>().is_ok() {
        return Some(host.to_string());
    }

    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() >= 2 {
        Some(format!("{}.{}", parts[parts.len() - 2], parts[parts.len() - 1]))
    } else {
        Some(host.to_string())
    }
}

/// Generate a JavaScript content script that restricts `document.cookie` access
/// in cross-origin iframes, providing third-party cookie blocking at the JS level.
///
/// This script overrides `document.cookie` so that cross-origin iframes cannot
/// read or write cookies. Same-origin frames and the top-level page are unaffected.
pub fn generate_cookie_policy_script() -> String {
    r#"
    (function() {
        'use strict';

        // Only restrict cookie access in cross-origin iframes
        var isThirdParty = false;
        try {
            // If we can't access top.location.origin, we're in a cross-origin iframe
            if (window.top !== window && window.location.origin !== window.top.location.origin) {
                isThirdParty = true;
            }
        } catch (e) {
            // Cross-origin frame — SecurityError means we're third-party
            if (window.top !== window) {
                isThirdParty = true;
            }
        }

        if (!isThirdParty) return;

        // Override document.cookie for third-party contexts
        var cookieDesc = Object.getOwnPropertyDescriptor(Document.prototype, 'cookie') ||
                         Object.getOwnPropertyDescriptor(HTMLDocument.prototype, 'cookie');

        if (cookieDesc) {
            Object.defineProperty(document, 'cookie', {
                get: function() { return ''; },
                set: function() { /* silently ignore */ },
                configurable: false
            });
        }

        // Make toString look native
        var nativeToString = Function.prototype.toString;
        var overrides = new Set();

        function maskAsNative(fn, name) {
            overrides.add(fn);
            var original = nativeToString;
            if (!Function.prototype.toString.__void_patched) {
                Function.prototype.toString = function() {
                    if (overrides.has(this)) {
                        return 'function ' + (name || this.name || '') + '() { [native code] }';
                    }
                    return original.call(this);
                };
                Function.prototype.toString.__void_patched = true;
                overrides.add(Function.prototype.toString);
            }
        }

        if (cookieDesc) {
            var getter = Object.getOwnPropertyDescriptor(document, 'cookie').get;
            var setter = Object.getOwnPropertyDescriptor(document, 'cookie').set;
            if (getter) maskAsNative(getter, 'get cookie');
            if (setter) maskAsNative(setter, 'set cookie');
        }
    })();
    "#.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_same_site_not_third_party() {
        assert!(!is_third_party(
            "https://cdn.example.com/style.css",
            "https://www.example.com"
        ));
        assert!(!is_third_party(
            "https://example.com/api",
            "https://example.com"
        ));
    }

    #[test]
    fn test_cross_site_is_third_party() {
        assert!(is_third_party(
            "https://tracker.com/pixel",
            "https://news.com"
        ));
        assert!(is_third_party(
            "https://ads.doubleclick.net/ad.js",
            "https://example.com"
        ));
    }

    #[test]
    fn test_subdomain_same_party() {
        assert!(!is_third_party(
            "https://api.github.com/repos",
            "https://github.com"
        ));
    }

    #[test]
    fn test_ip_addresses() {
        assert!(!is_third_party(
            "http://192.168.1.1/api",
            "http://192.168.1.1/page"
        ));
        assert!(is_third_party(
            "http://192.168.1.2/api",
            "http://192.168.1.1/page"
        ));
    }

    #[test]
    fn test_invalid_urls_not_blocked() {
        assert!(!is_third_party("not-a-url", "https://example.com"));
        assert!(!is_third_party("https://example.com", "not-a-url"));
    }

    #[test]
    fn test_registrable_domain() {
        assert_eq!(
            registrable_domain("https://sub.example.com/path"),
            Some("example.com".to_string())
        );
        assert_eq!(
            registrable_domain("https://example.com"),
            Some("example.com".to_string())
        );
        assert_eq!(
            registrable_domain("http://localhost"),
            Some("localhost".to_string())
        );
    }
}
