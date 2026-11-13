# PulseStream

Tauri 2 + Rust, plain HTML/CSS/JS UI (no Node build step for the app itself).

## Getting the .exe (no local setup)
1. Create a GitHub repo and push this folder to it.
2. Open **Actions → Build PulseStream (Windows) → Run workflow**.
3. When it finishes, download the **PulseStream-windows** artifact:
   `pulsestream.exe` (standalone) and an NSIS installer.

## Building locally on Windows
Install Rust (MSVC) + VS Build Tools, then: `npm i -g @tauri-apps/cli@2` and `tauri build`
(icons are already included). Output: `src-tauri/target/release/pulsestream.exe`.
Needs the WebView2 runtime (preinstalled on Windows 10/11).

## Features
Search (Deezer previews), queue, shuffle, repeat off/all/one, seek, volume, media keys, tray
(close = hide), mini player, dark/light, recently played, liked songs, playlists (click a chip
to pick a target, then "+", or drag a track onto a chip), lyrics (LRCLIB), sleep timer,
start-with-Windows, track-change notifications while hidden.

## Adding a provider
Implement `Provider` in `src-tauri/src/providers/`, register it in `main.rs`.
