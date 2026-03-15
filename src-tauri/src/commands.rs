use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::browser::navigation;
use crate::browser::tabs::{Tab, TabInfo, TabManager};
use crate::browser::webview;
use crate::privacy::ad_blocker::ShieldState;

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
    let prev_id = {
        let mut mgr = tab_mgr.lock().map_err(|e| e.to_string())?;
        let prev = mgr.active_tab_id.clone();
        if !mgr.set_active(&tab_id) {
            return Err(format!("Tab not found: {tab_id}"));
        }
        prev
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
    let url_string = navigation::resolve_input(&input);
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

