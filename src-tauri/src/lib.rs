mod browser;
mod commands;

use std::sync::{Arc, Mutex};

use tauri::Manager;

use browser::tabs::{Tab, TabManager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(Arc::new(Mutex::new(TabManager::new())))
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
        ])
        .setup(|app| {
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
