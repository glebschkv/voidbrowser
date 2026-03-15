mod browser;
mod commands;
mod privacy;

use std::sync::{Arc, Mutex};
use std::time::Instant;

use tauri::Manager;

use browser::tabs::{Tab, TabManager};
use privacy::ad_blocker::{AdBlocker, ShieldState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(Arc::new(Mutex::new(TabManager::new())))
        .manage(Arc::new(Mutex::new(ShieldState::new())))
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
        ])
        .setup(|app| {
            // Initialize the ad blocker engine
            let start = Instant::now();
            let ad_blocker = AdBlocker::new();
            let elapsed = start.elapsed();
            eprintln!("Ad blocker engine initialized in {elapsed:?}");
            app.manage(Arc::new(Mutex::new(ad_blocker)));

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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
