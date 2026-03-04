# grok-overlay (Tauri 2 Rewrite)

A lightweight cross-platform desktop overlay for `grok.com`, rewritten with **Tauri 2** for **macOS + Windows**.

## Features

- Dedicated Grok window (`https://grok.com`)
- Frameless overlay style with custom top bar and one close button
- macOS runs in accessory mode (no persistent Dock icon; use tray + shortcut)
- Global show/hide shortcut
  - macOS default: `Alt+Space`
  - Windows default: `Ctrl+Alt+G`
- Tray icon with quick actions:
  - Show / Hide Grok
  - Go to Grok home
  - Open settings
  - Toggle launch at login
  - Quit
- Settings window:
  - Record and update global shortcut (no manual string typing needed)
  - Toggle always-on-top
  - Toggle launch at login
- Local settings persistence (`settings.json` in AppData app config directory)

## Tech Stack

- Tauri 2 (Rust backend + native WebView)
- `tauri-plugin-global-shortcut`
- `tauri-plugin-autostart`
- `tauri-plugin-opener`
- Plain HTML/CSS/JS settings UI

## Requirements

- Node.js 20+
- Rust stable (with Cargo)
- Platform prerequisites for Tauri desktop development:
  - macOS: Xcode Command Line Tools
  - Windows: MSVC Build Tools + WebView2 runtime

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

## CI Release

GitHub Actions workflow: `.github/workflows/release.yml`

- Trigger: push a tag matching `v*` (example: `v1.0.1`)
- Matrix targets:
  - Windows x64 (`x86_64-pc-windows-msvc`)
  - Windows ARM64 (`aarch64-pc-windows-msvc`)
  - macOS x64 (`x86_64-apple-darwin`)
  - macOS ARM64 (`aarch64-apple-darwin`)
- Runners:
  - Windows: `windows-latest`
  - macOS Intel: `macos-15-intel`
  - macOS Apple Silicon: `macos-15`
- Output:
  - Windows: NSIS installer (`.exe`)
  - macOS: DMG (`.dmg`)
- Publish: all artifacts are attached automatically to the GitHub Release for that tag.
- macOS CI build uses ad-hoc signing (`APPLE_SIGNING_IDENTITY="-"`) to avoid unsigned-app corruption warnings.

Create a release build by pushing a tag:

```bash
git tag v1.0.1
git push origin v1.0.1
```

If macOS still shows "App is damaged":

```bash
xattr -dr com.apple.quarantine "/Applications/Grok Overlay.app"
```

## Project Layout

```text
.
+-- web/                     # Local settings page assets
|   +-- settings.html
|   +-- settings.js
|   +-- styles.css
+-- src-tauri/
|   +-- src/
|   |   +-- lib.rs          # App logic (tray/shortcut/settings/autostart)
|   |   +-- main.rs
|   +-- capabilities/
|   +-- icons/
|   +-- Cargo.toml
|   +-- tauri.conf.json
+-- package.json
```
