# Tauri2 Any WebApp Wrapper

A lightweight [Tauri v2](https://v2.tauri.app/) desktop app that wraps any website into a native window. Just point it at a URL via a JSON config file and you have an instant desktop app — no code changes required.

## Features

- **Any URL** — Load any website in a native desktop window
- **Dynamic title** — Window title automatically syncs with the loaded page title (Windows)
- **Custom title** — Optionally set a fixed window title via config
- **Custom icon** — Set your own window icon (ICO or PNG)
- **Rename-to-configure** — Rename the executable and it auto-detects its config file (`MyApp.exe` → `MyApp.json`)
- **Cross-platform** — Builds for Windows x64, Linux x64, and macOS ARM64

## Quick Start

1. Download the binary for your platform from the [latest release](https://github.com/AtmanActive/Tauri2_Any_WebApp_Wrapper/releases/latest)
2. Create a JSON config file next to the binary, matching its name (e.g. `app.json` for `app.exe`):
   ```json
   {
     "url": "https://example.com",
     "title": "",
     "icon": ""
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

| Field | Required | Description |
|-------|----------|-------------|
| `url` | Yes | The website URL to load |
| `title` | No | Fixed window title. If empty, the title syncs with the page title (Windows only) |
| `icon` | No | Path to a custom window icon (`.ico` or `.png`). Absolute path, or relative to the executable |

### Example

```json
{
  "url": "https://music.youtube.com",
  "title": "YouTube Music",
  "icon": "music.png"
}
```

## Platform Notes

| Platform | Runtime Requirement |
|----------|-------------------|
| **Windows** | WebView2 (pre-installed on Windows 10/11) |
| **Linux** | WebKit2GTK 4.1 (`libwebkit2gtk-4.1`) |
| **macOS** | None (uses WKWebView) |

- **Dynamic title sync** uses the WebView2 COM API and is only available on Windows. On macOS and Linux, the window title stays at the default unless a static `title` is set in the config.

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
        ├── lib.rs               # App setup, navigation, title sync
        └── config.rs            # Config struct + loader
```

## License

[MIT](LICENSE) © AtmanActive

---

Vibecoded by AtmanActive using Claude Code (Opus 4.6), 2026.
