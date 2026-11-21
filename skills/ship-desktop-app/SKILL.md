---
name: ship-desktop-app
description: Use when building, testing or releasing a desktop app, especially Tauri 2 (web front end, Rust backend): commands and IPC, capabilities and CSP, startup and process-tree memory, testing the real WebView, installers (NSIS, Inno Setup, MSI, Velopack), code signing, SmartScreen and updaters.
license: MIT
---

# Ship a desktop app

Name the real stack first (Tauri ≠ Electron ≠ native). Test the release build and the installed artifact, never the dev server.

- **Ship:** primary flows in release build / error + recovery states usable / checks pass / startup + memory measured
- **Best:** + clean install, upgrade, uninstall / signature + update verified / long session without growth / UI passed `review-ui-quality`

## Tauri 2
- Commands = the API: validate input / typed errors / no panics / heavy work `async` or `spawn_blocking`
- Big or streaming payloads → channels or events, not one giant JSON reply; measure real payloads first
- Capabilities per window, only what it uses / scope fs + shell to paths and programs / strict CSP, never loosened for a test
- Startup: launch → first paint → first usable action, timed separately / defer init the first screen doesn't need
- Memory = whole process tree (WebView2/WebKit children) after a fixed settle time
- Updater only for online products / update key ≠ code-signing cert / neither in the repo

## Test the real app
- Browser automation on the dev URL sees only the web layer
- Real WebView (DOM, JS, IPC, native screenshot): dev-only bridge plugin (`tauri-plugin-connector`) behind a cargo feature, never in release
- Traps: window must load the bundled frontend (`tauri://localhost`, not `http://…`) or `window.__TAURI__` is missing / on Windows focus the window before snapshot
- Windows rendering (DPI, contrast themes, capture): `review-ui-quality` → `references/windows-native.md`

## Installers + signing (Windows) → [references/windows-installer.md](references/windows-installer.md)
- NSIS (Tauri default) / Inno Setup (themed wizard) / MSI (managed company deployment only) / Velopack (one-click + delta updates)
- No UPX or exe packers (antivirus false positives)
- Sign + timestamp every exe and installer / same publisher name every release / EV no longer skips SmartScreen
- Installer size: state bootstrapper vs offline WebView, they aren't comparable

## Report
Changes / checks run / startup + memory with build ID + machine / artifact paths / untested (platforms, upgrade, signing) stays pending. Speed → `make-code-faster`; pipeline → `fix-ci-and-releases`.
