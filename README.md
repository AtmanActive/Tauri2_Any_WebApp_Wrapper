# Tauri2 Any WebApp Wrapper

A lightweight [Tauri v2](https://v2.tauri.app/) desktop app that wraps any website into a native window. Just point it at a URL via a JSON config file and you have an instant desktop app — no code changes required.

## Features

- **Any URL** — Load any website in a native desktop window
- **Dynamic title** — Window title automatically syncs with the loaded page title (Windows)
- **Custom title** — Optionally set a fixed window title via config
- **Custom icon** — Set your own window icon (ICO or PNG)
- **Dark mode control** — Request dark/light theme from sites, or force-dark all sites (Windows)
- **Remember window position** — Window size, position, and maximized state are saved and restored across sessions
- **Start minimized** — Optionally launch the app minimized to the taskbar
- **Single-instance control** — Prevent multiple instances, or let the latest instance take over (Windows)
- **Rename-to-configure** — Rename the executable and it auto-detects its config file (`MyApp.exe` → `MyApp.json`)
- **Cross-platform** — Builds for Windows x64, Linux x64, and macOS ARM64

## Quick Start

1. Download the binary for your platform from the [latest release](https://github.com/AtmanActive/Tauri2_Any_WebApp_Wrapper/releases/latest)
2. Create a JSON config file next to the binary, matching its name (e.g. `app.json` for `app.exe`):
   ```json
   {
     "url": "https://example.com"
   }
   ```
3. Run the binary

That's it. The app opens a native window and loads the configured URL.

## Configuration

The config file is a simple JSON file placed next to the executable. The filename must match the executable name (without extension):

| Executable | Config file |
|-----------|-------------|
| `app.exe` | `app.json` |
| `MyWebApp.exe` | `MyWebApp.json` |
| `Spotify.exe` | `Spotify.json` |

### Config fields

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `url` | Yes | — | The website URL to load |
| `title` | No | `""` | Fixed window title. If empty, the title syncs with the page title (Windows only) |
| `icon` | No | `""` | Path to a custom window icon (`.ico` or `.png`). Absolute path, or relative to the executable |
| `prefer_dark_mode` | No | `"default"` | Color scheme preference: `"default"` (let OS decide), `"dark"` (request dark theme), `"light"` (request light theme). Only affects sites that support `prefers-color-scheme` CSS. Windows only |
| `force_dark_mode` | No | `"off"` | Force-dark rendering: `"on"` or `"off"`. When `"on"`, forces all sites into dark mode even if they don't natively support it — same as Chrome's force-dark flag. Windows only |
| `start_minimized` | No | `"off"` | Start minimized to taskbar: `"on"` or `"off"` |
| `allow_only_one_instance` | No | `"off"` | Single-instance mode: `"off"` (allow multiple), `"on"` or `"first"` (exit if already running), `"last"` (kill existing and take over). Windows only |

### Example — minimal

```json
{
  "url": "https://music.youtube.com"
}
```

### Example — full

```json
{
  "url": "https://music.youtube.com",
  "title": "YouTube Music",
  "icon": "music.png",
  "prefer_dark_mode": "dark",
  "force_dark_mode": "off",
  "start_minimized": "off",
  "allow_only_one_instance": "off"
}
```

### Dark mode options explained

**`prefer_dark_mode`** tells the website your color scheme preference via the CSS `prefers-color-scheme` media query. Sites that support dark mode (like GitHub, YouTube, etc.) will switch their theme accordingly. Set to `"dark"` or `"light"` to override the OS setting, or `"default"` to let the OS decide.

**`force_dark_mode`** is the nuclear option — it enables Chromium's built-in force-dark rendering engine (equivalent to `chrome://flags/#enable-force-dark-web-contents`). This will force-render **all** sites in dark mode, even ones that don't have any dark theme support. Results vary per site — some look great, others may look odd. Set to `"on"` to enable.

The two options can be combined: `prefer_dark_mode` handles CSS-aware sites gracefully, while `force_dark_mode` catches everything else.

### Window state persistence

The app automatically remembers your window position, size, and maximized state between sessions. This works out of the box — no configuration needed.

- The state is saved to `<exe_name>.window.json` beside the executable (e.g. `app.window.json`)
- Updated every time you move, resize, or maximize/restore the window
- On next launch, the window opens exactly where you left it
- To reset to defaults, simply delete the `.window.json` file
- When multiple instances are allowed, each new instance opens with a +32px offset so windows don't stack exactly on top of each other

### Single-instance mode

**`allow_only_one_instance`** controls how the app handles multiple instances:

| Value | Behavior |
|-------|----------|
| `"off"` (default) | Multiple instances allowed. New windows cascade with a +32px offset |
| `"on"` or `"first"` | If an instance is already running, it is brought to the foreground (restored from minimized if needed) and the new one exits |
| `"last"` | If an instance is already running, it is terminated and the new one takes over |

This is useful for apps where only one window should exist at a time, like a dedicated music player or chat client.

## Platform Notes

| Platform | Runtime Requirement |
|----------|-------------------|
| **Windows** | WebView2 (pre-installed on Windows 10/11) |
| **Linux** | WebKit2GTK 4.1 (`libwebkit2gtk-4.1`) |
| **macOS** | None (uses WKWebView) |

- **Dynamic title sync**, **prefer_dark_mode**, **force_dark_mode**, and **allow_only_one_instance** use Windows APIs and are only available on Windows. On macOS and Linux, the window title stays at the default unless a static `title` is set in the config.

## Building from Source

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) (v22+)
- Platform-specific dependencies:
  - **Windows**: Visual Studio Build Tools (C++ workload), WebView2
  - **Linux**: `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libsoup-3.0-dev libjavascriptcoregtk-4.1-dev`
  - **macOS**: Xcode Command Line Tools

### Build

```bash
npm install
cd src-tauri
cargo build --release
```

The binary will be at `src-tauri/target/release/app` (or `app.exe` on Windows).

## Project Structure

```
├── app.json                     # Runtime config (rename to match your exe)
├── src/
│   └── index.html               # Brief loading splash
└── src-tauri/
    ├── Cargo.toml               # Rust dependencies
    ├── tauri.conf.json           # Tauri build config
    └── src/
        ├── main.rs              # Entry point
        ├── lib.rs               # App setup, navigation, title sync, dark mode, window state, single-instance
        └── config.rs            # Config struct + loader
```

## Replacing the Executable Icon

By default, the `.exe` file ships with the Tauri icon. To give your wrapped app its own identity in the taskbar and file explorer, you can replace the embedded icon using a resource editor.

### Recommended tools

| Tool | Type | Description |
|------|------|-------------|
| [Resource Hacker](https://www.angusj.com/resourcehacker/) | Free, portable | The standard tool for editing Windows PE resources. Open the `.exe`, go to **Icon Group**, right-click → **Replace Icon**, pick your `.ico` file, and save |
| [Greenfish Icon Editor Pro](http://greenfishsoftware.org/gfie.php) | Free, open-source | Full icon editor — import a PNG, export as multi-size `.ico` |
| [IcoFX](https://icofx.ro/) | Shareware | Feature-rich icon editor with PNG-to-ICO conversion |
| [ImageMagick](https://imagemagick.org/) | Free, open-source, CLI | Convert from the command line: `magick convert icon.png icon.ico` |

### Steps

1. **Create an `.ico` file** from your PNG using one of the tools above (ideally include 16x16, 32x32, 48x48, and 256x256 sizes)
2. **Open the `.exe`** in Resource Hacker
3. Navigate to **Icon Group** → right-click → **Replace Icon** → select your `.ico` file
4. **Save** the modified `.exe`

> **Note**: This replaces the icon shown in File Explorer and the taskbar. The window icon at runtime can also be set via the `icon` field in your JSON config — both approaches can be used together.

## License

[MIT](LICENSE) © AtmanActive

---

Vibecoded by AtmanActive using Claude Code (Opus 4.6), 2026.
