mod browser;
mod commands;
mod privacy;

use std::sync::{Arc, Mutex};
use std::time::Instant;

use tauri::Manager;

use browser::tabs::{Tab, TabManager};
use privacy::ad_blocker::{AdBlocker, ShieldState};
use privacy::fingerprint::FingerprintShield;
use privacy::https_only::HttpsOnlyState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(Arc::new(Mutex::new(TabManager::new())))
        .manage(Arc::new(Mutex::new(ShieldState::new())))
        .manage(Arc::new(Mutex::new(HttpsOnlyState::new())))
        .invoke_handler(tauri::generate_handler![
            commands::navigate_to,
            commands::go_back,
            commands::go_forward,
            commands::reload_page,
            commands::get_current_url,
            commands::create_tab,
            commands::close_tab,
            commands::switch_tab,
            commands::get_tabs,
            commands::reorder_tabs,
            commands::get_blocked_count,
            commands::toggle_shield,
            commands::allow_http_and_navigate,
            commands::toggle_site_shield,
            commands::get_site_shield_status,
        ])
        .setup(|app| {
            // Initialize the ad blocker engine
            let start = Instant::now();
            let ad_blocker = AdBlocker::new();
            let elapsed = start.elapsed();
            eprintln!("Ad blocker engine initialized in {elapsed:?}");
            app.manage(Arc::new(Mutex::new(ad_blocker)));

            // Initialize fingerprint resistance (immutable after creation, no Mutex needed)
            let fp_shield = FingerprintShield::new();
            eprintln!("Fingerprint shield initialized");
            app.manage(fp_shield);

            let window = app
                .get_window("main")
                .expect("main window not found in setup");

            // Create the first tab
            let tab = Tab::new("void://newtab".to_string());
            let tab_id = tab.id.clone();

            let tab_mgr = app.state::<Arc<Mutex<TabManager>>>();
            {
                let mut mgr = tab_mgr
                    .lock()
                    .map_err(|e| format!("Lock error: {e}"))?;
                mgr.add_tab(tab);
                mgr.set_active(&tab_id);
            }

            if let Err(e) =
                browser::webview::create_tab_webview(&window, &tab_id, "void://newtab", true)
            {
                eprintln!("Failed to create initial tab webview: {e}");
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                // Delete ephemeral webview data on exit for defense-in-depth.
                // With incognito(true) enabled on all webviews, cookies and storage
                // are already in-memory only, but we clean up any residual files.
                if let Ok(data_dir) = app_handle.path().app_data_dir() {
                    let webview_data = data_dir.join("EBWebView");
                    let _ = std::fs::remove_dir_all(&webview_data);
                    eprintln!(
                        "Cleaned up ephemeral webview data at {}",
                        webview_data.display()
                    );
                }
            }
        });
}
