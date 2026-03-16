use std::sync::{Arc, Mutex};

use tauri::{
    webview::{PageLoadEvent, WebviewBuilder},
    window::Window,
    Emitter, LogicalPosition, LogicalSize, Manager, Runtime, Webview, WebviewUrl,
};

use super::tabs::TabManager;
#[cfg(target_os = "windows")]
use crate::privacy::ad_blocker::AdBlocker;
use crate::privacy::ad_blocker::ShieldState;
use crate::privacy::cookie_policy;
use crate::privacy::fingerprint::FingerprintShield;
use crate::privacy::https_only::{self, HttpsOnlyState};
use crate::storage::history::SessionHistory;

/// Height of the toolbar area in logical pixels (36px tab bar + 46px toolbar).
pub const TOOLBAR_HEIGHT: f64 = 82.0;

/// Build the webview label for a given tab id.
pub fn tab_webview_label(tab_id: &str) -> String {
    format!("tab-{tab_id}")
}

/// Derive a favicon URL from a page URL (uses /favicon.ico at the origin).
fn derive_favicon_url(page_url: &str) -> Option<String> {
    let parsed: url::Url = page_url.parse().ok()?;
    let origin = parsed.origin();
    match origin {
        url::Origin::Tuple(scheme, host, port) => {
            let default_port = matches!(
                (scheme.as_str(), port),
                ("http", 80) | ("https", 443)
            );
            if default_port {
                Some(format!("{scheme}://{host}/favicon.ico"))
            } else {
                Some(format!("{scheme}://{host}:{port}/favicon.ico"))
            }
        }
        _ => None,
    }
}

/// Create a webview for a tab, positioned below the toolbar area.
pub fn create_tab_webview<R: Runtime>(
    window: &Window<R>,
    tab_id: &str,
    url: &str,
    visible: bool,
) -> Result<Webview<R>, String> {
    let size = window.inner_size().map_err(|e| e.to_string())?;
    let scale = window.scale_factor().map_err(|e| e.to_string())?;

    let logical_width = size.width as f64 / scale;
    let logical_height = size.height as f64 / scale;

    let label = tab_webview_label(tab_id);
    let tab_id_for_nav = tab_id.to_string();
    let tab_id_for_title = tab_id.to_string();
    let tab_id_for_load = tab_id.to_string();
    let app_handle_for_nav = window.app_handle().clone();
    let app_handle_for_title = window.app_handle().clone();
    let app_handle_for_load = window.app_handle().clone();
    let app_handle_for_history_nav = window.app_handle().clone();
    let app_handle_for_history_title = window.app_handle().clone();

    let webview_url = WebviewUrl::External(
        url.parse()
            .map_err(|e: url::ParseError| e.to_string())?,
    );

    let mut builder = WebviewBuilder::new(&label, webview_url)
        .auto_resize()
        .incognito(true);

    // Inject fingerprint resistance script into ALL webviews (runs before any page JS)
    let fp_shield = window.app_handle().state::<FingerprintShield>();
    builder = builder.initialization_script(fp_shield.get_injection_script());

    // Inject cookie policy script to block third-party cookie access in iframes
    let cookie_script = cookie_policy::generate_cookie_policy_script();
    builder = builder.initialization_script(&cookie_script);

    // Inject keyboard shortcut capture script so shortcuts work when
    // focus is in the content webview (not the toolbar).
    builder = builder.initialization_script(KEYBOARD_SHORTCUT_SCRIPT);

    // Clone label for use in on_navigation closure to look up the webview
    let label_for_nav = label.clone();

    let builder = builder
        .on_navigation(move |nav_url| {
            let url_str = nav_url.to_string();

            // Allow void:// internal pages through without any checks.
            // On Windows, WebView2 rewrites custom protocols to
            // http://void.newtab/ etc., so match that pattern too.
            if url_str.starts_with("void://") || url_str.starts_with("http://void.") {
                return true;
            }

            // ── HTTPS-Only Mode ──────────────────────────────────────
            // Block HTTP navigations and redirect to HTTPS, unless the
            // user has explicitly allowed HTTP for this domain or the
            // shield is disabled for the site.
            if url_str.starts_with("http://") {
                let domain = https_only::extract_domain(&url_str)
                    .unwrap_or_default();

                let https_state =
                    app_handle_for_nav.state::<Arc<Mutex<HttpsOnlyState>>>();
                let shield_state =
                    app_handle_for_nav.state::<Arc<Mutex<ShieldState>>>();

                // Check if HTTP is allowed for this domain
                let http_allowed = match https_state.lock() {
                    Ok(s) => s.is_http_allowed(&domain),
                    Err(e) => e.into_inner().is_http_allowed(&domain),
                };

                // Check if shield is disabled for this site
                let site_disabled = match shield_state.lock() {
                    Ok(s) => s.is_site_disabled(&domain),
                    Err(e) => e.into_inner().is_site_disabled(&domain),
                };

                if !http_allowed && !site_disabled {
                    // Block HTTP navigation — navigate to about:blank and
                    // set a pending warning so on_page_load injects the
                    // warning page after about:blank finishes loading.
                    if let Ok(mut hs) = https_state.lock() {
                        hs.set_pending_warning(&tab_id_for_nav, &url_str);
                        hs.record_upgrade(&tab_id_for_nav);
                    }

                    // Navigate to about:blank; the warning will be injected
                    // in the on_page_load handler once it finishes loading.
                    if let Some(wv) =
                        app_handle_for_nav.get_webview(&label_for_nav)
                    {
                        let blank_url: url::Url =
                            "about:blank".parse().expect("valid URL");
                        let _ = wv.navigate(blank_url);
                    }

                    return false;
                }
            }

            // ── Normal navigation handling ────────────────────────────
            let favicon = derive_favicon_url(&url_str);

            // Update TabManager state
            let tab_mgr = app_handle_for_nav.state::<Arc<Mutex<TabManager>>>();
            let tab_info = if let Ok(mut mgr) = tab_mgr.lock() {
                if let Some(tab) = mgr.get_tab_mut(&tab_id_for_nav) {
                    tab.url = url_str.clone();
                    tab.favicon_url = favicon;
                    Some(tab.to_info())
                } else {
                    None
                }
            } else {
                None
            };

            // Reset blocked count on navigation to a new page
            let shield_state = app_handle_for_nav.state::<Arc<Mutex<ShieldState>>>();
            if let Ok(mut shield) = shield_state.lock() {
                shield.reset(&tab_id_for_nav);
            }
            let _ = app_handle_for_nav.emit(
                "blocked-count-updated",
                serde_json::json!({
                    "tabId": tab_id_for_nav,
                    "count": 0
                }),
            );

            let _ = app_handle_for_nav.emit(
                "tab-url-changed",
                serde_json::json!({
                    "tabId": tab_id_for_nav,
                    "url": url_str
                }),
            );

            if let Some(info) = tab_info {
                let _ = app_handle_for_nav.emit("tab-updated", &info);
            }

            // Record navigation in session history
            let history_arc = {
                let s = app_handle_for_history_nav.state::<Arc<Mutex<SessionHistory>>>();
                Arc::clone(&*s)
            };
            let mut guard = match history_arc.lock() {
                Ok(g) => g,
                Err(e) => e.into_inner(),
            };
            guard.add_entry(&url_str, "");
            drop(guard);

            true
        })
        .on_document_title_changed(move |_webview, title| {
            // Update TabManager state
            let tab_mgr = app_handle_for_title.state::<Arc<Mutex<TabManager>>>();
            let (tab_info, tab_url, is_active) = if let Ok(mut mgr) = tab_mgr.lock() {
                let active = mgr.active_tab_id.as_deref() == Some(&tab_id_for_title);
                if let Some(tab) = mgr.get_tab_mut(&tab_id_for_title) {
                    tab.title = title.clone();
                    (Some(tab.to_info()), Some(tab.url.clone()), active)
                } else {
                    (None, None, false)
                }
            } else {
                (None, None, false)
            };

            if let Some(info) = tab_info {
                let _ = app_handle_for_title.emit("tab-updated", &info);
            }

            // Update window title if this is the active tab
            if is_active {
                if let Some(window) = app_handle_for_title.get_window("main") {
                    let window_title = if title.is_empty() {
                        "VoidBrowser".to_string()
                    } else {
                        format!("{title} \u{2014} VoidBrowser")
                    };
                    let _ = window.set_title(&window_title);
                }
            }

            // Update session history entry title
            if let Some(url) = tab_url {
                let history_arc = {
                    let s = app_handle_for_history_title.state::<Arc<Mutex<SessionHistory>>>();
                    Arc::clone(&*s)
                };
                let mut guard = match history_arc.lock() {
                    Ok(g) => g,
                    Err(e) => e.into_inner(),
                };
                guard.update_title(&url, &title);
                drop(guard);
            }
        })
        .on_page_load(move |webview, payload| {
            let is_loading = matches!(payload.event(), PageLoadEvent::Started);

            // Update loading state in TabManager and emit tab-updated event
            let tab_mgr = app_handle_for_load.state::<Arc<Mutex<TabManager>>>();
            if let Ok(mut mgr) = tab_mgr.lock() {
                if let Some(tab) = mgr.get_tab_mut(&tab_id_for_load) {
                    tab.is_loading = is_loading;
                    let info = tab.to_info();
                    let _ = app_handle_for_load.emit("tab-updated", &info);
                }
            }

            // Inject HTTPS warning page after about:blank finishes loading
            if matches!(payload.event(), PageLoadEvent::Finished) {
                let https_state =
                    app_handle_for_load.state::<Arc<Mutex<HttpsOnlyState>>>();
                let pending_url = match https_state.lock() {
                    Ok(mut s) => s.take_pending_warning(&tab_id_for_load),
                    Err(e) => e.into_inner().take_pending_warning(&tab_id_for_load),
                };

                if let Some(original_http_url) = pending_url {
                    let warning_script =
                        https_only::generate_https_warning_page(
                            &original_http_url,
                            &tab_id_for_load,
                        );
                    let _ = webview.eval(&warning_script);
                }
            }
        });

    let webview = window
        .add_child(
            builder,
            LogicalPosition::new(0.0, TOOLBAR_HEIGHT),
            LogicalSize::new(logical_width, logical_height - TOOLBAR_HEIGHT),
        )
        .map_err(|e| e.to_string())?;

    // Hide the webview if it shouldn't be visible initially
    if !visible {
        let _ = webview.hide();
    }

    // Set up native WebView2 request interception for ad blocking (Windows only)
    #[cfg(target_os = "windows")]
    {
        let tab_id_owned = tab_id.to_string();
        let app_handle = window.app_handle().clone();
        if let Err(e) = setup_request_interception(&webview, tab_id_owned, app_handle) {
            eprintln!("Failed to set up request interception: {e}");
        }
    }

    Ok(webview)
}

/// Safely convert a PWSTR to a String, returning empty string on null or error.
///
/// # Safety
/// The caller must ensure that if `pwstr` is non-null, it points to a valid
/// null-terminated UTF-16 string.
#[cfg(target_os = "windows")]
unsafe fn pwstr_to_string_safe(pwstr: windows::core::PWSTR) -> String {
    if pwstr.is_null() {
        return String::new();
    }
    pwstr.to_string().unwrap_or_default()
}

/// Set up WebView2 native request interception via the COM API.
///
/// This hooks into every HTTP/HTTPS request the webview makes and checks it
/// against the adblock engine. Blocked requests receive an empty 204 response.
/// Third-party requests also have their Cookie headers stripped.
///
/// The entire callback body is wrapped in `catch_unwind` so that a panic in any
/// step lets the request through instead of crashing the webview process.
#[cfg(target_os = "windows")]
fn setup_request_interception<R: Runtime>(
    webview: &Webview<R>,
    tab_id: String,
    app_handle: tauri::AppHandle<R>,
) -> Result<(), String> {
    use std::panic::{catch_unwind, AssertUnwindSafe};

    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2, ICoreWebView2_2, ICoreWebView2Environment,
        ICoreWebView2WebResourceRequestedEventArgs,
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
    };
    use windows::core::{Interface, HSTRING, PWSTR};
    use webview2_com::WebResourceRequestedEventHandler;
    type EventRegistrationToken = i64;

    eprintln!("[AdBlock] Setting up request interception for tab {tab_id}");

    webview
        .with_webview(move |wv| {
            // SAFETY: We access the WebView2 COM interface through the controller
            // provided by wry. The controller lifetime is tied to the webview.
            let result = catch_unwind(AssertUnwindSafe(|| unsafe {
                let core: ICoreWebView2 = match wv.controller().CoreWebView2() {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("[AdBlock] Failed to get CoreWebView2: {e}");
                        return;
                    }
                };

                // Cast to ICoreWebView2_2 which exposes the Environment() method
                let core2: ICoreWebView2_2 = match core.cast::<ICoreWebView2_2>() {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("[AdBlock] Failed to cast to ICoreWebView2_2: {e}");
                        return;
                    }
                };

                // Get the environment for creating responses
                let env: ICoreWebView2Environment = match core2.Environment() {
                    Ok(e) => e,
                    Err(e) => {
                        eprintln!("[AdBlock] Failed to get environment: {e}");
                        return;
                    }
                };

                // Register filter to intercept all HTTP/HTTPS requests
                if let Err(e) = core.AddWebResourceRequestedFilter(
                    &HSTRING::from("*"),
                    COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
                ) {
                    eprintln!("[AdBlock] Failed to add resource filter: {e}");
                    return;
                }

                let blocker = app_handle.state::<Arc<Mutex<AdBlocker>>>();
                let shield = app_handle.state::<Arc<Mutex<ShieldState>>>();
                let blocker = Arc::clone(&*blocker);
                let shield = Arc::clone(&*shield);
                let app_for_emit = app_handle.clone();

                let tab_id_for_log = tab_id.clone();
                let handler = WebResourceRequestedEventHandler::create(Box::new(
                    move |sender: Option<ICoreWebView2>,
                          args: Option<
                        ICoreWebView2WebResourceRequestedEventArgs,
                    >| {
                        // Wrap the ENTIRE handler body in catch_unwind so that
                        // any panic lets the request through instead of crashing
                        // the webview across FFI.
                        let result = catch_unwind(AssertUnwindSafe(|| {
                            let args = match args {
                                Some(a) => a,
                                None => return,
                            };

                            // Get the request URL — if this fails, let the request through
                            let request = match args.Request() {
                                Ok(r) => r,
                                Err(e) => {
                                    eprintln!("[AdBlock] Failed to get request: {e}");
                                    return;
                                }
                            };
                            let url_str = {
                                let mut uri = PWSTR::null();
                                match request.Uri(&mut uri) {
                                    Ok(()) => pwstr_to_string_safe(uri),
                                    Err(e) => {
                                        eprintln!("[AdBlock] Failed to get URI: {e}");
                                        return;
                                    }
                                }
                            };

                            // Skip non-HTTP requests and data URIs
                            if !url_str.starts_with("http://")
                                && !url_str.starts_with("https://")
                            {
                                return;
                            }

                            // Get the page URL from the sender webview as source_url
                            let source_url = if let Some(ref sender) = sender {
                                let mut url = PWSTR::null();
                                if sender.Source(&mut url).is_ok() {
                                    pwstr_to_string_safe(url)
                                } else {
                                    String::new()
                                }
                            } else {
                                String::new()
                            };

                            // Map WebView2 resource context to adblock resource type string
                            let resource_type = {
                                let mut ctx = COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL;
                                if args.ResourceContext(&mut ctx).is_ok() {
                                    map_resource_context(ctx)
                                } else {
                                    "other"
                                }
                            };

                            // Check the shield state — recover from poisoned mutex
                            // Check both per-tab and per-site disabled state
                            let domain = crate::privacy::https_only::extract_domain(&url_str)
                                .unwrap_or_default();
                            let is_disabled = match shield.lock() {
                                Ok(s) => {
                                    s.is_disabled(&tab_id)
                                        || s.is_site_disabled(&domain)
                                }
                                Err(e) => {
                                    eprintln!(
                                        "[AdBlock] Shield lock poisoned, recovering: {e}"
                                    );
                                    let s = e.into_inner();
                                    s.is_disabled(&tab_id)
                                        || s.is_site_disabled(&domain)
                                }
                            };

                            if is_disabled {
                                return;
                            }

                            // Strip Cookie header from third-party requests
                            if !source_url.is_empty()
                                && cookie_policy::is_third_party(&url_str, &source_url)
                            {
                                if let Ok(headers) = request.Headers() {
                                    let _ = headers.RemoveHeader(&HSTRING::from("Cookie"));
                                }
                            }

                            // Check the adblock engine — recover from poisoned mutex
                            let should_block = match blocker.lock() {
                                Ok(b) => {
                                    b.should_block(&url_str, &source_url, resource_type)
                                }
                                Err(e) => {
                                    eprintln!(
                                        "[AdBlock] Blocker lock poisoned, recovering: {e}"
                                    );
                                    e.into_inner()
                                        .should_block(&url_str, &source_url, resource_type)
                                }
                            };

                            // Never block the main document — only block sub-resources
                            if resource_type == "document" || !should_block {
                                return;
                            }

                            // Create an empty 204 No Content response to block the request.
                            // If anything fails here, let the request through.
                            let response = match env.CreateWebResourceResponse(
                                None, // no content stream
                                204,
                                &HSTRING::from("No Content"),
                                &HSTRING::from(""),
                            ) {
                                Ok(r) => r,
                                Err(e) => {
                                    eprintln!(
                                        "[AdBlock] Failed to create blocked response: {e}"
                                    );
                                    return;
                                }
                            };

                            if let Err(e) = args.SetResponse(&response) {
                                eprintln!("[AdBlock] Failed to set response: {e}");
                                return;
                            }

                            // Increment blocked count and emit event to frontend
                            let count = match shield.lock() {
                                Ok(mut s) => s.increment(&tab_id, &url_str),
                                Err(e) => e.into_inner().increment(&tab_id, &url_str),
                            };
                            let _ = app_for_emit.emit(
                                "blocked-count-updated",
                                serde_json::json!({
                                    "tabId": tab_id,
                                    "count": count
                                }),
                            );
                        }));

                        if result.is_err() {
                            eprintln!("[AdBlock] Handler panicked — letting request through");
                        }
                        // Always return Ok to COM so the webview stays alive
                        Ok(())
                    },
                ));

                let mut token = EventRegistrationToken::default();
                if let Err(e) = core.add_WebResourceRequested(&handler, &mut token) {
                    eprintln!("[AdBlock] Failed to register handler: {e}");
                    return;
                }

                eprintln!("[AdBlock] Request interception active for tab {tab_id_for_log}");
            }));

            if result.is_err() {
                eprintln!("[AdBlock] Request interception setup panicked");
            }
        })
        .map_err(|e| e.to_string())
}

/// Map a WebView2 resource context enum to an adblock-compatible resource type string.
#[cfg(target_os = "windows")]
fn map_resource_context(
    ctx: webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_WEB_RESOURCE_CONTEXT,
) -> &'static str {
    use webview2_com::Microsoft::Web::WebView2::Win32::*;

    match ctx {
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_DOCUMENT => "document",
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_STYLESHEET => "stylesheet",
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_IMAGE => "image",
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_MEDIA => "media",
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_FONT => "font",
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_SCRIPT => "script",
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_XML_HTTP_REQUEST => "xmlhttprequest",
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_FETCH => "xmlhttprequest",
        _ => "other",
    }
}

/// JavaScript that captures keyboard shortcuts in content webviews and
/// forwards them to the Rust backend via Tauri IPC.
const KEYBOARD_SHORTCUT_SCRIPT: &str = r#"
(function() {
    document.addEventListener('keydown', function(e) {
        var ctrl = e.ctrlKey || e.metaKey;
        if (!ctrl) return;

        var key = null;
        switch (e.key) {
            case 'b': key = 'toggle_sidebar'; break;
            case 'd': key = 'bookmark_page'; break;
            case ',': key = 'open_settings'; break;
            case 't': key = 'new_tab'; break;
            case 'w': key = 'close_tab'; break;
            case 'l': key = 'focus_address_bar'; break;
            case 'f': key = 'find_in_page'; break;
            case '=': case '+': key = 'zoom_in'; break;
            case '-': key = 'zoom_out'; break;
            case '0': key = 'zoom_reset'; break;
            default: return;
        }

        e.preventDefault();
        e.stopPropagation();

        try {
            if (window.__TAURI_INTERNALS__) {
                window.__TAURI_INTERNALS__.invoke('handle_keyboard_shortcut', { key: key });
            }
        } catch (_) {}
    }, true);
})();
"#;

/// Return the full HTML for the new tab page, served via the `void://` protocol.
pub fn new_tab_page_html() -> String {
    r#"<!DOCTYPE html>\
<html>\
<head>\
    <meta charset="utf-8">\
    <title>New Tab</title>\
    <style>\
        * { margin: 0; padding: 0; box-sizing: border-box; }\
        body {\
            background: #171717;\
            color: #f5f5f5;\
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;\
            display: flex;\
            flex-direction: column;\
            align-items: center;\
            justify-content: center;\
            min-height: 100vh;\
            user-select: none;\
            padding: 2rem;\
        }\
        h1 {\
            font-size: 2.5rem;\
            font-weight: 300;\
            margin-bottom: 2rem;\
            color: #e5e5e5;\
            letter-spacing: 0.05em;\
        }\
        h1 span { color: #6366f1; }\
        .search-container {\
            width: 100%;\
            max-width: 580px;\
            position: relative;\
        }\
        input {\
            width: 100%;\
            padding: 14px 20px;\
            background: #262626;\
            border: 1px solid #404040;\
            border-radius: 8px;\
            color: #f5f5f5;\
            font-size: 1rem;\
            outline: none;\
            transition: border-color 0.2s;\
        }\
        input:focus { border-color: #6366f1; }\
        input::placeholder { color: #737373; }\
        .bookmarks-grid {\
            display: grid;\
            grid-template-columns: repeat(4, 1fr);\
            gap: 12px;\
            max-width: 580px;\
            width: 100%;\
            margin-top: 2rem;\
        }\
        .bookmark-tile {\
            display: flex;\
            flex-direction: column;\
            align-items: center;\
            gap: 8px;\
            padding: 16px 8px;\
            background: #262626;\
            border-radius: 8px;\
            cursor: pointer;\
            transition: background 0.2s;\
            text-decoration: none;\
            color: #d4d4d4;\
            min-height: 80px;\
        }\
        .bookmark-tile:hover { background: #404040; }\
        .bookmark-favicon {\
            width: 24px;\
            height: 24px;\
            border-radius: 4px;\
            background: #404040;\
            display: flex;\
            align-items: center;\
            justify-content: center;\
            font-size: 14px;\
            color: #a3a3a3;\
            overflow: hidden;\
        }\
        .bookmark-favicon img { width: 100%; height: 100%; object-fit: cover; }\
        .bookmark-title {\
            font-size: 0.75rem;\
            text-align: center;\
            overflow: hidden;\
            text-overflow: ellipsis;\
            white-space: nowrap;\
            max-width: 100%;\
        }\
        .stats-footer {\
            margin-top: 3rem;\
            color: #525252;\
            font-size: 0.85rem;\
        }\
    </style>\
</head>\
<body>\
    <h1>Void<span>Browser</span></h1>\
    <div class="search-container">\
        <input type="text" placeholder="Search the web or enter a URL" autofocus id="searchInput" />\
    </div>\
    <div id="bookmarksGrid" class="bookmarks-grid"></div>\
    <p id="statsFooter" class="stats-footer">Your browser. Your data. Nobody else&apos;s.</p>\
    <script>\
        document.getElementById("searchInput").addEventListener("keydown", function(e) {\
            if (e.key === "Enter" && this.value.trim()) {\
                var val = this.value.trim();\
                if (val.includes(".") && !val.includes(" ")) {\
                    if (val.startsWith("http://") || val.startsWith("https://")) {\
                        window.location.href = val;\
                    } else {\
                        window.location.href = "https://" + val;\
                    }\
                } else {\
                    window.location.href = "https://duckduckgo.com/?q=" + encodeURIComponent(val);\
                }\
            }\
        });\
        /* Load bookmarks */\
        (function() {\
            if (!window.__TAURI_INTERNALS__) return;\
            try {\
                window.__TAURI_INTERNALS__.invoke("get_bookmarks", { folder: null }).then(function(bookmarks) {\
                    var grid = document.getElementById("bookmarksGrid");\
                    if (!grid || !bookmarks || bookmarks.length === 0) return;\
                    var top8 = bookmarks.slice(0, 8);\
                    top8.forEach(function(bm) {\
                        var a = document.createElement("a");\
                        a.className = "bookmark-tile";\
                        a.href = bm.url;\
                        var fav = document.createElement("div");\
                        fav.className = "bookmark-favicon";\
                        try {\
                            var u = new URL(bm.url);\
                            var img = document.createElement("img");\
                            img.src = u.origin + "/favicon.ico";\
                            img.onerror = function() { this.parentNode.textContent = bm.title.charAt(0).toUpperCase(); };\
                            fav.appendChild(img);\
                        } catch(e) { fav.textContent = bm.title.charAt(0).toUpperCase(); }\
                        var title = document.createElement("span");\
                        title.className = "bookmark-title";\
                        title.textContent = bm.title;\
                        a.appendChild(fav);\
                        a.appendChild(title);\
                        grid.appendChild(a);\
                    });\
                }).catch(function() {});\
                window.__TAURI_INTERNALS__.invoke("get_privacy_stats").then(function(stats) {\
                    var footer = document.getElementById("statsFooter");\
                    if (!footer || !stats) return;\
                    if (stats.totalBlocked > 0) {\
                        footer.textContent = stats.totalBlocked + " trackers blocked this session";\
                    }\
                }).catch(function() {});\
            } catch(e) {}\
        })();\
    </script>\
</body>\
</html>"#.to_string()
}

/// Return the full HTML for the privacy dashboard page.
pub fn privacy_dashboard_html() -> String {
    r#"<!DOCTYPE html>\
<html>\
<head>\
    <meta charset="utf-8">\
    <title>Privacy Dashboard — VoidBrowser</title>\
    <style>\
        * { margin: 0; padding: 0; box-sizing: border-box; }\
        body {\
            background: #171717;\
            color: #f5f5f5;\
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;\
            padding: 3rem 2rem;\
            user-select: none;\
        }\
        .header {\
            text-align: center;\
            margin-bottom: 3rem;\
        }\
        h1 {\
            font-size: 1.75rem;\
            font-weight: 300;\
            color: #e5e5e5;\
            letter-spacing: 0.05em;\
        }\
        h1 span { color: #6366f1; }\
        .subtitle {\
            color: #737373;\
            font-size: 0.9rem;\
            margin-top: 0.5rem;\
        }\
        .stats-grid {\
            display: grid;\
            grid-template-columns: repeat(3, 1fr);\
            gap: 1.5rem;\
            max-width: 700px;\
            margin: 0 auto 3rem;\
        }\
        .stat-card {\
            background: #262626;\
            border-radius: 12px;\
            padding: 2rem 1.5rem;\
            text-align: center;\
        }\
        .stat-number {\
            font-size: 3rem;\
            font-weight: 700;\
            color: #6366f1;\
            line-height: 1;\
            margin-bottom: 0.5rem;\
        }\
        .stat-label {\
            color: #a3a3a3;\
            font-size: 0.85rem;\
            text-transform: uppercase;\
            letter-spacing: 0.05em;\
        }\
        .domains-section {\
            max-width: 700px;\
            margin: 0 auto;\
        }\
        .domains-section h2 {\
            font-size: 1.1rem;\
            font-weight: 500;\
            color: #d4d4d4;\
            margin-bottom: 1rem;\
        }\
        .domain-row {\
            display: flex;\
            justify-content: space-between;\
            align-items: center;\
            padding: 0.75rem 1rem;\
            background: #262626;\
            border-radius: 6px;\
            margin-bottom: 4px;\
        }\
        .domain-name {\
            font-family: monospace;\
            font-size: 0.85rem;\
            color: #d4d4d4;\
        }\
        .domain-count {\
            font-size: 0.85rem;\
            color: #6366f1;\
            font-weight: 600;\
        }\
        .empty-state {\
            text-align: center;\
            color: #525252;\
            font-size: 0.9rem;\
            padding: 2rem;\
        }\
    </style>\
</head>\
<body>\
    <div class="header">\
        <h1>Privacy <span>Dashboard</span></h1>\
        <p class="subtitle">Session stats — resets when you close the browser</p>\
    </div>\
    <div class="stats-grid">\
        <div class="stat-card">\
            <div class="stat-number" id="blocked">0</div>\
            <div class="stat-label">Trackers Blocked</div>\
        </div>\
        <div class="stat-card">\
            <div class="stat-number" id="ads">0</div>\
            <div class="stat-label">Ads Blocked</div>\
        </div>\
        <div class="stat-card">\
            <div class="stat-number" id="upgrades">0</div>\
            <div class="stat-label">HTTPS Upgrades</div>\
        </div>\
    </div>\
    <div class="domains-section">\
        <h2>Top Blocked Domains</h2>\
        <div id="domainList"><p class="empty-state">No blocked domains yet</p></div>\
    </div>\
    <script>\
        (function() {\
            if (!window.__TAURI_INTERNALS__) return;\
            try {\
                window.__TAURI_INTERNALS__.invoke("get_privacy_stats").then(function(stats) {\
                    if (!stats) return;\
                    document.getElementById("blocked").textContent = stats.totalBlocked;\
                    document.getElementById("ads").textContent = stats.totalBlocked;\
                    document.getElementById("upgrades").textContent = stats.totalUpgrades;\
                    var list = document.getElementById("domainList");\
                    if (stats.topBlockedDomains && stats.topBlockedDomains.length > 0) {\
                        list.innerHTML = "";\
                        stats.topBlockedDomains.forEach(function(entry) {\
                            var row = document.createElement("div");\
                            row.className = "domain-row";\
                            var name = document.createElement("span");\
                            name.className = "domain-name";\
                            name.textContent = entry[0];\
                            var count = document.createElement("span");\
                            count.className = "domain-count";\
                            count.textContent = entry[1];\
                            row.appendChild(name);\
                            row.appendChild(count);\
                            list.appendChild(row);\
                        });\
                    }\
                }).catch(function() {});\
            } catch(e) {}\
        })();\
    </script>\
</body>\
</html>"#.to_string()
}

/// Return the full HTML for the about page.
pub fn about_page_html() -> String {
    r#"<!DOCTYPE html>\
<html>\
<head>\
    <meta charset="utf-8">\
    <title>About — VoidBrowser</title>\
    <style>\
        * { margin: 0; padding: 0; box-sizing: border-box; }\
        body {\
            background: #171717;\
            color: #f5f5f5;\
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;\
            display: flex;\
            flex-direction: column;\
            align-items: center;\
            justify-content: center;\
            min-height: 100vh;\
            user-select: none;\
            padding: 2rem;\
        }\
        h1 {\
            font-size: 2.5rem;\
            font-weight: 300;\
            color: #e5e5e5;\
            letter-spacing: 0.05em;\
            margin-bottom: 0.5rem;\
        }\
        h1 span { color: #6366f1; }\
        .version {\
            color: #737373;\
            font-size: 0.9rem;\
            margin-bottom: 2.5rem;\
        }\
        .info-card {\
            background: #262626;\
            border-radius: 12px;\
            padding: 2rem;\
            max-width: 420px;\
            width: 100%;\
        }\
        .info-row {\
            display: flex;\
            justify-content: space-between;\
            padding: 0.6rem 0;\
            border-bottom: 1px solid #404040;\
            font-size: 0.85rem;\
        }\
        .info-row:last-child { border-bottom: none; }\
        .info-label { color: #a3a3a3; }\
        .info-value { color: #d4d4d4; }\
        .privacy-pledge {\
            margin-top: 2.5rem;\
            text-align: center;\
            max-width: 420px;\
        }\
        .privacy-pledge p {\
            color: #6366f1;\
            font-size: 1.1rem;\
            font-weight: 500;\
            margin-bottom: 0.5rem;\
        }\
        .privacy-pledge span {\
            color: #525252;\
            font-size: 0.8rem;\
        }\
    </style>\
</head>\
<body>\
    <h1>Void<span>Browser</span></h1>\
    <p class="version">Version 0.1.0</p>\
    <div class="info-card">\
        <div class="info-row">\
            <span class="info-label">Built with</span>\
            <span class="info-value">Tauri + Rust + SolidJS</span>\
        </div>\
        <div class="info-row">\
            <span class="info-label">License</span>\
            <span class="info-value">MPL-2.0</span>\
        </div>\
        <div class="info-row">\
            <span class="info-label">Platform</span>\
            <span class="info-value">Windows 10/11</span>\
        </div>\
        <div class="info-row">\
            <span class="info-label">Engine</span>\
            <span class="info-value">WebView2 (Chromium)</span>\
        </div>\
    </div>\
    <div class="privacy-pledge">\
        <p>We collect nothing. We never will.</p>\
        <span>No telemetry. No analytics. No accounts. No cloud.</span>\
    </div>\
</body>\
</html>"#.to_string()
}
