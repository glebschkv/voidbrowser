use std::sync::{Arc, Mutex};

use tauri::{
    webview::WebviewBuilder, window::Window, Emitter, LogicalPosition, LogicalSize, Manager,
    Runtime, Webview, WebviewUrl,
};

use super::tabs::TabManager;

/// Height of the toolbar area in logical pixels (36px tab bar + 46px toolbar).
pub const TOOLBAR_HEIGHT: f64 = 82.0;

/// Build the webview label for a given tab id.
pub fn tab_webview_label(tab_id: &str) -> String {
    format!("tab-{tab_id}")
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
    let app_handle_for_nav = window.app_handle().clone();
    let app_handle_for_title = window.app_handle().clone();

    let is_new_tab = url == "void://newtab";

    let webview_url = if is_new_tab {
        WebviewUrl::default()
    } else {
        WebviewUrl::External(
            url.parse()
                .map_err(|e: url::ParseError| e.to_string())?,
        )
    };

    let builder = WebviewBuilder::new(&label, webview_url)
        .auto_resize()
        .on_navigation(move |nav_url| {
            let url_str = nav_url.to_string();

            // Update TabManager state
            let tab_mgr = app_handle_for_nav.state::<Arc<Mutex<TabManager>>>();
            if let Ok(mut mgr) = tab_mgr.lock() {
                if let Some(tab) = mgr.get_tab_mut(&tab_id_for_nav) {
                    tab.url = url_str.clone();
                }
            }

            let _ = app_handle_for_nav.emit(
                "tab-url-changed",
                serde_json::json!({
                    "tabId": tab_id_for_nav,
                    "url": url_str
                }),
            );
            true
        })
        .on_document_title_changed(move |_webview, title| {
            // Update TabManager state
            let tab_mgr = app_handle_for_title.state::<Arc<Mutex<TabManager>>>();
            let tab_info = if let Ok(mut mgr) = tab_mgr.lock() {
                if let Some(tab) = mgr.get_tab_mut(&tab_id_for_title) {
                    tab.title = title.clone();
                    Some(tab.to_info())
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(info) = tab_info {
                let _ = app_handle_for_title.emit("tab-updated", &info);
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

    // For new tab page, inject the HTML content
    if is_new_tab {
        let new_tab_html = generate_new_tab_page_script();
        let _ = webview.eval(&new_tab_html);
    }

    Ok(webview)
}

/// Generate JavaScript that replaces the page with our new tab content.
fn generate_new_tab_page_script() -> String {
    r#"
    document.open();
    document.write(`<!DOCTYPE html>
    <html>
    <head>
        <meta charset="utf-8">
        <title>New Tab</title>
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
            }
            h1 {
                font-size: 2.5rem;
                font-weight: 300;
                margin-bottom: 2rem;
                color: #e5e5e5;
                letter-spacing: 0.05em;
            }
            h1 span {
                color: #6366f1;
            }
            .search-container {
                width: 100%;
                max-width: 580px;
                position: relative;
            }
            input {
                width: 100%;
                padding: 14px 20px;
                background: #262626;
                border: 1px solid #404040;
                border-radius: 8px;
                color: #f5f5f5;
                font-size: 1rem;
                outline: none;
                transition: border-color 0.2s;
            }
            input:focus {
                border-color: #6366f1;
            }
            input::placeholder {
                color: #737373;
            }
            .tagline {
                margin-top: 3rem;
                color: #525252;
                font-size: 0.85rem;
            }
        </style>
    </head>
    <body>
        <h1>Void<span>Browser</span></h1>
        <div class="search-container">
            <input
                type="text"
                placeholder="Search the web or enter a URL"
                autofocus
                id="searchInput"
            />
        </div>
        <p class="tagline">Your browser. Your data. Nobody else's.</p>
        <script>
            document.getElementById('searchInput').addEventListener('keydown', function(e) {
                if (e.key === 'Enter' && this.value.trim()) {
                    const val = this.value.trim();
                    if (val.includes('.') && !val.includes(' ')) {
                        if (val.startsWith('http://') || val.startsWith('https://')) {
                            window.location.href = val;
                        } else {
                            window.location.href = 'https://' + val;
                        }
                    } else {
                        window.location.href = 'https://duckduckgo.com/?q=' + encodeURIComponent(val);
                    }
                }
            });
        </script>
    </body>
    </html>`);
    document.close();
    "#.to_string()
}
