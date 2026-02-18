mod config;

use config::AppConfig;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let config = AppConfig::load().expect("Failed to load config.json");

            let window = app
                .get_webview_window("main")
                .expect("Failed to get main window");

            // Set initial title from config (if provided)
            if !config.title.is_empty() {
                window
                    .set_title(&config.title)
                    .expect("Failed to set window title");
            }

            // Set custom icon from config (if provided)
            if let Some(icon_path) = config.resolve_icon_path() {
                if let Ok(icon_data) = std::fs::read(&icon_path) {
                    if let Ok(img) = tauri::image::Image::from_bytes(&icon_data) {
                        let _ = window.set_icon(img);
                    }
                }
            }

            // Register dynamic title sync via WebView2 COM API
            let title_window = window.clone();
            let has_static_title = !config.title.is_empty();
            setup_title_changed_handler(&window, title_window, has_static_title);

            // Navigate to the configured URL
            let url: tauri::Url = config.url.parse().expect("Invalid URL in config.json");
            let _ = window.navigate(url);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(target_os = "windows")]
fn setup_title_changed_handler(
    webview_window: &tauri::WebviewWindow,
    title_window: tauri::WebviewWindow,
    has_static_title: bool,
) {
    // If user specified a static title, skip dynamic title updates
    if has_static_title {
        return;
    }

    webview_window
        .with_webview(move |webview| unsafe {
            use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2;
            use webview2_com::DocumentTitleChangedEventHandler;

            let controller = webview.controller();
            let core: ICoreWebView2 = controller
                .CoreWebView2()
                .expect("Failed to get ICoreWebView2");

            let win = title_window.clone();
            let handler =
                DocumentTitleChangedEventHandler::create(Box::new(move |webview, _args| {
                    if let Some(wv) = webview {
                        let mut title = windows::core::PWSTR::null();
                        wv.DocumentTitle(&mut title)?;
                        if !title.is_null() {
                            let title_str = title.to_string().unwrap_or_default();
                            let _ = win.set_title(&title_str);
                        }
                    }
                    Ok(())
                }));

            let mut token: i64 = 0;
            let _ = core.add_DocumentTitleChanged(&handler, &mut token);
        })
        .expect("Failed to access webview");
}

#[cfg(not(target_os = "windows"))]
fn setup_title_changed_handler(
    _webview_window: &tauri::WebviewWindow,
    _title_window: tauri::WebviewWindow,
    _has_static_title: bool,
) {
    // Title sync via WebView2 is Windows-only
}
