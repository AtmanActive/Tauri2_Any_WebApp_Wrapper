mod config;

use config::AppConfig;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load config early — before Tauri creates the webview — so we can set
    // environment variables that affect WebView2 initialization.
    let config = AppConfig::load().expect("Failed to load config.json");

    // Force dark mode: set the Chromium flag before WebView2 is created.
    // This is the equivalent of Chrome's chrome://flags/#enable-force-dark-web-contents
    // and will force-render all sites in dark mode even if they don't support it natively.
    if config.force_dark_mode.eq_ignore_ascii_case("on") {
        std::env::set_var(
            "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
            "--enable-features=WebContentsForceDark",
        );
    }

    tauri::Builder::default()
        .setup(move |app| {
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

            // Register WebView2 handlers (title sync + color scheme preference)
            let title_window = window.clone();
            let has_static_title = !config.title.is_empty();
            let color_scheme = config.prefer_dark_mode.clone();
            setup_webview_handlers(&window, title_window, has_static_title, &color_scheme);

            // Navigate to the configured URL
            let url: tauri::Url = config.url.parse().expect("Invalid URL in config.json");
            let _ = window.navigate(url);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(target_os = "windows")]
fn setup_webview_handlers(
    webview_window: &tauri::WebviewWindow,
    title_window: tauri::WebviewWindow,
    has_static_title: bool,
    color_scheme: &str,
) {
    let needs_color_scheme = matches!(color_scheme.to_lowercase().as_str(), "dark" | "light");

    // Nothing to do if no dynamic title and no color scheme override
    if has_static_title && !needs_color_scheme {
        return;
    }

    let color_scheme = color_scheme.to_lowercase();
    webview_window
        .with_webview(move |webview| unsafe {
            use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2;
            use windows::core::Interface;

            let controller = webview.controller();
            let core: ICoreWebView2 = controller
                .CoreWebView2()
                .expect("Failed to get ICoreWebView2");

            // Set preferred color scheme via ICoreWebView2Profile (requires v13+)
            // "dark" = force dark preference, "light" = force light preference,
            // anything else (including "" or "default") = let the OS decide
            if needs_color_scheme {
                use webview2_com::Microsoft::Web::WebView2::Win32::{
                    ICoreWebView2_13, COREWEBVIEW2_PREFERRED_COLOR_SCHEME_DARK,
                    COREWEBVIEW2_PREFERRED_COLOR_SCHEME_LIGHT,
                };
                let scheme = if color_scheme == "dark" {
                    COREWEBVIEW2_PREFERRED_COLOR_SCHEME_DARK
                } else {
                    COREWEBVIEW2_PREFERRED_COLOR_SCHEME_LIGHT
                };
                if let Ok(core13) = core.cast::<ICoreWebView2_13>() {
                    if let Ok(profile) = core13.Profile() {
                        let _ = profile.SetPreferredColorScheme(scheme);
                    }
                }
            }

            // Register dynamic title sync
            if !has_static_title {
                use webview2_com::DocumentTitleChangedEventHandler;

                let win = title_window.clone();
                let handler = DocumentTitleChangedEventHandler::create(Box::new(
                    move |webview, _args| {
                        if let Some(wv) = webview {
                            let mut title = windows::core::PWSTR::null();
                            wv.DocumentTitle(&mut title)?;
                            if !title.is_null() {
                                let title_str = title.to_string().unwrap_or_default();
                                let _ = win.set_title(&title_str);
                            }
                        }
                        Ok(())
                    },
                ));

                let mut token: i64 = 0;
                let _ = core.add_DocumentTitleChanged(&handler, &mut token);
            }
        })
        .expect("Failed to access webview");
}

#[cfg(not(target_os = "windows"))]
fn setup_webview_handlers(
    _webview_window: &tauri::WebviewWindow,
    _title_window: tauri::WebviewWindow,
    _has_static_title: bool,
    _color_scheme: &str,
) {
    // WebView2 APIs are Windows-only; color scheme and title sync are no-ops on other platforms
}
