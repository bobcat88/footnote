# Footnote One launcher

The desktop window over the `footnote` engine (Tauri 2 + React). It holds no data of its own: each screen runs read-only engine
commands (`status`, `projects`, `activity`, `models list`, `code`, `code-map`) as a child process, through one Tauri command
with an allowlist (`src-tauri/src/main.rs`). Design: [DESIGN.md](../../DESIGN.md) section 11; composition reference
`.design-studio/launcher/chosen/index.html`.

## Build and run (Windows; Linux and macOS untested)

Needs Node 20+, Rust 1.92+, WebView2 (shipped with Windows 11), and the `footnote` engine: `FOOTNOTE_BIN`, beside the app,
`%LOCALAPPDATA%\Programs\footnote\footnote.exe`, or on PATH.

```sh
npm ci --ignore-scripts        # 25 packages, none needs an install script (checked 16 Nov 2026); .npmrc enforces it
npx tauri dev                  # window with live reload
npx tauri build --no-bundle    # src-tauri/target/release/footnote-launcher.exe (no installer yet: P1 M4)
```

`src-tauri` is its own Cargo workspace, so Tauri stays out of the engine's build and CI.

## Screenshots without the app

`npx vite build && npx vite preview`, then open `/#/overview?mock=1&state=ok|empty|missing|loading`: invented sample data
(`src/fixtures.js`) for the QLoop UI gates. Outside the app and without `mock=1`, the page shows "engine not found".
