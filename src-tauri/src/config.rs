use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Deserialize)]
pub struct AppConfig {
    pub url: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub prefer_dark_mode: String,
    #[serde(default)]
    pub force_dark_mode: String,
    #[serde(default)]
    pub start_minimized: String,
}

/// Persisted window geometry — saved beside the config as `<name>.window.json`
#[derive(Serialize, Deserialize, Default)]
pub struct WindowState {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
}

impl AppConfig {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = Self::find_config_path()?;
        let contents = std::fs::read_to_string(&config_path)?;
        let config: AppConfig = serde_json::from_str(&contents)?;
        Ok(config)
    }

    fn config_filename() -> String {
        // Derive config filename from the executable name: MyApp.exe -> MyApp.json
        std::env::current_exe()
            .ok()
            .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
            .map(|name| format!("{}.json", name))
            .unwrap_or_else(|| "config.json".to_string())
    }

    fn find_config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let config_name = Self::config_filename();

        // In debug mode, check project root first (via CARGO_MANIFEST_DIR)
        #[cfg(debug_assertions)]
        {
            if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
                let project_root = PathBuf::from(manifest_dir)
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_default();
                let path = project_root.join(&config_name);
                if path.exists() {
                    return Ok(path);
                }
            }
        }

        // Check beside the executable
        let exe_dir = std::env::current_exe()?
            .parent()
            .ok_or("Cannot determine exe directory")?
            .to_path_buf();
        let path = exe_dir.join(&config_name);
        if path.exists() {
            return Ok(path);
        }

        Err(format!("{} not found", config_name).into())
    }

    /// Path for the window state file: `<exe_name>.window.json` beside the config
    pub fn window_state_path() -> Option<PathBuf> {
        let exe_name = std::env::current_exe()
            .ok()
            .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))?;
        let filename = format!("{}.window.json", exe_name);

        // In debug mode, check project root first
        #[cfg(debug_assertions)]
        {
            if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
                if let Some(project_root) = PathBuf::from(manifest_dir).parent() {
                    return Some(project_root.join(&filename));
                }
            }
        }

        // Beside the executable
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join(&filename)))
    }

    pub fn resolve_icon_path(&self) -> Option<PathBuf> {
        if self.icon.is_empty() {
            return None;
        }

        let icon_path = PathBuf::from(&self.icon);
        if icon_path.is_absolute() && icon_path.exists() {
            return Some(icon_path);
        }

        // Resolve relative to exe directory
        if let Ok(exe) = std::env::current_exe() {
            if let Some(exe_dir) = exe.parent() {
                let resolved = exe_dir.join(&icon_path);
                if resolved.exists() {
                    return Some(resolved);
                }
            }
        }

        // In debug mode, also resolve relative to project root
        #[cfg(debug_assertions)]
        {
            if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
                if let Some(project_root) = PathBuf::from(manifest_dir).parent() {
                    let resolved = project_root.join(&icon_path);
                    if resolved.exists() {
                        return Some(resolved);
                    }
                }
            }
        }

        None
    }
}

impl WindowState {
    pub fn load() -> Option<Self> {
        let path = AppConfig::window_state_path()?;
        let contents = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&contents).ok()
    }

    pub fn save(&self) {
        if let Some(path) = AppConfig::window_state_path() {
            if let Ok(json) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(path, json);
            }
        }
    }
}
