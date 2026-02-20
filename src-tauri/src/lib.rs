mod config;

use config::{AppConfig, WindowState};
use tauri::Manager;

const APP_VERSION: &str = "3.0.7";
const APP_REPO_URL: &str = "https://github.com/AtmanActive/Tauri2_Any_WebApp_Wrapper";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load config early — before Tauri creates the webview — so we can set
    // environment variables that affect WebView2 initialization.
    let config = match AppConfig::load() {
        Ok(c) => c,
        Err(e) => {
            show_config_error(&e.to_string());
            std::process::exit(1);
        }
    };

    // Single-instance enforcement (before any window is created)
    if let Some(mode) = config.instance_mode() {
        enforce_single_instance(mode);
    }

    // For multi-instance mode: count running siblings to compute cascade offset
    // so each new instance opens at +32px from the previous one
    let cascade_offset = if config.instance_mode().is_none() {
        count_sibling_instances() as i32 * 32
    } else {
        0
    };

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

            // Restore saved window position/size (with cascade offset for multi-instance)
            restore_window_state(&window, cascade_offset);

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

            // Add "About" item to the system menu (window icon menu)
            setup_system_menu(&window);

            // Register WebView2 handlers (title sync + color scheme preference)
            let title_window = window.clone();
            let has_static_title = !config.title.is_empty();
            let color_scheme = config.prefer_dark_mode.clone();
            setup_webview_handlers(&window, title_window, has_static_title, &color_scheme);

            // Register window event handler to persist position/size
            let save_window = window.clone();
            window.on_window_event(move |event| {
                use tauri::WindowEvent;
                match event {
                    WindowEvent::Moved(_) | WindowEvent::Resized(_) => {
                        save_window_state(&save_window);
                    }
                    _ => {}
                }
            });

            // Navigate to the configured URL
            let url: tauri::Url = config.url.parse().expect("Invalid URL in config.json");
            let _ = window.navigate(url);

            // Start minimized (if configured)
            if config.start_minimized.eq_ignore_ascii_case("on") {
                let _ = window.minimize();
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Restore window position, size, and maximized state from the saved state file.
/// `cascade_offset` adds N pixels to both X and Y to cascade multiple instances
/// so they don't stack exactly on top of each other (0 = no offset).
fn restore_window_state(window: &tauri::WebviewWindow, cascade_offset: i32) {
    if let Some(state) = WindowState::load() {
        // Validate that the saved size is reasonable (at least 200x200)
        if state.width >= 200 && state.height >= 200 {
            let _ = window.set_size(tauri::PhysicalSize::new(state.width, state.height));
        }
        // Restore position with cascade offset
        let _ = window.set_position(tauri::PhysicalPosition::new(
            state.x + cascade_offset,
            state.y + cascade_offset,
        ));
        // Restore maximized state
        if state.maximized {
            let _ = window.maximize();
        }
    } else if cascade_offset > 0 {
        // No saved state (first run), but we have siblings — offset from default position
        if let Ok(pos) = window.outer_position() {
            let _ = window.set_position(tauri::PhysicalPosition::new(
                pos.x + cascade_offset,
                pos.y + cascade_offset,
            ));
        }
    }
}

/// Save current window position, size, and maximized state to disk
fn save_window_state(window: &tauri::WebviewWindow) {
    // When minimized, Windows moves the window to (-32000, -32000).
    // Don't save that — we want to keep the last normal position.
    if window.is_minimized().unwrap_or(false) {
        return;
    }

    let maximized = window.is_maximized().unwrap_or(false);

    // When maximized, don't overwrite the saved normal position/size —
    // we want to restore the non-maximized geometry next time.
    // Only save the maximized flag.
    if maximized {
        if let Some(mut state) = WindowState::load() {
            state.maximized = true;
            state.save();
        } else {
            // No previous state — save current dimensions with maximized flag
            let pos = window.outer_position().unwrap_or_default();
            let size = window.outer_size().unwrap_or_default();
            let state = WindowState {
                x: pos.x,
                y: pos.y,
                width: size.width,
                height: size.height,
                maximized: true,
            };
            state.save();
        }
        return;
    }

    let pos = window.outer_position().unwrap_or_default();
    let size = window.outer_size().unwrap_or_default();
    let state = WindowState {
        x: pos.x,
        y: pos.y,
        width: size.width,
        height: size.height,
        maximized: false,
    };
    state.save();
}

/// Count how many other processes with the same executable name are running.
/// Used to compute the cascade offset for multi-instance window stacking.
#[cfg(target_os = "windows")]
fn count_sibling_instances() -> u32 {
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Threading::GetCurrentProcessId;

    let our_pid = unsafe { GetCurrentProcessId() };
    let our_exe = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase().to_string()))
        .unwrap_or_default();

    if our_exe.is_empty() {
        return 0;
    }

    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    let snapshot = match snapshot {
        Ok(h) => h,
        Err(_) => return 0,
    };

    let mut entry = PROCESSENTRY32W::default();
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
    let mut count: u32 = 0;

    unsafe {
        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let name = String::from_utf16_lossy(
                    &entry.szExeFile[..entry
                        .szExeFile
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(entry.szExeFile.len())],
                );

                if name.to_lowercase() == our_exe && entry.th32ProcessID != our_pid {
                    count += 1;
                }

                entry = PROCESSENTRY32W::default();
                entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = windows::Win32::Foundation::CloseHandle(snapshot);
    }

    count
}

#[cfg(not(target_os = "windows"))]
fn count_sibling_instances() -> u32 {
    0
}

/// Bring the main window of the given process IDs to the foreground.
/// Enumerates all top-level windows, finds one owned by one of the target PIDs,
/// and uses ShowWindow + SetForegroundWindow to restore and activate it.
#[cfg(target_os = "windows")]
fn activate_process_windows(pids: &[u32]) {
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowLongW, GetWindowTextLengthW, GetWindowThreadProcessId,
        SetForegroundWindow, ShowWindow, GWL_STYLE, SW_RESTORE, SW_SHOW,
        WS_VISIBLE, WS_MINIMIZE,
    };

    struct CallbackData {
        pids: Vec<u32>,
        found: HWND,
    }

    unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> windows::core::BOOL {
        let data = &mut *(lparam.0 as *mut CallbackData);
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid as *mut u32));

        if data.pids.contains(&pid) {
            // Check if this is a real app window (has a title bar text)
            // This filters out invisible helper windows that processes often create
            if GetWindowTextLengthW(hwnd) > 0 {
                data.found = hwnd;
                return windows::core::BOOL(0); // Stop enumerating
            }
        }
        windows::core::BOOL(1) // Continue
    }

    let mut data = CallbackData {
        pids: pids.to_vec(),
        found: HWND::default(),
    };

    unsafe {
        let _ = EnumWindows(Some(enum_callback), LPARAM(&mut data as *mut _ as isize));

        if data.found != HWND::default() {
            // Check window style to determine if minimized
            let style = GetWindowLongW(data.found, GWL_STYLE) as u32;
            if style & WS_MINIMIZE.0 != 0 {
                // Window is minimized — restore it
                let _ = ShowWindow(data.found, SW_RESTORE);
            } else if style & WS_VISIBLE.0 == 0 {
                // Window exists but isn't visible — show it
                let _ = ShowWindow(data.found, SW_SHOW);
            }
            let _ = SetForegroundWindow(data.found);
        }
    }
}

/// Enforce single-instance policy by checking for other processes with the same exe name.
/// Mode "first": exit if another instance is already running.
/// Mode "last": kill any existing instances, then continue.
#[cfg(target_os = "windows")]
fn enforce_single_instance(mode: &str) {
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Threading::{
        GetCurrentProcessId, OpenProcess, TerminateProcess, PROCESS_TERMINATE,
    };

    // Get our own exe name and PID
    let our_pid = unsafe { GetCurrentProcessId() };
    let our_exe = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase().to_string()))
        .unwrap_or_default();

    if our_exe.is_empty() {
        return;
    }

    // Snapshot all processes
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    let snapshot = match snapshot {
        Ok(h) => h,
        Err(_) => return,
    };

    let mut entry = PROCESSENTRY32W::default();
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

    let mut found_pids: Vec<u32> = Vec::new();

    unsafe {
        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let name = String::from_utf16_lossy(
                    &entry.szExeFile[..entry
                        .szExeFile
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(entry.szExeFile.len())],
                );

                if name.to_lowercase() == our_exe && entry.th32ProcessID != our_pid {
                    found_pids.push(entry.th32ProcessID);
                }

                entry = PROCESSENTRY32W::default();
                entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = windows::Win32::Foundation::CloseHandle(snapshot);
    }

    if found_pids.is_empty() {
        return; // No other instance running, proceed normally
    }

    match mode {
        "first" => {
            // Another instance is already running — bring it to focus, then exit
            activate_process_windows(&found_pids);
            std::process::exit(0);
        }
        "last" => {
            // Kill all other instances, then continue
            for pid in found_pids {
                unsafe {
                    if let Ok(handle) = OpenProcess(PROCESS_TERMINATE, false, pid) {
                        let _ = TerminateProcess(handle, 1);
                        let _ = windows::Win32::Foundation::CloseHandle(handle);
                    }
                }
            }
            // Brief sleep to let the OS clean up the terminated processes
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        _ => {}
    }
}

#[cfg(not(target_os = "windows"))]
fn enforce_single_instance(_mode: &str) {
    // Process enumeration is Windows-only; no-op on other platforms
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

/// Show a native error dialog when the config file cannot be loaded.
#[cfg(target_os = "windows")]
fn show_config_error(_error: &str) {
    use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_OK, MB_ICONERROR};

    let config_name = AppConfig::config_filename();
    let message = format!(
        "Could not load configuration file.\n\n\
         Expected file: {}\n\
         Place it next to the executable.\n\n\
         Minimum required content:\n\n\
         {{\n  \"url\": \"https://example.com\"\n}}",
        config_name
    );

    let caption: Vec<u16> = "Configuration Error\0".encode_utf16().collect();
    let text: Vec<u16> = message.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let _ = MessageBoxW(
            None,
            windows::core::PCWSTR(text.as_ptr()),
            windows::core::PCWSTR(caption.as_ptr()),
            MB_OK | MB_ICONERROR,
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn show_config_error(_error: &str) {
    let config_name = AppConfig::config_filename();
    eprintln!(
        "Could not load configuration file.\n\n\
         Expected file: {}\n\
         Place it next to the executable.\n\n\
         Minimum required content:\n\n\
         {{\n  \"url\": \"https://example.com\"\n}}",
        config_name
    );
}

/// Custom command ID for the "About" item in the system menu (window icon menu).
/// Must be below 0xF000 and above standard SC_* values to avoid conflicts.
#[cfg(target_os = "windows")]
const SC_ABOUT: usize = 0x0010;

/// Add a custom "Tauri WebApp on Demand vX.Y.Z" item to the window's system menu
/// and subclass the window to handle clicks on it.
#[cfg(target_os = "windows")]
fn setup_system_menu(window: &tauri::WebviewWindow) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        AppendMenuW, GetSystemMenu, MF_SEPARATOR, MF_STRING,
    };
    use windows::Win32::UI::Shell::SetWindowSubclass;

    let Ok(hwnd) = window.hwnd() else { return };
    let hwnd = HWND(hwnd.0 as *mut _);

    unsafe {
        let hmenu = GetSystemMenu(hwnd, false);
        if hmenu.is_invalid() {
            return;
        }

        // Add separator + about item
        let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, None);
        let label: Vec<u16> = format!("Tauri WebApp on Demand v{}", APP_VERSION)
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let _ = AppendMenuW(
            hmenu,
            MF_STRING,
            SC_ABOUT,
            windows::core::PCWSTR(label.as_ptr()),
        );

        // Subclass to intercept WM_SYSCOMMAND for our custom menu item
        let _ = SetWindowSubclass(hwnd, Some(sysmenu_subclass_proc), 1, 0);
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn sysmenu_subclass_proc(
    hwnd: windows::Win32::Foundation::HWND,
    umsg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
    _uidsubclass: usize,
    _dwrefdata: usize,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::Shell::DefSubclassProc;
    use windows::Win32::UI::WindowsAndMessaging::WM_SYSCOMMAND;

    if umsg == WM_SYSCOMMAND && wparam.0 == SC_ABOUT {
        // Open the project URL in the default browser
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", APP_REPO_URL])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .spawn();
        return windows::Win32::Foundation::LRESULT(0);
    }

    DefSubclassProc(hwnd, umsg, wparam, lparam)
}

#[cfg(not(target_os = "windows"))]
fn setup_system_menu(_window: &tauri::WebviewWindow) {
    // System menu customization is Windows-only
}
