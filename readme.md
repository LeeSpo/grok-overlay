# Grok Overlay

A lightweight, always-on-top desktop overlay for [grok.com](https://grok.com), built with **Tauri 2**. Frameless, minimal, and cross-platform — macOS and Windows.

## Features

- **Immersive frameless window** — no system title bar; a custom 36px dark title bar with a single circular close button blends seamlessly with Grok's dark UI
- **Multi-Webview architecture** — the title bar is a local webview (fully controllable), the content area loads grok.com in a separate webview
- **Global shortcut** to show / hide the overlay instantly
  - macOS: `Alt + Space`
  - Windows: `Ctrl + Alt + G`
- **System tray** with quick actions: show/hide, go to grok.com home, settings, toggle launch at login, quit
- **Settings window** — record custom shortcuts, toggle always-on-top, toggle launch at login
- **macOS accessory mode** — no persistent Dock icon; interact via tray and shortcut only
- **Local settings persistence** — saved to `settings.json` in the platform's app config directory

## Architecture

```
┌──────────────────────────────────────┐
│  (x)   titlebar.html    [36px, #000] │  Webview 1 — local page, drag + close
├──────────────────────────────────────┤
│                                      │
│            grok.com                  │  Webview 2 — external page
│                                      │
└──────────────────────────────────────┘
```

The main window is created with `decorations(false)` (no OS chrome) and hosts two webviews inside a single native `Window` via Tauri 2's multi-webview API (`unstable` feature). The title bar webview handles drag-to-move and close via custom Tauri commands; the content webview loads grok.com without any script injection.

## Tech Stack

- **Tauri 2** (Rust backend + native WebView) with `unstable` multi-webview support
- `tauri-plugin-global-shortcut` — system-wide show/hide hotkey
- `tauri-plugin-autostart` — launch at login
- `tauri-plugin-opener` — open external links
- Plain HTML / CSS / JS for settings and title bar UI (no framework)

## Requirements

- Node.js 20+
- Rust stable (with Cargo)
- Platform prerequisites:
  - **macOS** — Xcode Command Line Tools
  - **Windows** — MSVC Build Tools + WebView2 Runtime

## Development

```bash
npm install
npm run tauri:dev
```

## Build

```bash
npm install
npm run tauri:build
```

Build outputs are created by Tauri under `src-tauri/target`.

## CI / Release

Automated via GitHub Actions (`.github/workflows/release.yml`).

**Trigger:** push a tag matching `v*`

```bash
git tag v1.0.1
git push origin v1.0.1
```

**Targets:**

| Platform | Runner | Bundle |
|----------|--------|--------|
| Windows x64 | `windows-latest` | NSIS `.exe` |
| Windows ARM64 | `windows-latest` | NSIS `.exe` |
| macOS x64 | `macos-15-intel` | `.dmg` |
| macOS ARM64 | `macos-15` | `.dmg` |

All artifacts are attached to the GitHub Release automatically. macOS builds use ad-hoc signing (`APPLE_SIGNING_IDENTITY="-"`).

If macOS shows "App is damaged" after download:

```bash
xattr -dr com.apple.quarantine "/Applications/Grok Overlay.app"
```

## Project Layout

```
.
├── web/
│   ├── titlebar.html        # Custom title bar (close button + drag region)
│   ├── settings.html        # Settings page
│   ├── settings.js
│   ├── styles.css
│   └── index.html
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs           # Core logic: multi-webview setup, tray, shortcuts, settings
│   │   └── main.rs
│   ├── capabilities/
│   │   └── default.json     # Tauri permission grants
│   ├── icons/
│   ├── Cargo.toml
│   └── tauri.conf.json
├── .github/workflows/
│   └── release.yml          # CI build + release pipeline
├── package.json
└── LICENSE                   # MIT
```

## License

[MIT](LICENSE)
