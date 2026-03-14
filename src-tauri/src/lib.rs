mod browser;
mod commands;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::navigate_to,
            commands::go_back,
            commands::go_forward,
            commands::reload_page,
            commands::get_current_url,
        ])
        .setup(|app| {
            let window = app
                .get_window("main")
                .expect("main window not found in setup");

            // Create the content webview below the toolbar
            if let Err(e) = browser::webview::create_content_webview(&window) {
                eprintln!("Failed to create content webview: {e}");
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
