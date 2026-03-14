use tauri::{AppHandle, Manager, Runtime};

use crate::browser::navigation;
use crate::browser::webview::CONTENT_WEBVIEW_LABEL;

#[tauri::command]
pub async fn navigate_to<R: Runtime>(
    app: AppHandle<R>,
    input: String,
) -> Result<(), String> {
    let url_string = navigation::resolve_input(&input);
    let parsed: url::Url = url_string.parse().map_err(|e: url::ParseError| e.to_string())?;

    let webview = app
        .get_webview(CONTENT_WEBVIEW_LABEL)
        .ok_or_else(|| "Content webview not found".to_string())?;

    webview.navigate(parsed).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn go_back<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    let webview = app
        .get_webview(CONTENT_WEBVIEW_LABEL)
        .ok_or_else(|| "Content webview not found".to_string())?;

    webview
        .eval("window.history.back()")
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn go_forward<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    let webview = app
        .get_webview(CONTENT_WEBVIEW_LABEL)
        .ok_or_else(|| "Content webview not found".to_string())?;

    webview
        .eval("window.history.forward()")
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reload_page<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    let webview = app
        .get_webview(CONTENT_WEBVIEW_LABEL)
        .ok_or_else(|| "Content webview not found".to_string())?;

    webview.reload().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_current_url<R: Runtime>(app: AppHandle<R>) -> Result<String, String> {
    let webview = app
        .get_webview(CONTENT_WEBVIEW_LABEL)
        .ok_or_else(|| "Content webview not found".to_string())?;

    let url = webview.url().map_err(|e| e.to_string())?;
    Ok(url.to_string())
}
