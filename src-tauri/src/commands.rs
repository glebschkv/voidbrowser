use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::browser::navigation;
use crate::browser::tabs::{Tab, TabInfo, TabManager};
use crate::browser::webview;
use crate::privacy::ad_blocker::ShieldState;
use crate::privacy::https_only::{self, HttpsOnlyState};
use crate::storage::bookmarks::{self, Bookmark};
use crate::storage::database::Database;
use crate::storage::history::{HistoryEntry, SessionHistory};
use crate::storage::settings;

/// Helper: get the webview label for a tab id.
fn webview_label(tab_id: &str) -> String {
    webview::tab_webview_label(tab_id)
}

/// Helper: get the active tab id from the manager, or return an error.
fn active_tab_id(mgr: &TabManager) -> Result<String, String> {
    mgr.active_tab_id
        .clone()
        .ok_or_else(|| "No active tab".to_string())
}

// ── Tab management commands ──────────────────────────────────────────

#[tauri::command]
pub async fn create_tab<R: Runtime>(
    app: AppHandle<R>,
    url: Option<String>,
) -> Result<TabInfo, String> {
    let nav_url = match &url {
        Some(u) if !u.is_empty() => navigation::resolve_input(u),
        _ => "void://newtab".to_string(),
    };

    let tab = Tab::new(nav_url.clone());
    let tab_id = tab.id.clone();

    let tab_mgr = app.state::<Arc<Mutex<TabManager>>>();
    let info = {
        let mut mgr = tab_mgr.lock().map_err(|e| e.to_string())?;
        let info = mgr.add_tab(tab);

        // Hide the currently active webview
        if let Some(prev_id) = &mgr.active_tab_id {
            let prev_label = webview_label(prev_id);
            if let Some(wv) = app.get_webview(&prev_label) {
                let _ = wv.hide();
            }
        }

        mgr.set_active(&tab_id);
        info
    };

    // Create the webview (must be done outside the lock since it may trigger callbacks)
    let window = app
        .get_window("main")
        .ok_or_else(|| "Main window not found".to_string())?;
    webview::create_tab_webview(&window, &tab_id, &nav_url, true)?;

    let _ = app.emit("tab-created", &info);
    let _ = app.emit("active-tab-changed", &tab_id);

    Ok(info)
}

#[tauri::command]
pub async fn close_tab<R: Runtime>(
    app: AppHandle<R>,
    tab_id: String,
) -> Result<(), String> {
    let tab_mgr = app.state::<Arc<Mutex<TabManager>>>();

    let (was_active, new_active_id, need_new_tab) = {
        let mut mgr = tab_mgr.lock().map_err(|e| e.to_string())?;
        let was_active = mgr.active_tab_id.as_deref() == Some(&tab_id);
        mgr.remove_tab(&tab_id);
        let new_active = mgr.active_tab_id.clone();
        let need_new = mgr.tabs.is_empty();
        (was_active, new_active, need_new)
    };

    // Close the webview
    let label = webview_label(&tab_id);
    if let Some(wv) = app.get_webview(&label) {
        let _ = wv.close();
    }

    let _ = app.emit("tab-closed", &tab_id);

    if need_new_tab {
        // Open a fresh tab when the last one is closed
        return create_tab(app, None).await.map(|_| ());
    }

    if was_active {
        if let Some(new_id) = &new_active_id {
            // Show the new active tab's webview
            let new_label = webview_label(new_id);
            if let Some(wv) = app.get_webview(&new_label) {
                let _ = wv.show();
            }
            let _ = app.emit("active-tab-changed", new_id);
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn switch_tab<R: Runtime>(
    app: AppHandle<R>,
    tab_id: String,
) -> Result<(), String> {
    let tab_mgr = app.state::<Arc<Mutex<TabManager>>>();
    let (prev_id, tab_title) = {
        let mut mgr = tab_mgr.lock().map_err(|e| e.to_string())?;
        let prev = mgr.active_tab_id.clone();
        if !mgr.set_active(&tab_id) {
            return Err(format!("Tab not found: {tab_id}"));
        }
        let title = mgr.get_tab(&tab_id).map(|t| t.title.clone()).unwrap_or_default();
        (prev, title)
    };

    // Hide previous, show new
    if let Some(prev) = &prev_id {
        if prev != &tab_id {
            let prev_label = webview_label(prev);
            if let Some(wv) = app.get_webview(&prev_label) {
                let _ = wv.hide();
            }
        }
    }

    let new_label = webview_label(&tab_id);
    if let Some(wv) = app.get_webview(&new_label) {
        let _ = wv.show();
    }

    // Update window title to reflect the new active tab
    if let Some(window) = app.get_window("main") {
        let window_title = if tab_title.is_empty() {
            "VoidBrowser".to_string()
        } else {
            format!("{tab_title} \u{2014} VoidBrowser")
        };
        let _ = window.set_title(&window_title);
    }

    let _ = app.emit("active-tab-changed", &tab_id);
    Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TabsResponse {
    pub tabs: Vec<TabInfo>,
    pub active_tab_id: Option<String>,
}

#[tauri::command]
pub async fn get_tabs<R: Runtime>(
    app: AppHandle<R>,
) -> Result<TabsResponse, String> {
    let tab_mgr = app.state::<Arc<Mutex<TabManager>>>();
    let mgr = tab_mgr.lock().map_err(|e| e.to_string())?;
    Ok(TabsResponse {
        tabs: mgr.all_tab_infos(),
        active_tab_id: mgr.active_tab_id.clone(),
    })
}

#[tauri::command]
pub async fn reorder_tabs<R: Runtime>(
    app: AppHandle<R>,
    tab_ids: Vec<String>,
) -> Result<(), String> {
    let tab_mgr = app.state::<Arc<Mutex<TabManager>>>();
    let mut mgr = tab_mgr.lock().map_err(|e| e.to_string())?;
    mgr.reorder(&tab_ids);
    Ok(())
}

// ── Navigation commands (now tab-aware) ──────────────────────────────

#[tauri::command]
pub async fn navigate_to<R: Runtime>(
    app: AppHandle<R>,
    tab_id: String,
    input: String,
) -> Result<(), String> {
    // Resolve URL using the user's configured search engine.
    let url_string = {
        let db = app.state::<Arc<Mutex<Database>>>();
        let search_template = match db.lock() {
            Ok(db) => {
                let engine = settings::get_setting_or_default(&db, "search_engine");
                settings::search_engine_url(&engine).to_string()
            }
            Err(_) => settings::search_engine_url("duckduckgo").to_string(),
        };
        navigation::resolve_input_with_engine(&input, &search_template)
    };
    let parsed: url::Url = url_string.parse().map_err(|e: url::ParseError| e.to_string())?;

    let label = webview_label(&tab_id);
    let webview = app
        .get_webview(&label)
        .ok_or_else(|| format!("Webview not found for tab: {tab_id}"))?;

    webview.navigate(parsed).map_err(|e| e.to_string())?;

    // Update tab URL in manager
    let tab_mgr = app.state::<Arc<Mutex<TabManager>>>();
    if let Ok(mut mgr) = tab_mgr.lock() {
        if let Some(tab) = mgr.get_tab_mut(&tab_id) {
            tab.url = url_string;
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn go_back<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    let tab_mgr = app.state::<Arc<Mutex<TabManager>>>();
    let tab_id = {
        let mgr = tab_mgr.lock().map_err(|e| e.to_string())?;
        active_tab_id(&mgr)?
    };

    let label = webview_label(&tab_id);
    let webview = app
        .get_webview(&label)
        .ok_or_else(|| "Active tab webview not found".to_string())?;

    webview
        .eval("window.history.back()")
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn go_forward<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    let tab_mgr = app.state::<Arc<Mutex<TabManager>>>();
    let tab_id = {
        let mgr = tab_mgr.lock().map_err(|e| e.to_string())?;
        active_tab_id(&mgr)?
    };

    let label = webview_label(&tab_id);
    let webview = app
        .get_webview(&label)
        .ok_or_else(|| "Active tab webview not found".to_string())?;

    webview
        .eval("window.history.forward()")
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reload_page<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    let tab_mgr = app.state::<Arc<Mutex<TabManager>>>();
    let tab_id = {
        let mgr = tab_mgr.lock().map_err(|e| e.to_string())?;
        active_tab_id(&mgr)?
    };

    let label = webview_label(&tab_id);
    let webview = app
        .get_webview(&label)
        .ok_or_else(|| "Active tab webview not found".to_string())?;

    webview.reload().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_current_url<R: Runtime>(app: AppHandle<R>) -> Result<String, String> {
    let tab_mgr = app.state::<Arc<Mutex<TabManager>>>();
    let tab_id = {
        let mgr = tab_mgr.lock().map_err(|e| e.to_string())?;
        active_tab_id(&mgr)?
    };

    let label = webview_label(&tab_id);
    let webview = app
        .get_webview(&label)
        .ok_or_else(|| "Active tab webview not found".to_string())?;

    let url = webview.url().map_err(|e| e.to_string())?;
    Ok(url.to_string())
}

// ── Privacy commands ─────────────────────────────────────────────────

#[tauri::command]
pub async fn get_blocked_count<R: Runtime>(
    app: AppHandle<R>,
    tab_id: String,
) -> Result<u64, String> {
    let shield = app.state::<Arc<Mutex<ShieldState>>>();
    let state = shield.lock().map_err(|e| e.to_string())?;
    Ok(state.get_count(&tab_id))
}

#[tauri::command]
pub async fn toggle_shield<R: Runtime>(
    app: AppHandle<R>,
    tab_id: String,
) -> Result<bool, String> {
    let shield = app.state::<Arc<Mutex<ShieldState>>>();
    let mut state = shield.lock().map_err(|e| e.to_string())?;
    let enabled = state.toggle(&tab_id);
    Ok(enabled)
}

// ── Phase 5: HTTPS-Only + Per-site shield commands ──────────────────

#[tauri::command]
pub async fn allow_http_and_navigate<R: Runtime>(
    app: AppHandle<R>,
    tab_id: String,
    url: String,
) -> Result<(), String> {
    let domain = https_only::extract_domain(&url)
        .ok_or_else(|| format!("Cannot extract domain from URL: {url}"))?;

    // Add domain to the allowed-HTTP set
    let https_state = app.state::<Arc<Mutex<HttpsOnlyState>>>();
    if let Ok(mut state) = https_state.lock() {
        state.allow_http(&domain);
    }

    // Navigate the tab's webview to the HTTP URL
    let parsed: url::Url = url.parse().map_err(|e: url::ParseError| e.to_string())?;
    let label = webview_label(&tab_id);
    let webview = app
        .get_webview(&label)
        .ok_or_else(|| format!("Webview not found for tab: {tab_id}"))?;
    webview.navigate(parsed).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn toggle_site_shield<R: Runtime>(
    app: AppHandle<R>,
    domain: String,
) -> Result<bool, String> {
    let shield = app.state::<Arc<Mutex<ShieldState>>>();
    let mut state = shield.lock().map_err(|e| e.to_string())?;
    let enabled = state.toggle_site(&domain);

    let _ = app.emit(
        "site-shield-toggled",
        serde_json::json!({
            "domain": domain,
            "enabled": enabled
        }),
    );

    Ok(enabled)
}

#[tauri::command]
pub async fn get_site_shield_status<R: Runtime>(
    app: AppHandle<R>,
    domain: String,
) -> Result<bool, String> {
    let shield = app.state::<Arc<Mutex<ShieldState>>>();
    let state = shield.lock().map_err(|e| e.to_string())?;
    Ok(!state.is_site_disabled(&domain))
}

// ── Bookmark commands ────────────────────────────────────────────────

#[tauri::command]
pub async fn add_bookmark<R: Runtime>(
    app: AppHandle<R>,
    url: String,
    title: String,
    folder: Option<String>,
) -> Result<Bookmark, String> {
    let db = app.state::<Arc<Mutex<Database>>>();
    let db = db.lock().map_err(|e| e.to_string())?;
    bookmarks::add_bookmark(&db, &url, &title, folder.as_deref())
}

#[tauri::command]
pub async fn remove_bookmark<R: Runtime>(
    app: AppHandle<R>,
    id: String,
) -> Result<(), String> {
    let db = app.state::<Arc<Mutex<Database>>>();
    let db = db.lock().map_err(|e| e.to_string())?;
    bookmarks::remove_bookmark(&db, &id)
}

#[tauri::command]
pub async fn get_bookmarks<R: Runtime>(
    app: AppHandle<R>,
    folder: Option<String>,
) -> Result<Vec<Bookmark>, String> {
    let db = app.state::<Arc<Mutex<Database>>>();
    let db = db.lock().map_err(|e| e.to_string())?;
    bookmarks::get_bookmarks(&db, folder.as_deref())
}

#[tauri::command]
pub async fn search_bookmarks<R: Runtime>(
    app: AppHandle<R>,
    query: String,
) -> Result<Vec<Bookmark>, String> {
    let db = app.state::<Arc<Mutex<Database>>>();
    let db = db.lock().map_err(|e| e.to_string())?;
    bookmarks::search_bookmarks(&db, &query)
}

// ── Settings commands ────────────────────────────────────────────────

#[tauri::command]
pub async fn get_setting<R: Runtime>(
    app: AppHandle<R>,
    key: String,
) -> Result<Option<String>, String> {
    let db = app.state::<Arc<Mutex<Database>>>();
    let db = db.lock().map_err(|e| e.to_string())?;
    settings::get_setting(&db, &key)
}

#[tauri::command]
pub async fn set_setting<R: Runtime>(
    app: AppHandle<R>,
    key: String,
    value: String,
) -> Result<(), String> {
    let db = app.state::<Arc<Mutex<Database>>>();
    let db = db.lock().map_err(|e| e.to_string())?;
    settings::set_setting(&db, &key, &value)
}

#[tauri::command]
pub async fn get_all_settings<R: Runtime>(
    app: AppHandle<R>,
) -> Result<HashMap<String, String>, String> {
    let db = app.state::<Arc<Mutex<Database>>>();
    let db = db.lock().map_err(|e| e.to_string())?;
    settings::get_all_settings(&db)
}

// ── History commands ─────────────────────────────────────────────────

#[tauri::command]
pub async fn search_history<R: Runtime>(
    app: AppHandle<R>,
    query: String,
) -> Result<Vec<HistoryEntry>, String> {
    let history = app.state::<Arc<Mutex<SessionHistory>>>();
    let h = history.lock().map_err(|e| e.to_string())?;
    Ok(h.search(&query))
}

#[tauri::command]
pub async fn add_history_entry<R: Runtime>(
    app: AppHandle<R>,
    url: String,
    title: String,
) -> Result<(), String> {
    let history = app.state::<Arc<Mutex<SessionHistory>>>();
    let mut h = history.lock().map_err(|e| e.to_string())?;
    h.add_entry(&url, &title);
    Ok(())
}

// ── Layout commands ─────────────────────────────────────────────────

#[tauri::command]
pub async fn set_sidebar_open<R: Runtime>(
    app: AppHandle<R>,
    open: bool,
) -> Result<(), String> {
    let window = app
        .get_window("main")
        .ok_or("Main window not found")?;
    let size = window.inner_size().map_err(|e| e.to_string())?;
    let scale = window.scale_factor().map_err(|e| e.to_string())?;

    let logical_width = size.width as f64 / scale;
    let logical_height = size.height as f64 / scale;

    let sidebar_width = if open { 300.0 } else { 0.0 };

    let tab_mgr = app.state::<Arc<Mutex<TabManager>>>();
    let tab_ids: Vec<String> = {
        let mgr = tab_mgr.lock().map_err(|e| e.to_string())?;
        mgr.tabs.iter().map(|t| t.id.clone()).collect()
    };

    for tab_id in tab_ids {
        let label = webview_label(&tab_id);
        if let Some(wv) = app.get_webview(&label) {
            let _ = wv.set_position(tauri::LogicalPosition::new(
                sidebar_width,
                webview::TOOLBAR_HEIGHT,
            ));
            let _ = wv.set_size(tauri::LogicalSize::new(
                logical_width - sidebar_width,
                logical_height - webview::TOOLBAR_HEIGHT,
            ));
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn set_settings_open<R: Runtime>(
    app: AppHandle<R>,
    open: bool,
) -> Result<(), String> {
    let tab_mgr = app.state::<Arc<Mutex<TabManager>>>();
    let active_id = {
        let mgr = tab_mgr.lock().map_err(|e| e.to_string())?;
        mgr.active_tab_id.clone()
    };

    if let Some(tab_id) = active_id {
        let label = webview_label(&tab_id);
        if let Some(wv) = app.get_webview(&label) {
            if open {
                let _ = wv.hide();
            } else {
                let _ = wv.show();
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn handle_keyboard_shortcut<R: Runtime>(
    app: AppHandle<R>,
    key: String,
) -> Result<(), String> {
    let _ = app.emit("menu-shortcut", &key);
    Ok(())
}

// ── Privacy stats commands ──────────────────────────────────────────

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyStats {
    pub total_blocked: u64,
    pub total_upgrades: u64,
    pub top_blocked_domains: Vec<(String, u64)>,
}

#[tauri::command]
pub async fn get_privacy_stats<R: Runtime>(
    app: AppHandle<R>,
) -> Result<PrivacyStats, String> {
    let shield = app.state::<Arc<Mutex<ShieldState>>>();
    let shield_state = shield.lock().map_err(|e| e.to_string())?;

    let https_state_arc = app.state::<Arc<Mutex<HttpsOnlyState>>>();
    let https_state = https_state_arc.lock().map_err(|e| e.to_string())?;

    Ok(PrivacyStats {
        total_blocked: shield_state.get_total_blocked(),
        total_upgrades: https_state.get_total_upgrades(),
        top_blocked_domains: shield_state.get_top_blocked_domains(10),
    })
}

// ── Find-in-page commands ───────────────────────────────────────────

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FindResult {
    pub total_matches: u32,
    pub current_index: u32,
}

#[tauri::command]
pub async fn find_in_page<R: Runtime>(
    app: AppHandle<R>,
    query: String,
) -> Result<FindResult, String> {
    let tab_mgr = app.state::<Arc<Mutex<TabManager>>>();
    let tab_id = {
        let mgr = tab_mgr.lock().map_err(|e| e.to_string())?;
        active_tab_id(&mgr)?
    };

    let label = webview_label(&tab_id);
    let webview = app
        .get_webview(&label)
        .ok_or_else(|| "Active tab webview not found".to_string())?;

    let escaped_query = query
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "\\r");

    let script = format!(
        r#"(function() {{
            // Clear previous highlights
            if (window.__VOID_FIND__) {{
                window.__VOID_FIND__.clear();
            }}

            var query = '{}';
            if (!query) return JSON.stringify({{ totalMatches: 0, currentIndex: 0 }});

            var marks = [];
            var currentIdx = 0;

            function clear() {{
                marks.forEach(function(mark) {{
                    var parent = mark.parentNode;
                    if (parent) {{
                        parent.replaceChild(document.createTextNode(mark.textContent), mark);
                        parent.normalize();
                    }}
                }});
                marks = [];
                currentIdx = 0;
            }}

            function highlightAll() {{
                var walker = document.createTreeWalker(
                    document.body,
                    NodeFilter.SHOW_TEXT,
                    null
                );
                var textNodes = [];
                while (walker.nextNode()) {{
                    var node = walker.currentNode;
                    if (node.parentElement &&
                        !['SCRIPT','STYLE','NOSCRIPT','TEXTAREA','INPUT'].includes(node.parentElement.tagName)) {{
                        textNodes.push(node);
                    }}
                }}

                var lowerQuery = query.toLowerCase();
                for (var i = 0; i < textNodes.length; i++) {{
                    var node = textNodes[i];
                    var text = node.textContent;
                    var lowerText = text.toLowerCase();
                    var idx = lowerText.indexOf(lowerQuery);
                    if (idx === -1) continue;

                    var parts = [];
                    var lastIdx = 0;
                    while (idx !== -1) {{
                        if (idx > lastIdx) {{
                            parts.push(document.createTextNode(text.substring(lastIdx, idx)));
                        }}
                        var mark = document.createElement('mark');
                        mark.textContent = text.substring(idx, idx + query.length);
                        mark.style.cssText = 'background:#fbbf24;color:#171717;padding:0 1px;border-radius:2px;';
                        mark.className = '__void_find_mark';
                        marks.push(mark);
                        parts.push(mark);
                        lastIdx = idx + query.length;
                        idx = lowerText.indexOf(lowerQuery, lastIdx);
                    }}
                    if (lastIdx < text.length) {{
                        parts.push(document.createTextNode(text.substring(lastIdx)));
                    }}

                    var parent = node.parentNode;
                    for (var j = 0; j < parts.length; j++) {{
                        parent.insertBefore(parts[j], node);
                    }}
                    parent.removeChild(node);
                }}
            }}

            function goTo(idx) {{
                if (marks.length === 0) return;
                if (marks[currentIdx]) {{
                    marks[currentIdx].style.background = '#fbbf24';
                }}
                currentIdx = ((idx % marks.length) + marks.length) % marks.length;
                if (marks[currentIdx]) {{
                    marks[currentIdx].style.background = '#f97316';
                    marks[currentIdx].scrollIntoView({{ block: 'center', behavior: 'smooth' }});
                }}
            }}

            highlightAll();
            if (marks.length > 0) goTo(0);

            window.__VOID_FIND__ = {{
                marks: marks,
                currentIdx: currentIdx,
                clear: clear,
                next: function() {{
                    this.currentIdx++;
                    goTo(this.currentIdx);
                    return JSON.stringify({{ totalMatches: this.marks.length, currentIndex: this.currentIdx }});
                }},
                prev: function() {{
                    this.currentIdx--;
                    goTo(this.currentIdx);
                    return JSON.stringify({{ totalMatches: this.marks.length, currentIndex: this.currentIdx }});
                }},
                getResult: function() {{
                    return JSON.stringify({{ totalMatches: this.marks.length, currentIndex: this.currentIdx }});
                }}
            }};

            return JSON.stringify({{ totalMatches: marks.length, currentIndex: currentIdx }});
        }})()"#,
        escaped_query
    );

    let result = webview.eval(&script).map_err(|e| e.to_string())?;
    // eval doesn't return values, so we return a default and let the frontend
    // use a follow-up call or event. For simplicity, return a placeholder.
    let _ = result;
    Ok(FindResult {
        total_matches: 0,
        current_index: 0,
    })
}

#[tauri::command]
pub async fn find_next<R: Runtime>(
    app: AppHandle<R>,
) -> Result<(), String> {
    let tab_mgr = app.state::<Arc<Mutex<TabManager>>>();
    let tab_id = {
        let mgr = tab_mgr.lock().map_err(|e| e.to_string())?;
        active_tab_id(&mgr)?
    };

    let label = webview_label(&tab_id);
    let webview = app
        .get_webview(&label)
        .ok_or_else(|| "Active tab webview not found".to_string())?;

    webview
        .eval("if (window.__VOID_FIND__) { window.__VOID_FIND__.next(); }")
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn find_previous<R: Runtime>(
    app: AppHandle<R>,
) -> Result<(), String> {
    let tab_mgr = app.state::<Arc<Mutex<TabManager>>>();
    let tab_id = {
        let mgr = tab_mgr.lock().map_err(|e| e.to_string())?;
        active_tab_id(&mgr)?
    };

    let label = webview_label(&tab_id);
    let webview = app
        .get_webview(&label)
        .ok_or_else(|| "Active tab webview not found".to_string())?;

    webview
        .eval("if (window.__VOID_FIND__) { window.__VOID_FIND__.prev(); }")
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_find_in_page<R: Runtime>(
    app: AppHandle<R>,
) -> Result<(), String> {
    let tab_mgr = app.state::<Arc<Mutex<TabManager>>>();
    let tab_id = {
        let mgr = tab_mgr.lock().map_err(|e| e.to_string())?;
        active_tab_id(&mgr)?
    };

    let label = webview_label(&tab_id);
    let webview = app
        .get_webview(&label)
        .ok_or_else(|| "Active tab webview not found".to_string())?;

    webview
        .eval("if (window.__VOID_FIND__) { window.__VOID_FIND__.clear(); window.__VOID_FIND__ = null; }")
        .map_err(|e| e.to_string())
}

// ── Zoom commands ───────────────────────────────────────────────────

#[tauri::command]
pub async fn zoom_in<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    let tab_mgr = app.state::<Arc<Mutex<TabManager>>>();
    let tab_id = {
        let mgr = tab_mgr.lock().map_err(|e| e.to_string())?;
        active_tab_id(&mgr)?
    };

    let label = webview_label(&tab_id);
    let webview = app
        .get_webview(&label)
        .ok_or_else(|| "Active tab webview not found".to_string())?;

    webview
        .eval(r#"(function(){
            var z = parseFloat(document.body.style.zoom || '1');
            z = Math.min(z + 0.1, 3.0);
            document.body.style.zoom = z;
        })()"#)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn zoom_out<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    let tab_mgr = app.state::<Arc<Mutex<TabManager>>>();
    let tab_id = {
        let mgr = tab_mgr.lock().map_err(|e| e.to_string())?;
        active_tab_id(&mgr)?
    };

    let label = webview_label(&tab_id);
    let webview = app
        .get_webview(&label)
        .ok_or_else(|| "Active tab webview not found".to_string())?;

    webview
        .eval(r#"(function(){
            var z = parseFloat(document.body.style.zoom || '1');
            z = Math.max(z - 0.1, 0.3);
            document.body.style.zoom = z;
        })()"#)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn zoom_reset<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    let tab_mgr = app.state::<Arc<Mutex<TabManager>>>();
    let tab_id = {
        let mgr = tab_mgr.lock().map_err(|e| e.to_string())?;
        active_tab_id(&mgr)?
    };

    let label = webview_label(&tab_id);
    let webview = app
        .get_webview(&label)
        .ok_or_else(|| "Active tab webview not found".to_string())?;

    webview
        .eval("document.body.style.zoom = '1';")
        .map_err(|e| e.to_string())
}

