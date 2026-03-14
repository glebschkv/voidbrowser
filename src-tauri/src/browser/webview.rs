use tauri::{
    webview::WebviewBuilder, window::Window, Emitter, LogicalPosition, LogicalSize, Manager,
    Runtime, Webview, WebviewUrl,
};

/// Height of the toolbar area in logical pixels.
pub const TOOLBAR_HEIGHT: f64 = 50.0;

/// Label used for the content webview.
pub const CONTENT_WEBVIEW_LABEL: &str = "content";

/// Create the content webview as a child of the given window,
/// positioned below the toolbar.
pub fn create_content_webview<R: Runtime>(
    window: &Window<R>,
) -> Result<Webview<R>, String> {
    let size = window.inner_size().map_err(|e| e.to_string())?;
    let scale = window.scale_factor().map_err(|e| e.to_string())?;

    let logical_width = size.width as f64 / scale;
    let logical_height = size.height as f64 / scale;

    let app_handle = window.app_handle().clone();

    let webview_builder = WebviewBuilder::new(
        CONTENT_WEBVIEW_LABEL,
        WebviewUrl::External(
            "https://duckduckgo.com"
                .parse()
                .map_err(|e: url::ParseError| e.to_string())?,
        ),
    )
    .auto_resize()
    .on_navigation(move |url| {
        let _ = app_handle.emit("url-changed", url.to_string());
        true
    });

    window
        .add_child(
            webview_builder,
            LogicalPosition::new(0.0, TOOLBAR_HEIGHT),
            LogicalSize::new(logical_width, logical_height - TOOLBAR_HEIGHT),
        )
        .map_err(|e| e.to_string())
}
